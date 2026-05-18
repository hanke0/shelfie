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
    /// 与 username 二选一
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
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
    /// 馆内角色：admin（馆管理员）或 member（成员）
    pub role: String,
    pub can_view: bool,
    pub can_edit: bool,
    pub can_delete: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserLibraryMembershipDto {
    pub library_id: String,
    pub library_name: String,
    pub role: String,
    #[serde(flatten)]
    pub permissions: PermissionFlags,
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

/// 仅系统管理员可修改成员角色与细粒度权限
pub fn can_edit_member_permissions(user: &AuthUser) -> bool {
    user.role == "system_admin"
}

/// 系统管理员或该馆 library admin（拉人 / 踢人）
pub async fn can_manage_members(
    db: &SqlitePool,
    user: &AuthUser,
    library_id: &Uuid,
) -> AppResult<bool> {
    if user.role == "system_admin" {
        return Ok(true);
    }
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT role FROM library_members WHERE library_id = ? AND user_id = ?",
    )
    .bind(library_id.to_string())
    .bind(user.id.to_string())
    .fetch_optional(db)
    .await?;
    Ok(row.is_some_and(|(role,)| role == "admin"))
}

async fn resolve_member_user_id(db: &SqlitePool, req: &AddMemberRequest) -> AppResult<String> {
    if let Some(id) = &req.user_id {
        if !id.is_empty() {
            return Ok(id.clone());
        }
    }
    if let Some(name) = &req.username {
        let name = name.trim();
        if !name.is_empty() {
            let row: Option<(String,)> =
                sqlx::query_as("SELECT id FROM users WHERE username = ?")
                    .bind(name)
                    .fetch_optional(db)
                    .await?;
            return row.ok_or_else(|| AppError::NotFound("User not found".into())).map(|(id,)| id);
        }
    }
    Err(AppError::BadRequest("user_id or username required".into()))
}

/// 管理员可见全部成员；普通用户仅返回自己在该馆的成员信息
pub async fn list_members_for_user(
    db: &SqlitePool,
    user: &AuthUser,
    library_id: &Uuid,
) -> AppResult<Vec<LibraryMemberDto>> {
    if can_manage_members(db, user, library_id).await? {
        list_members(db, library_id).await
    } else {
        let member = fetch_member(db, library_id, &user.id.to_string()).await?;
        Ok(vec![member])
    }
}

async fn fetch_member(
    db: &SqlitePool,
    library_id: &Uuid,
    user_id: &str,
) -> AppResult<LibraryMemberDto> {
    let row: Option<(String, String, String, i64, i64, i64)> = sqlx::query_as(
        r#"
        SELECT m.user_id, u.username, m.role, m.can_view, m.can_edit, m.can_delete
        FROM library_members m
        INNER JOIN users u ON u.id = m.user_id
        WHERE m.library_id = ? AND m.user_id = ?
        "#,
    )
    .bind(library_id.to_string())
    .bind(user_id)
    .fetch_optional(db)
    .await?;

    let (user_id, username, role, cv, ce, cd) =
        row.ok_or_else(|| AppError::NotFound("Not a member of this library".into()))?;

    Ok(LibraryMemberDto {
        user_id,
        username,
        role,
        permissions: PermissionFlags {
            can_view: cv != 0,
            can_edit: ce != 0,
            can_delete: cd != 0,
        },
    })
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

fn validate_library_role(role: &str) -> AppResult<()> {
    match role {
        "admin" | "member" => Ok(()),
        _ => Err(AppError::BadRequest(
            "library role must be admin or member".into(),
        )),
    }
}

pub async fn add_member(
    db: &SqlitePool,
    library_id: &Uuid,
    requester: &AuthUser,
    req: AddMemberRequest,
) -> AppResult<()> {
    validate_library_role(&req.role)?;
    if requester.role != "system_admin" && req.role == "admin" {
        return Err(AppError::Forbidden(
            "Only system admin can assign library admin role".into(),
        ));
    }
    let user_id = resolve_member_user_id(db, &req).await?;
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
    .bind(&user_id)
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
    validate_library_role(&req.role)?;

    let can_view = if req.role == "admin" {
        1
    } else {
        req.can_view as i64
    };
    let can_edit = if req.role == "admin" {
        1
    } else {
        req.can_edit as i64
    };
    let can_delete = if req.role == "admin" {
        1
    } else {
        req.can_delete as i64
    };

    let result = sqlx::query(
        "UPDATE library_members SET role = ?, can_view = ?, can_edit = ?, can_delete = ? WHERE library_id = ? AND user_id = ?",
    )
    .bind(&req.role)
    .bind(can_view)
    .bind(can_edit)
    .bind(can_delete)
    .bind(library_id.to_string())
    .bind(user_id)
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Member not found".into()));
    }
    Ok(())
}

/// 移除图书馆成员（馆管理员仅可移除普通成员）
pub async fn remove_member(
    db: &SqlitePool,
    library_id: &Uuid,
    requester: &AuthUser,
    target_user_id: &str,
) -> AppResult<()> {
    if !can_manage_members(db, requester, library_id).await? {
        return Err(AppError::Forbidden("Cannot manage library members".into()));
    }

    let target = fetch_member(db, library_id, target_user_id).await?;

    if target.role == "admin" && requester.role != "system_admin" {
        return Err(AppError::Forbidden(
            "Cannot remove a library admin; contact system admin".into(),
        ));
    }

    if target_user_id == requester.id.to_string() && requester.role != "system_admin" {
        return Err(AppError::BadRequest("Cannot remove yourself from the library".into()));
    }

    sqlx::query("DELETE FROM library_members WHERE library_id = ? AND user_id = ?")
        .bind(library_id.to_string())
        .bind(target_user_id)
        .execute(db)
        .await?;
    Ok(())
}

/// 查询用户在各图书馆的成员关系（系统管理员可查任意用户，普通用户仅能查自己）
pub async fn list_memberships_for_user(
    db: &SqlitePool,
    requester: &AuthUser,
    target_user_id: &str,
) -> AppResult<Vec<UserLibraryMembershipDto>> {
    if requester.role != "system_admin" && requester.id.to_string() != target_user_id {
        return Err(AppError::Forbidden("Cannot view other users' memberships".into()));
    }

    let rows: Vec<(String, String, String, i64, i64, i64)> = sqlx::query_as(
        r#"
        SELECT m.library_id, l.name, m.role, m.can_view, m.can_edit, m.can_delete
        FROM library_members m
        INNER JOIN libraries l ON l.id = m.library_id
        WHERE m.user_id = ?
        ORDER BY l.name
        "#,
    )
    .bind(target_user_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(library_id, library_name, role, cv, ce, cd)| UserLibraryMembershipDto {
            library_id,
            library_name,
            role,
            permissions: PermissionFlags {
                can_view: cv != 0,
                can_edit: ce != 0,
                can_delete: cd != 0,
            },
        })
        .collect())
}
