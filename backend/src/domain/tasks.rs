//! 通用后台长任务：DB 记录任务生命周期，内存保存运行中任务的进度与取消标志。

use crate::error::{AppError, AppResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    /// 服务重启时仍在运行的任务
    Interrupted,
}

impl TaskStatus {
    fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Interrupted => "interrupted",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "running" => TaskStatus::Running,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::Interrupted,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct TaskProgress {
    pub current: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TaskDto {
    pub id: String,
    pub tool: String,
    pub library_id: Option<String>,
    pub library_name: Option<String>,
    #[schema(value_type = Object)]
    pub params: Value,
    pub status: TaskStatus,
    pub cancel_requested: bool,
    pub progress: TaskProgress,
    /// 仅在查询单个任务时返回
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub result: Option<Value>,
    pub error: Option<String>,
    /// 发起人用户名
    pub created_by: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    /// 已运行时长（结束的任务为总耗时）
    pub elapsed_ms: i64,
}

/// 后台任务执行时可用的句柄：上报进度、检查是否被取消
#[derive(Clone)]
pub struct TaskContext {
    cancel: Arc<AtomicBool>,
    progress: Arc<Mutex<TaskProgress>>,
}

impl TaskContext {
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn set_total(&self, total: u64) {
        let mut p = self.progress.lock().unwrap();
        p.current = 0;
        p.total = Some(total);
    }

    pub fn set_progress(&self, current: u64, message: impl Into<String>) {
        let mut p = self.progress.lock().unwrap();
        p.current = current;
        p.message = Some(message.into());
    }

    pub fn set_message(&self, message: impl Into<String>) {
        self.progress.lock().unwrap().message = Some(message.into());
    }

    fn snapshot(&self) -> TaskProgress {
        self.progress.lock().unwrap().clone()
    }
}

pub struct NewTask {
    pub tool: String,
    /// 互斥键：同一键同时只能有一个运行中的任务
    pub lock_key: String,
    pub library_id: Option<Uuid>,
    pub params: Value,
    pub created_by: Uuid,
}

#[derive(Clone, Default)]
pub struct TaskManager {
    running: Arc<Mutex<HashMap<Uuid, TaskContext>>>,
}

impl TaskManager {
    /// 写入任务记录并在后台执行 `job`；同一 lock_key 已有运行中任务时返回 409。
    pub async fn spawn<F, Fut>(&self, db: &SqlitePool, task: NewTask, job: F) -> AppResult<Uuid>
    where
        F: FnOnce(TaskContext) -> Fut + Send + 'static,
        Fut: Future<Output = AppResult<Value>> + Send + 'static,
    {
        let id = Uuid::new_v4();
        let inserted = sqlx::query(
            r#"
            INSERT INTO maintenance_tasks (id, tool, library_id, lock_key, params, status, created_by, started_at)
            VALUES (?, ?, ?, ?, ?, 'running', ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(&task.tool)
        .bind(task.library_id.map(|l| l.to_string()))
        .bind(&task.lock_key)
        .bind(task.params.to_string())
        .bind(task.created_by.to_string())
        .bind(Utc::now().to_rfc3339())
        .execute(db)
        .await;
        match inserted {
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                return Err(AppError::Conflict(
                    "该对象上已有维护任务在运行，请等待其结束或先取消".into(),
                ));
            }
            other => {
                other?;
            }
        }

        let ctx = TaskContext {
            cancel: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Mutex::new(TaskProgress::default())),
        };
        self.running.lock().unwrap().insert(id, ctx.clone());

        let manager = self.clone();
        let db = db.clone();
        tokio::spawn(async move {
            let outcome = tokio::spawn(job(ctx.clone())).await;
            let cancelled = ctx.is_cancelled();
            let (status, result, error) = match outcome {
                Ok(Ok(v)) if cancelled => (TaskStatus::Cancelled, Some(v), None),
                Ok(Ok(v)) => (TaskStatus::Completed, Some(v), None),
                Ok(Err(e)) if cancelled => (TaskStatus::Cancelled, None, Some(e.to_string())),
                Ok(Err(e)) => (TaskStatus::Failed, None, Some(e.to_string())),
                Err(e) => (
                    TaskStatus::Failed,
                    None,
                    Some(format!("task panicked: {e}")),
                ),
            };
            if let Some(err) = &error {
                tracing::warn!(task_id = %id, error = %err, "maintenance task ended with error");
            }
            let progress = ctx.snapshot();
            let saved = sqlx::query(
                r#"
                UPDATE maintenance_tasks
                SET status = ?, result = ?, error = ?, progress_current = ?, progress_total = ?,
                    progress_message = ?, finished_at = ?
                WHERE id = ?
                "#,
            )
            .bind(status.as_str())
            .bind(result.map(|v| v.to_string()))
            .bind(error)
            .bind(progress.current as i64)
            .bind(progress.total.map(|t| t as i64))
            .bind(progress.message)
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(&db)
            .await;
            if let Err(e) = saved {
                tracing::error!(task_id = %id, error = %e, "failed to persist task result");
            }
            manager.running.lock().unwrap().remove(&id);
        });

        Ok(id)
    }

    /// 请求取消运行中的任务（协作式，任务在下一个检查点停止）。
    pub async fn cancel(&self, db: &SqlitePool, id: &Uuid) -> AppResult<()> {
        let ctx = self.running.lock().unwrap().get(id).cloned();
        let Some(ctx) = ctx else {
            // DB 显示 running 但进程内已无此任务：直接标记中断
            sqlx::query(
                "UPDATE maintenance_tasks SET status = 'interrupted', finished_at = ? WHERE id = ? AND status = 'running'",
            )
            .bind(Utc::now().to_rfc3339())
            .bind(id.to_string())
            .execute(db)
            .await?;
            return Ok(());
        };
        ctx.cancel.store(true, Ordering::Relaxed);
        sqlx::query("UPDATE maintenance_tasks SET cancel_requested = 1 WHERE id = ?")
            .bind(id.to_string())
            .execute(db)
            .await?;
        Ok(())
    }

    fn live_progress(&self, id: &str) -> Option<TaskProgress> {
        let id = Uuid::parse_str(id).ok()?;
        self.running.lock().unwrap().get(&id).map(|c| c.snapshot())
    }
}

/// 启动时调用：上次进程退出时仍在运行的任务标记为 interrupted
pub async fn mark_interrupted(db: &SqlitePool) -> AppResult<()> {
    sqlx::query(
        "UPDATE maintenance_tasks SET status = 'interrupted', finished_at = ? WHERE status = 'running'",
    )
    .bind(Utc::now().to_rfc3339())
    .execute(db)
    .await?;
    Ok(())
}

type TaskRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    i64,
    i64,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
);

macro_rules! select_task {
    ($tail:literal) => {
        concat!(
            "SELECT t.id, t.tool, t.library_id, l.name, t.params, t.status, t.cancel_requested, ",
            "t.progress_current, t.progress_total, t.progress_message, t.result, t.error, ",
            "u.username, t.started_at, t.finished_at ",
            "FROM maintenance_tasks t ",
            "LEFT JOIN libraries l ON l.id = t.library_id ",
            "LEFT JOIN users u ON u.id = t.created_by ",
            $tail
        )
    };
}

fn row_to_dto(manager: &TaskManager, r: TaskRow, with_result: bool) -> TaskDto {
    let status = TaskStatus::parse(&r.5);
    let progress = (status == TaskStatus::Running)
        .then(|| manager.live_progress(&r.0))
        .flatten()
        .unwrap_or(TaskProgress {
            current: r.7.max(0) as u64,
            total: r.8.map(|t| t.max(0) as u64),
            message: r.9,
        });
    let parse_time = |s: &str| DateTime::parse_from_rfc3339(s).ok().map(|t| t.to_utc());
    let end =
        r.14.as_deref()
            .and_then(parse_time)
            .unwrap_or_else(Utc::now);
    let elapsed_ms = parse_time(&r.13)
        .map(|start| (end - start).num_milliseconds().max(0))
        .unwrap_or(0);
    TaskDto {
        id: r.0,
        tool: r.1,
        library_id: r.2,
        library_name: r.3,
        params: serde_json::from_str(&r.4).unwrap_or(Value::Null),
        status,
        cancel_requested: r.6 != 0,
        progress,
        result: if with_result {
            r.10.and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        },
        error: r.11,
        created_by: r.12,
        started_at: r.13,
        finished_at: r.14,
        elapsed_ms,
    }
}

pub async fn get_task(db: &SqlitePool, manager: &TaskManager, id: &Uuid) -> AppResult<TaskDto> {
    let row: Option<TaskRow> = sqlx::query_as(select_task!("WHERE t.id = ?"))
        .bind(id.to_string())
        .fetch_optional(db)
        .await?;
    let row = row.ok_or_else(|| AppError::NotFound("Task not found".into()))?;
    Ok(row_to_dto(manager, row, true))
}

/// 运行中的任务在前，其后为最近结束的任务（不含 result）
pub async fn list_tasks(
    db: &SqlitePool,
    manager: &TaskManager,
    active_only: bool,
    limit: i64,
) -> AppResult<Vec<TaskDto>> {
    let rows: Vec<TaskRow> = sqlx::query_as(select_task!(
        "WHERE (? = 0 OR t.status = 'running') ORDER BY (t.status = 'running') DESC, t.started_at DESC LIMIT ?"
    ))
    .bind(active_only)
    .bind(limit)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| row_to_dto(manager, r, false))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::time::Duration;

    async fn pool() -> SqlitePool {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&db).await.unwrap();
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, role) VALUES (?, 'u', 'x', 'user')",
        )
        .bind(Uuid::nil().to_string())
        .execute(&db)
        .await
        .unwrap();
        db
    }

    fn new_task(lock: &str) -> NewTask {
        NewTask {
            tool: "test".into(),
            lock_key: lock.into(),
            library_id: None,
            params: serde_json::json!({}),
            created_by: Uuid::nil(),
        }
    }

    async fn wait_finished(db: &SqlitePool, m: &TaskManager, id: &Uuid) -> TaskDto {
        for _ in 0..200 {
            let t = get_task(db, m, id).await.unwrap();
            if t.status != TaskStatus::Running {
                return t;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("task did not finish");
    }

    async fn until_cancelled(ctx: TaskContext) -> AppResult<Value> {
        ctx.set_total(10);
        let mut n = 0;
        while !ctx.is_cancelled() {
            n += 1;
            ctx.set_progress(n.min(10), "working");
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        Ok(serde_json::json!({ "steps": n }))
    }

    #[tokio::test]
    async fn same_lock_is_rejected_until_cancelled() {
        let db = pool().await;
        let m = TaskManager::default();
        let id = m
            .spawn(&db, new_task("lib:1"), until_cancelled)
            .await
            .unwrap();

        let err = m
            .spawn(&db, new_task("lib:1"), until_cancelled)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
        let other = m
            .spawn(&db, new_task("lib:2"), until_cancelled)
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(20)).await;
        let running = list_tasks(&db, &m, true, 10).await.unwrap();
        assert_eq!(running.len(), 2);
        assert!(running.iter().all(|t| t.progress.total == Some(10)));

        m.cancel(&db, &id).await.unwrap();
        let done = wait_finished(&db, &m, &id).await;
        assert_eq!(done.status, TaskStatus::Cancelled);
        assert!(done.cancel_requested && done.finished_at.is_some());
        assert!(done.result.is_some(), "partial result is kept");

        m.spawn(&db, new_task("lib:1"), |_| async { Ok(Value::Null) })
            .await
            .unwrap();
        m.cancel(&db, &other).await.unwrap();
        wait_finished(&db, &m, &other).await;
    }

    #[tokio::test]
    async fn failures_and_restarts_are_recorded() {
        let db = pool().await;
        let m = TaskManager::default();
        let id = m
            .spawn(&db, new_task("x"), |_| async {
                Err(AppError::Internal("boom".into()))
            })
            .await
            .unwrap();
        let t = wait_finished(&db, &m, &id).await;
        assert_eq!(t.status, TaskStatus::Failed);
        assert_eq!(t.error.as_deref(), Some("boom"));

        let stale = m.spawn(&db, new_task("y"), until_cancelled).await.unwrap();
        // 模拟进程重启：新的 manager 不认识旧任务
        mark_interrupted(&db).await.unwrap();
        let fresh = TaskManager::default();
        let t = get_task(&db, &fresh, &stale).await.unwrap();
        assert_eq!(t.status, TaskStatus::Interrupted);
        m.cancel(&db, &stale).await.unwrap();
    }
}
