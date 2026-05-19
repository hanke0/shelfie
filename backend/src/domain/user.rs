use crate::domain::auth::{self, AuthUser};
use crate::domain::library::{self, AddMemberRequest};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use serde::Serialize;
use sqlx::SqlitePool;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub role: String,
}

pub async fn list_users(db: &SqlitePool) -> AppResult<Vec<UserDto>> {
    let rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT id, username, role FROM users ORDER BY username")
            .fetch_all(db)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, username, role)| UserDto { id, username, role })
        .collect())
}

pub fn require_system_admin(user: &AuthUser) -> AppResult<()> {
    if user.role == "system_admin" {
        Ok(())
    } else {
        Err(AppError::Forbidden("System admin required".into()))
    }
}

fn validate_user_role(role: &str) -> AppResult<()> {
    match role {
        "user" | "system_admin" => Ok(()),
        _ => Err(AppError::BadRequest(
            "role must be user or system_admin".into(),
        )),
    }
}

fn validate_library_role(role: &str) -> AppResult<()> {
    match role {
        "admin" | "member" => Ok(()),
        _ => Err(AppError::BadRequest(
            "library_role must be admin or member".into(),
        )),
    }
}

#[derive(Debug, Clone)]
pub struct CreateUserParams {
    pub username: String,
    pub password: String,
    pub role: String,
    pub library_id: Option<Uuid>,
    pub library_role: Option<String>,
    pub can_view: bool,
    pub can_edit: bool,
    pub can_delete: bool,
}

/// 创建用户；普通用户必须指定初始图书馆及权限
pub async fn create_user(
    state: &AppState,
    admin: &AuthUser,
    params: CreateUserParams,
) -> AppResult<AuthUser> {
    require_system_admin(admin)?;
    validate_user_role(&params.role)?;

    if params.role == "user" && params.library_id.is_none() {
        return Err(AppError::BadRequest(
            "library_id is required when creating a regular user".into(),
        ));
    }

    if params.role == "system_admin" && params.library_id.is_some() {
        return Err(AppError::BadRequest(
            "system_admin does not need library membership".into(),
        ));
    }

    let user =
        auth::register_user(&state.db, &params.username, &params.password, &params.role).await?;

    if let Some(library_id) = params.library_id {
        let library_role = params.library_role.unwrap_or_else(|| "member".to_string());
        validate_library_role(&library_role)?;

        // 确认图书馆存在
        library::get_library_root(&state.db, &library_id).await?;

        library::add_member(
            &state.db,
            &library_id,
            admin,
            AddMemberRequest {
                user_id: Some(user.id.to_string()),
                username: None,
                role: library_role,
                can_view: params.can_view,
                can_edit: params.can_edit,
                can_delete: params.can_delete,
            },
        )
        .await?;
    }

    Ok(user)
}

#[derive(Debug, Clone)]
pub struct ChangePasswordParams {
    pub current_password: Option<String>,
    pub new_password: String,
}

pub async fn update_username(
    db: &SqlitePool,
    requester: &AuthUser,
    target_user_id: &Uuid,
    raw_username: &str,
) -> AppResult<AuthUser> {
    if requester.id != *target_user_id {
        return Err(AppError::Forbidden("只能修改自己的用户名".into()));
    }

    let username = raw_username.trim();
    if username.is_empty() {
        return Err(AppError::BadRequest("username cannot be empty".into()));
    }
    if username.len() > 64 {
        return Err(AppError::BadRequest(
            "username must be at most 64 characters".into(),
        ));
    }
    if username == requester.username {
        return Ok(requester.clone());
    }

    let result = sqlx::query("UPDATE users SET username = ? WHERE id = ?")
        .bind(username)
        .bind(target_user_id.to_string())
        .execute(db)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db_err) = &e {
                if db_err.is_unique_violation() {
                    return AppError::Conflict("Username already exists".into());
                }
            }
            AppError::from(e)
        })?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".into()));
    }

    Ok(AuthUser {
        id: requester.id,
        username: username.to_string(),
        role: requester.role.clone(),
    })
}

pub async fn change_password(
    db: &SqlitePool,
    requester: &AuthUser,
    target_user_id: &Uuid,
    params: ChangePasswordParams,
) -> AppResult<()> {
    if params.new_password.len() < 6 {
        return Err(AppError::BadRequest(
            "password must be at least 6 characters".into(),
        ));
    }

    let is_self = requester.id == *target_user_id;
    if is_self {
        auth::change_password(
            db,
            target_user_id,
            params.current_password.as_deref(),
            &params.new_password,
            true,
        )
        .await
    } else {
        require_system_admin(requester)?;
        auth::change_password(db, target_user_id, None, &params.new_password, false).await
    }
}

pub async fn delete_user(
    db: &SqlitePool,
    admin: &AuthUser,
    target_user_id: &Uuid,
) -> AppResult<()> {
    require_system_admin(admin)?;

    if admin.id == *target_user_id {
        return Err(AppError::BadRequest(
            "Cannot delete your own account".into(),
        ));
    }

    let target_role: Option<(String,)> = sqlx::query_as("SELECT role FROM users WHERE id = ?")
        .bind(target_user_id.to_string())
        .fetch_optional(db)
        .await?;

    let (target_role,) = target_role.ok_or_else(|| AppError::NotFound("User not found".into()))?;

    if target_role == "system_admin" {
        let admin_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE role = 'system_admin'")
                .fetch_one(db)
                .await?;
        if admin_count.0 <= 1 {
            return Err(AppError::BadRequest(
                "Cannot delete the last system administrator".into(),
            ));
        }
    }

    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(target_user_id.to_string())
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".into()));
    }
    Ok(())
}
