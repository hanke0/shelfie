use crate::domain::auth::AuthUser;
use crate::error::{AppError, AppResult};
use serde::Serialize;
use sqlx::SqlitePool;
use utoipa::ToSchema;

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
        .map(|(id, username, role)| UserDto {
            id,
            username,
            role,
        })
        .collect())
}

pub fn require_system_admin(user: &AuthUser) -> AppResult<()> {
    if user.role == "system_admin" {
        Ok(())
    } else {
        Err(AppError::Forbidden("System admin required".into()))
    }
}
