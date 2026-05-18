use crate::domain::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::infra::fs;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct LibraryDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub root_path: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PermissionFlags {
    pub can_view: bool,
    pub can_edit: bool,
    pub can_delete: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LibraryMemberDto {
    pub user_id: String,
    pub username: String,
    pub role: String,
    #[serde(flatten)]
    pub permissions: PermissionFlags,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLibraryRequest {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddMemberRequest {
    pub user_id: String,
    pub role: String,
    #[serde(default)]
    pub can_view: bool,
    #[serde(default)]
    pub can_edit: bool,
    #[serde(default)]
    pub can_delete: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMemberPermissionsRequest {
    pub can_view: bool,
    pub can_edit: bool,
    pub can_delete: bool,
}

pub struct EffectivePermission {
    pub can_view: bool,
    pub can_edit: bool,
    pub can_delete: bool,
}

pub async fn list_libraries(db: &SqlitePool, user: &AuthUser) -> AppResult<Vec<LibraryDto>> {
    if user.role == "system_admin" {
        let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, name, slug, root_path, created_at FROM libraries ORDER BY name",
        )
        .fetch_all(db)
        .await?;
        return Ok(rows
            .into_iter()
            .map(|(id, name, slug, root_path, created_at)| LibraryDto {
                id,
                name,
                slug,
                root_path,
                created_at,
            })
            .collect());
    }

    let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
        r#"
        SELECT l.id, l.name, l.slug, l.root_path, l.created_at
        FROM libraries l
        INNER JOIN library_members m ON m.library_id = l.id
        WHERE m.user_id = ? AND m.can_view = 1
        ORDER BY l.name
        "#,
    )
    .bind(user.id.to_string())
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name, slug, root_path, created_at)| LibraryDto {
            id,
            name,
            slug,
            root_path,
            created_at,
        })
        .collect())
}

pub async fn create_library(state: &AppState, user: &AuthUser, req: CreateLibraryRequest) -> AppResult<LibraryDto> {
    crate::domain::user::require_system_admin(user)?;

    let id = Uuid::new_v4();
    let root_path = fs::library_root(&state.config.data_root, &id);
    tokio::fs::create_dir_all(&root_path).await?;

    sqlx::query(
        "INSERT INTO libraries (id, name, slug, root_path) VALUES (?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(&req.name)
    .bind(&req.slug)
    .bind(root_path.to_string_lossy().as_ref())
    .execute(&state.db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return AppError::Conflict("Library slug already exists".into());
            }
        }
        AppError::from(e)
    })?;

    let created_at: (String,) =
        sqlx::query_as("SELECT created_at FROM libraries WHERE id = ?")
            .bind(id.to_string())
            .fetch_one(&state.db)
            .await?;

    Ok(LibraryDto {
        id: id.to_string(),
        name: req.name,
        slug: req.slug,
        root_path: root_path.to_string_lossy().to_string(),
        created_at: created_at.0,
    })
}

pub async fn get_library_root(db: &SqlitePool, library_id: &Uuid) -> AppResult<PathBuf> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT root_path FROM libraries WHERE id = ?")
            .bind(library_id.to_string())
            .fetch_optional(db)
            .await?;
    let (root_path,) = row.ok_or_else(|| AppError::NotFound("Library not found".into()))?;
    Ok(PathBuf::from(root_path))
}

pub async fn resolve_permission(
    db: &SqlitePool,
    user: &AuthUser,
    library_id: &Uuid,
) -> AppResult<EffectivePermission> {
    if user.role == "system_admin" {
        return Ok(EffectivePermission {
            can_view: true,
            can_edit: true,
            can_delete: true,
        });
    }

    let row: Option<(String, i64, i64, i64)> = sqlx::query_as(
        "SELECT role, can_view, can_edit, can_delete FROM library_members WHERE library_id = ? AND user_id = ?",
    )
    .bind(library_id.to_string())
    .bind(user.id.to_string())
    .fetch_optional(db)
    .await?;

    let (role, can_view, can_edit, can_delete) =
        row.ok_or_else(|| AppError::Forbidden("Not a member of this library".into()))?;

    if role == "admin" {
        return Ok(EffectivePermission {
            can_view: true,
            can_edit: true,
            can_delete: true,
        });
    }

    Ok(EffectivePermission {
        can_view: can_view != 0,
        can_edit: can_edit != 0,
        can_delete: can_delete != 0,
    })
}

pub fn require_view(p: &EffectivePermission) -> AppResult<()> {
    if p.can_view {
        Ok(())
    } else {
        Err(AppError::Forbidden("View permission required".into()))
    }
}

pub fn require_edit(p: &EffectivePermission) -> AppResult<()> {
    if p.can_edit {
        Ok(())
    } else {
        Err(AppError::Forbidden("Edit permission required".into()))
    }
}

pub fn require_delete(p: &EffectivePermission) -> AppResult<()> {
    if p.can_delete {
        Ok(())
    } else {
        Err(AppError::Forbidden("Delete permission required".into()))
    }
}

pub async fn list_members(db: &SqlitePool, library_id: &Uuid) -> AppResult<Vec<LibraryMemberDto>> {
    let rows: Vec<(String, String, String, i64, i64, i64)> = sqlx::query_as(
        r#"
        SELECT m.user_id, u.username, m.role, m.can_view, m.can_edit, m.can_delete
        FROM library_members m
        INNER JOIN users u ON u.id = m.user_id
        WHERE m.library_id = ?
        "#,
    )
    .bind(library_id.to_string())
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(user_id, username, role, cv, ce, cd)| LibraryMemberDto {
            user_id,
            username,
            role,
            permissions: PermissionFlags {
                can_view: cv != 0,
                can_edit: ce != 0,
                can_delete: cd != 0,
            },
        })
        .collect())
}

pub async fn add_member(
    db: &SqlitePool,
    library_id: &Uuid,
    req: AddMemberRequest,
) -> AppResult<()> {
    let can_view = if req.role == "admin" { 1 } else { req.can_view as i64 };
    let can_edit = if req.role == "admin" { 1 } else { req.can_edit as i64 };
    let can_delete = if req.role == "admin" { 1 } else { req.can_delete as i64 };

    sqlx::query(
        r#"
        INSERT INTO library_members (library_id, user_id, role, can_view, can_edit, can_delete)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(library_id, user_id) DO UPDATE SET
            role = excluded.role,
            can_view = excluded.can_view,
            can_edit = excluded.can_edit,
            can_delete = excluded.can_delete
        "#,
    )
    .bind(library_id.to_string())
    .bind(&req.user_id)
    .bind(&req.role)
    .bind(can_view)
    .bind(can_edit)
    .bind(can_delete)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_member_permissions(
    db: &SqlitePool,
    library_id: &Uuid,
    user_id: &str,
    req: UpdateMemberPermissionsRequest,
) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE library_members SET can_view = ?, can_edit = ?, can_delete = ? WHERE library_id = ? AND user_id = ? AND role = 'member'",
    )
    .bind(req.can_view as i64)
    .bind(req.can_edit as i64)
    .bind(req.can_delete as i64)
    .bind(library_id.to_string())
    .bind(user_id)
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Member not found or is admin".into()));
    }
    Ok(())
}
