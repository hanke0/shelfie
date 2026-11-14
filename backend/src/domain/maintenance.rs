//! 系统维护工具：工具清单、参数校验、权限与后台任务分发。

use crate::domain::auth::AuthUser;
use crate::domain::library::{self, require_delete, require_edit, resolve_permission};
use crate::domain::sync;
use crate::domain::tasks::{self, NewTask, TaskDto};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use utoipa::ToSchema;
use uuid::Uuid;

const LIST_LIMIT: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceTool {
    LibraryScan,
    LibraryImportUntracked,
    LibraryResyncMetadata,
    LibraryDeleteUntracked,
    LibraryPurgeMissing,
    LibraryCleanDangling,
    LibraryRefreshHashes,
    LibraryFillMissingHashes,
}

impl MaintenanceTool {
    const ALL: [MaintenanceTool; 8] = [
        MaintenanceTool::LibraryScan,
        MaintenanceTool::LibraryImportUntracked,
        MaintenanceTool::LibraryResyncMetadata,
        MaintenanceTool::LibraryDeleteUntracked,
        MaintenanceTool::LibraryPurgeMissing,
        MaintenanceTool::LibraryCleanDangling,
        MaintenanceTool::LibraryRefreshHashes,
        MaintenanceTool::LibraryFillMissingHashes,
    ];

    fn as_str(self) -> &'static str {
        match self {
            MaintenanceTool::LibraryScan => "library_scan",
            MaintenanceTool::LibraryImportUntracked => "library_import_untracked",
            MaintenanceTool::LibraryResyncMetadata => "library_resync_metadata",
            MaintenanceTool::LibraryDeleteUntracked => "library_delete_untracked",
            MaintenanceTool::LibraryPurgeMissing => "library_purge_missing",
            MaintenanceTool::LibraryCleanDangling => "library_clean_dangling",
            MaintenanceTool::LibraryRefreshHashes => "library_refresh_hashes",
            MaintenanceTool::LibraryFillMissingHashes => "library_fill_missing_hashes",
        }
    }

    fn permission(self) -> ToolPermission {
        match self {
            MaintenanceTool::LibraryScan
            | MaintenanceTool::LibraryImportUntracked
            | MaintenanceTool::LibraryResyncMetadata
            | MaintenanceTool::LibraryRefreshHashes
            | MaintenanceTool::LibraryFillMissingHashes => ToolPermission::Edit,
            MaintenanceTool::LibraryDeleteUntracked
            | MaintenanceTool::LibraryPurgeMissing
            | MaintenanceTool::LibraryCleanDangling => ToolPermission::Delete,
        }
    }

    fn describe(self) -> MaintenanceToolDto {
        let (name, description, destructive, items_label) = match self {
            MaintenanceTool::LibraryScan => (
                "图书馆扫描对比",
                "对比图书馆目录中的图书文件与数据库记录，列出缺失文件的记录、未入库的文件和多余附属文件。只读，不修改任何数据。",
                false,
                None,
            ),
            MaintenanceTool::LibraryImportUntracked => (
                "导入未入库图书",
                "把磁盘上有、数据库中没有的图书导入数据库（读取同名 metadata，没有则按文件名生成）。",
                false,
                Some("图书文件路径"),
            ),
            MaintenanceTool::LibraryResyncMetadata => (
                "以磁盘 metadata 覆盖数据库",
                "用每本书同名 metadata.json 的内容覆盖数据库中的元数据与分类。",
                true,
                Some("图书 ID"),
            ),
            MaintenanceTool::LibraryDeleteUntracked => (
                "删除未入库的图书文件",
                "从磁盘删除数据库中没有记录的图书文件，以及仅属于它的 metadata / 封面。",
                true,
                Some("图书文件路径"),
            ),
            MaintenanceTool::LibraryPurgeMissing => (
                "删除缺失文件的图书记录",
                "从数据库删除图书文件已不存在的记录（阅读进度等关联数据一并删除）。",
                true,
                Some("图书 ID"),
            ),
            MaintenanceTool::LibraryCleanDangling => (
                "清理多余附属文件",
                "删除分类目录中没有对应图书文件的 metadata、封面与缩略图。",
                true,
                Some("文件路径"),
            ),
            MaintenanceTool::LibraryRefreshHashes => (
                "刷新所有图书哈希",
                "重新计算每本书文件的 MD5 / partial MD5 / SHA-256，写回数据库与 metadata.json（用于 KOReader 匹配与重复检测）。文件缺失的图书会跳过。",
                false,
                Some("图书 ID"),
            ),
            MaintenanceTool::LibraryFillMissingHashes => (
                "补充缺失的图书哈希",
                "只为缺少 MD5 / partial MD5 / SHA-256 中任一项的图书计算哈希并写回。",
                false,
                Some("图书 ID"),
            ),
        };
        let mut params = vec![ToolParamDto {
            name: "library_id".into(),
            label: "图书馆".into(),
            kind: ToolParamKind::Library,
            required: true,
            description: None,
        }];
        if let Some(label) = items_label {
            params.push(ToolParamDto {
                name: "items".into(),
                label: label.into(),
                kind: ToolParamKind::StringList,
                required: false,
                description: Some("留空则处理扫描到的全部项".into()),
            });
        }
        MaintenanceToolDto {
            id: self,
            name: name.into(),
            description: description.into(),
            required_permission: self.permission(),
            destructive,
            params,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermission {
    Edit,
    Delete,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolParamKind {
    /// 图书馆 ID
    Library,
    /// 字符串数组（通常来自扫描结果中勾选的项）
    StringList,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ToolParamDto {
    pub name: String,
    pub label: String,
    pub kind: ToolParamKind,
    pub required: bool,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MaintenanceToolDto {
    pub id: MaintenanceTool,
    pub name: String,
    pub description: String,
    /// 在目标图书馆上所需的权限
    pub required_permission: ToolPermission,
    /// 会删除或覆盖数据，前端应二次确认
    pub destructive: bool,
    pub params: Vec<ToolParamDto>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct StartTaskRequest {
    pub tool: MaintenanceTool,
    /// 工具参数，字段见 `MaintenanceToolDto.params`
    #[serde(default)]
    #[schema(value_type = Object)]
    pub params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct LibraryToolParams {
    library_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    items: Option<Vec<String>>,
}

pub fn list_tools() -> Vec<MaintenanceToolDto> {
    MaintenanceTool::ALL.iter().map(|t| t.describe()).collect()
}

pub async fn start_task(
    state: &AppState,
    user: &AuthUser,
    req: StartTaskRequest,
) -> AppResult<TaskDto> {
    let params: LibraryToolParams = serde_json::from_value(req.params)
        .map_err(|e| AppError::BadRequest(format!("Invalid params: {e}")))?;
    let library_id = params.library_id;
    let perm = resolve_permission(&state.db, user, &library_id).await?;
    match req.tool.permission() {
        ToolPermission::Edit => require_edit(&perm)?,
        ToolPermission::Delete => require_delete(&perm)?,
    }
    library::get_library_root(&state.db, &library_id).await?;

    let tool = req.tool;
    let db = state.db.clone();
    let stored_params = serde_json::to_value(&params)?;
    let items = params.items;
    let id = state
        .tasks
        .spawn(
            &state.db,
            NewTask {
                tool: tool.as_str().into(),
                lock_key: format!("library:{library_id}"),
                library_id: Some(library_id),
                params: stored_params,
                created_by: user.id,
            },
            move |ctx| async move {
                let items = items.as_deref();
                let value = match tool {
                    MaintenanceTool::LibraryScan => {
                        serde_json::to_value(sync::scan_library(&db, &library_id, &ctx).await?)?
                    }
                    MaintenanceTool::LibraryImportUntracked => serde_json::to_value(
                        sync::import_untracked(&db, &library_id, items, &ctx).await?,
                    )?,
                    MaintenanceTool::LibraryResyncMetadata => serde_json::to_value(
                        sync::resync_metadata(&db, &library_id, items, &ctx).await?,
                    )?,
                    MaintenanceTool::LibraryDeleteUntracked => serde_json::to_value(
                        sync::delete_untracked(&db, &library_id, items, &ctx).await?,
                    )?,
                    MaintenanceTool::LibraryPurgeMissing => serde_json::to_value(
                        sync::purge_missing(&db, &library_id, items, &ctx).await?,
                    )?,
                    MaintenanceTool::LibraryCleanDangling => serde_json::to_value(
                        sync::clean_dangling(&db, &library_id, items, &ctx).await?,
                    )?,
                    MaintenanceTool::LibraryRefreshHashes => serde_json::to_value(
                        sync::refresh_hashes(&db, &library_id, items, false, &ctx).await?,
                    )?,
                    MaintenanceTool::LibraryFillMissingHashes => serde_json::to_value(
                        sync::refresh_hashes(&db, &library_id, items, true, &ctx).await?,
                    )?,
                };
                Ok(value)
            },
        )
        .await?;
    tasks::get_task(&state.db, &state.tasks, &id).await
}

async fn editable_library_ids(state: &AppState, user: &AuthUser) -> AppResult<HashSet<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT library_id FROM library_members WHERE user_id = ? AND (role = 'admin' OR can_edit = 1)",
    )
    .bind(user.id.to_string())
    .fetch_all(&state.db)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// 可见性：系统管理员看全部；其他用户仅看自己有编辑权限的图书馆上的任务
pub async fn list_tasks(
    state: &AppState,
    user: &AuthUser,
    active_only: bool,
) -> AppResult<Vec<TaskDto>> {
    let all = tasks::list_tasks(&state.db, &state.tasks, active_only, 200).await?;
    if user.role == "system_admin" {
        return Ok(all.into_iter().take(LIST_LIMIT).collect());
    }
    let allowed = editable_library_ids(state, user).await?;
    Ok(all
        .into_iter()
        .filter(|t| t.library_id.as_ref().is_some_and(|l| allowed.contains(l)))
        .take(LIST_LIMIT)
        .collect())
}

/// 无权访问的任务一律返回 404，避免泄露其存在
async fn visible_task(state: &AppState, user: &AuthUser, id: &Uuid) -> AppResult<TaskDto> {
    let task = tasks::get_task(&state.db, &state.tasks, id).await?;
    if user.role == "system_admin" {
        return Ok(task);
    }
    let not_found = || AppError::NotFound("Task not found".into());
    let library_id = task
        .library_id
        .as_deref()
        .and_then(|l| Uuid::parse_str(l).ok())
        .ok_or_else(not_found)?;
    let perm = resolve_permission(&state.db, user, &library_id)
        .await
        .map_err(|_| not_found())?;
    require_edit(&perm).map_err(|_| not_found())?;
    Ok(task)
}

pub async fn get_task(state: &AppState, user: &AuthUser, id: &Uuid) -> AppResult<TaskDto> {
    visible_task(state, user, id).await
}

pub async fn cancel_task(state: &AppState, user: &AuthUser, id: &Uuid) -> AppResult<TaskDto> {
    let task = visible_task(state, user, id).await?;
    if task.status != tasks::TaskStatus::Running {
        return Err(AppError::Conflict("Task is not running".into()));
    }
    state.tasks.cancel(&state.db, id).await?;
    tasks::get_task(&state.db, &state.tasks, id).await
}
