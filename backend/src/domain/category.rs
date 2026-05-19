use crate::domain::auth::AuthUser;
use crate::domain::library;
use crate::error::{AppError, AppResult};
use crate::infra::{fs, safe_name};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::BTreeSet;
use std::path::Path;
use utoipa::ToSchema;
use uuid::Uuid;

pub const DEFAULT_CATEGORY: &str = "未分类";

#[derive(Debug, Serialize, ToSchema)]
pub struct CategoryDto {
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCategoryRequest {
    pub name: String,
}

/// 校验并规范化为磁盘/数据库使用的分类名（与 `category_dir` 一致）
pub fn canonical_category_name(input: &str) -> AppResult<String> {
    safe_name::validate_category(input)?;
    let seg = safe_name::sanitize_path_segment(input.trim());
    if seg.is_empty() {
        return Err(AppError::BadRequest("category cannot be empty".into()));
    }
    safe_name::validate_category(&seg)?;
    Ok(seg)
}

async fn collect_category_names(
    db: &SqlitePool,
    library_id: &Uuid,
    lib_root: &Path,
) -> AppResult<Vec<String>> {
    let mut names = BTreeSet::new();

    for name in fs::list_library_categories(lib_root).await? {
        names.insert(name);
    }

    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT category FROM books WHERE library_id = ? ORDER BY category",
    )
    .bind(library_id.to_string())
    .fetch_all(db)
    .await?;

    for (name,) in rows {
        names.insert(name);
    }

    Ok(names.into_iter().collect())
}

pub async fn list_categories(
    state: &AppState,
    user: &AuthUser,
    library_id: &Uuid,
) -> AppResult<Vec<CategoryDto>> {
    let perm = library::resolve_permission(&state.db, user, library_id).await?;
    library::require_view(&perm)?;

    let lib_root = library::get_library_root(&state.db, library_id).await?;
    let names = collect_category_names(&state.db, library_id, &lib_root).await?;

    Ok(names.into_iter().map(|name| CategoryDto { name }).collect())
}

pub async fn create_category(
    state: &AppState,
    user: &AuthUser,
    library_id: &Uuid,
    req: CreateCategoryRequest,
) -> AppResult<CategoryDto> {
    let perm = library::resolve_permission(&state.db, user, library_id).await?;
    library::require_edit(&perm)?;

    let name = canonical_category_name(&req.name)?;
    let lib_root = library::get_library_root(&state.db, library_id).await?;
    let dir = fs::category_dir(&lib_root, &name)?;

    if dir.exists() {
        if dir.is_dir() {
            return Ok(CategoryDto { name });
        }
        return Err(AppError::Conflict(
            "A file exists at the category path".into(),
        ));
    }

    tokio::fs::create_dir_all(&dir).await?;
    Ok(CategoryDto { name })
}

pub async fn delete_category(
    state: &AppState,
    user: &AuthUser,
    library_id: &Uuid,
    category: &str,
) -> AppResult<()> {
    let perm = library::resolve_permission(&state.db, user, library_id).await?;
    library::require_edit(&perm)?;

    if category == DEFAULT_CATEGORY {
        return Err(AppError::BadRequest(
            "Cannot delete the default category".into(),
        ));
    }

    let name = canonical_category_name(category)?;
    let lib_root = library::get_library_root(&state.db, library_id).await?;
    let dir = fs::category_dir(&lib_root, &name)?;

    let book_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM books WHERE library_id = ? AND category = ?")
            .bind(library_id.to_string())
            .bind(&name)
            .fetch_one(&state.db)
            .await?;

    if book_count.0 > 0 {
        return Err(AppError::Conflict(
            "Cannot delete category while it contains books".into(),
        ));
    }

    if dir.exists() && fs::dir_has_any_files(&dir).await? {
        return Err(AppError::Conflict(
            "Cannot delete category while its directory contains files".into(),
        ));
    }

    if dir.exists() {
        tokio::fs::remove_dir(&dir).await?;
    }

    Ok(())
}

pub async fn ensure_default_category(lib_root: &Path) -> AppResult<()> {
    let dir = fs::category_dir(lib_root, DEFAULT_CATEGORY)?;
    if !dir.exists() {
        tokio::fs::create_dir_all(&dir).await?;
    }
    Ok(())
}

pub async fn require_category_exists(
    db: &SqlitePool,
    library_id: &Uuid,
    lib_root: &Path,
    category: &str,
) -> AppResult<()> {
    let name = canonical_category_name(category)?;
    let names = collect_category_names(db, library_id, lib_root).await?;
    if names.iter().any(|n| n == &name) {
        return Ok(());
    }
    Err(AppError::BadRequest(format!(
        "Category '{name}' does not exist; create it in library settings first"
    )))
}
