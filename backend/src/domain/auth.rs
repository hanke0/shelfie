use crate::error::{AppError, AppResult};
use crate::state::AppState;
use bcrypt::{hash, verify, DEFAULT_COST};
use crate::infra::hash::md5_hex;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub username: String,
    pub role: String,
    pub exp: i64,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}

pub async fn login(
    db: &SqlitePool,
    jwt_secret: &str,
    username: &str,
    password: &str,
) -> AppResult<(String, AuthUser)> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT id, password_hash, role FROM users WHERE username = ?",
    )
    .bind(username)
    .fetch_optional(db)
    .await?;

    let (id, password_hash, role) = row.ok_or_else(|| AppError::Unauthorized("Invalid credentials".into()))?;

    if !verify(password, &password_hash)? {
        return Err(AppError::Unauthorized("Invalid credentials".into()));
    }

    let password_md5 = md5_hex(password.as_bytes());
    sqlx::query("UPDATE users SET password_md5 = ? WHERE id = ?")
        .bind(&password_md5)
        .bind(&id)
        .execute(db)
        .await?;

    let user = AuthUser {
        id: Uuid::parse_str(&id).map_err(|e| AppError::Internal(e.to_string()))?,
        username: username.to_string(),
        role,
    };

    let token = issue_token(jwt_secret, &user)?;
    Ok((token, user))
}

pub async fn change_password(
    db: &SqlitePool,
    user_id: &Uuid,
    current_password: Option<&str>,
    new_password: &str,
    verify_current: bool,
) -> AppResult<()> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT password_hash FROM users WHERE id = ?")
            .bind(user_id.to_string())
            .fetch_optional(db)
            .await?;

    let (password_hash,) = row.ok_or_else(|| AppError::NotFound("User not found".into()))?;

    if verify_current {
        let current = current_password.ok_or_else(|| {
            AppError::BadRequest("current_password is required".into())
        })?;
        if !verify(current, &password_hash)? {
            return Err(AppError::Unauthorized("Invalid current password".into()));
        }
    }

    let new_hash = hash(new_password, DEFAULT_COST)?;
    let password_md5 = md5_hex(new_password.as_bytes());
    sqlx::query("UPDATE users SET password_hash = ?, password_md5 = ? WHERE id = ?")
        .bind(new_hash)
        .bind(password_md5)
        .bind(user_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}

pub async fn register_user(
    db: &SqlitePool,
    username: &str,
    password: &str,
    role: &str,
) -> AppResult<AuthUser> {
    let id = Uuid::new_v4();
    let password_hash = hash(password, DEFAULT_COST)?;
    let password_md5 = md5_hex(password.as_bytes());
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, password_md5, role) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(username)
    .bind(password_hash)
    .bind(password_md5)
    .bind(role)
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

    Ok(AuthUser {
        id,
        username: username.to_string(),
        role: role.to_string(),
    })
}

pub fn issue_token(jwt_secret: &str, user: &AuthUser) -> AppResult<String> {
    let exp = (Utc::now() + Duration::hours(24 * 7)).timestamp();
    let claims = JwtClaims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: user.role.clone(),
        exp,
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )?)
}

pub fn decode_token(jwt_secret: &str, token: &str) -> AppResult<JwtClaims> {
    let data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

pub async fn ensure_default_admin(state: &AppState) -> AppResult<()> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    if count.0 == 0 {
        register_user(&state.db, "admin", "admin123", "system_admin").await?;
        tracing::info!("Created default admin user: admin / admin123");
    }
    Ok(())
}
