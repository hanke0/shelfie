use crate::domain::auth::AuthUser;
use crate::domain::book::{mark_orphan, upsert_from_fs};
use crate::domain::library::{self, require_edit, resolve_permission};
use crate::error::AppResult;
use crate::infra::fs;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshDiff {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub removed: Vec<String>,
    /// 已从磁盘删除的无主附属文件（metadata / 封面 / 缩略图）路径
    pub cleaned: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshJobResponse {
    pub job_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<RefreshDiff>,
}

pub async fn start_refresh(
    state: &AppState,
    user: &AuthUser,
    library_id: &Uuid,
    prefer_db: bool,
) -> AppResult<RefreshJobResponse> {
    let perm = resolve_permission(&state.db, user, library_id).await?;
    require_edit(&perm)?;

    let job_id = Uuid::new_v4();
    sqlx::query("INSERT INTO refresh_jobs (id, library_id, status) VALUES (?, ?, 'running')")
        .bind(job_id.to_string())
        .bind(library_id.to_string())
        .execute(&state.db)
        .await?;

    let diff = run_refresh(state, library_id, prefer_db).await;
    let status = "completed";

    let result_json = serde_json::to_string(&diff)?;
    sqlx::query(
        "UPDATE refresh_jobs SET status = ?, result = ?, completed_at = datetime('now') WHERE id = ?",
    )
    .bind(status)
    .bind(&result_json)
    .bind(job_id.to_string())
    .execute(&state.db)
    .await?;

    Ok(RefreshJobResponse {
        job_id: job_id.to_string(),
        status: status.to_string(),
        result: Some(diff),
    })
}

pub async fn get_refresh_job(
    state: &AppState,
    user: &AuthUser,
    library_id: &Uuid,
    job_id: &Uuid,
) -> AppResult<RefreshJobResponse> {
    let perm = resolve_permission(&state.db, user, library_id).await?;
    require_edit(&perm)?;

    let row: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT status, result FROM refresh_jobs WHERE id = ? AND library_id = ?")
            .bind(job_id.to_string())
            .bind(library_id.to_string())
            .fetch_optional(&state.db)
            .await?;

    let (status, result) =
        row.ok_or_else(|| crate::error::AppError::NotFound("Job not found".into()))?;
    let diff = result.map(|r| serde_json::from_str(&r)).transpose()?;

    Ok(RefreshJobResponse {
        job_id: job_id.to_string(),
        status,
        result: diff,
    })
}

async fn run_refresh(state: &AppState, library_id: &Uuid, prefer_db: bool) -> RefreshDiff {
    let mut diff = RefreshDiff {
        added: vec![],
        updated: vec![],
        removed: vec![],
        cleaned: vec![],
        errors: vec![],
    };

    let lib_root = match library::get_library_root(&state.db, library_id).await {
        Ok(p) => p,
        Err(e) => {
            diff.errors.push(e.to_string());
            return diff;
        }
    };

    let scanned = match fs::scan_library_books(&lib_root).await {
        Ok(s) => s,
        Err(e) => {
            diff.errors.push(e.to_string());
            return diff;
        }
    };

    let mut fs_paths = HashSet::new();

    for entry in &scanned {
        let path_str = entry.book_file.to_string_lossy().to_string();
        fs_paths.insert(path_str.clone());

        let existing: Option<(String,)> =
            sqlx::query_as("SELECT id FROM books WHERE book_file_path = ?")
                .bind(&path_str)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();

        if prefer_db {
            if existing.is_none() {
                let book_id = Uuid::new_v4();
                match upsert_from_fs(
                    &state.db,
                    library_id,
                    &entry.category,
                    &book_id,
                    &entry.book_file,
                )
                .await
                {
                    Ok(()) => diff.added.push(book_id.to_string()),
                    Err(e) => diff.errors.push(format!("{path_str}: {e}")),
                }
            }
            continue;
        }

        let is_update = existing.is_some();
        let book_id = if let Some((id,)) = existing {
            Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::new_v4())
        } else {
            Uuid::new_v4()
        };

        match upsert_from_fs(
            &state.db,
            library_id,
            &entry.category,
            &book_id,
            &entry.book_file,
        )
        .await
        {
            Ok(()) => {
                if is_update {
                    diff.updated.push(book_id.to_string());
                } else {
                    diff.added.push(book_id.to_string());
                }
            }
            Err(e) => diff.errors.push(format!("{path_str}: {e}")),
        }
    }

    let db_rows: Vec<(String, String)> =
        match sqlx::query_as("SELECT id, book_file_path FROM books WHERE library_id = ?")
            .bind(library_id.to_string())
            .fetch_all(&state.db)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                diff.errors.push(e.to_string());
                return diff;
            }
        };

    for (id, book_file_path) in db_rows {
        if !fs_paths.contains(&book_file_path) && !Path::new(&book_file_path).is_file() {
            if let Ok(book_id) = Uuid::parse_str(&id) {
                if let Err(e) = mark_orphan(&state.db, &book_id).await {
                    diff.errors.push(format!("{id}: {e}"));
                } else {
                    diff.removed.push(id);
                }
            }
        }
    }

    match fs::cleanup_dangling_files(&lib_root, &scanned).await {
        Ok(removed) => {
            diff.cleaned = removed
                .into_iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
        }
        Err(e) => diff.errors.push(format!("cleanup: {e}")),
    }

    diff
}
