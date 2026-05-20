use crate::api::handlers::auth::{LoginResponse, UserInfo};
use crate::api::middleware::auth::AuthContext;
use crate::domain::auth;
use crate::domain::library::{self, UserLibraryMembershipDto};
use crate::domain::reading_history::{self, ReadingHistoryEntry};
use crate::domain::user::{self, ChangePasswordParams, UserDto};
use crate::error::AppResult;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub struct UpdateUsernameRequest {
    pub username: String,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ReadingHistoryQuery {
    pub library_id: Option<String>,
    #[param(default = 50, minimum = 1, maximum = 200)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/users/me/reading-history",
    tag = "Users",
    params(ReadingHistoryQuery),
    responses((status = 200, body = [ReadingHistoryEntry])),
    security(("bearer_auth" = []))
)]
pub async fn list_my_reading_history(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Query(q): Query<ReadingHistoryQuery>,
) -> AppResult<Json<Vec<ReadingHistoryEntry>>> {
    let library_id = match q.library_id {
        None => None,
        Some(s) => Some(
            Uuid::parse_str(&s)
                .map_err(|_| crate::error::AppError::BadRequest("Invalid library id".into()))?,
        ),
    };
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = q.offset.unwrap_or(0).max(0);
    Ok(Json(
        reading_history::list_for_user(&state.db, &user, library_id, limit, offset).await?,
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    /// 修改自己的密码时必填
    #[serde(default)]
    pub current_password: Option<String>,
    pub new_password: String,
}

#[utoipa::path(
    get,
    path = "/users",
    tag = "Users",
    responses((status = 200, body = [UserDto])),
    security(("bearer_auth" = []))
)]
pub async fn list_users(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
) -> AppResult<Json<Vec<UserDto>>> {
    user::require_system_admin(&user)?;
    Ok(Json(user::list_users(&state.db).await?))
}

#[utoipa::path(
    get,
    path = "/users/{user_id}/memberships",
    tag = "Users",
    params(("user_id" = String, Path)),
    responses((status = 200, body = [UserLibraryMembershipDto])),
    security(("bearer_auth" = []))
)]
pub async fn list_user_memberships(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(user_id): Path<String>,
) -> AppResult<Json<Vec<UserLibraryMembershipDto>>> {
    Ok(Json(
        library::list_memberships_for_user(&state.db, &user, &user_id).await?,
    ))
}

#[utoipa::path(
    patch,
    path = "/users/{user_id}/username",
    tag = "Users",
    params(("user_id" = String, Path)),
    request_body = UpdateUsernameRequest,
    responses((status = 200, body = LoginResponse)),
    security(("bearer_auth" = []))
)]
pub async fn update_username(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateUsernameRequest>,
) -> AppResult<Json<LoginResponse>> {
    let target_id = Uuid::parse_str(&user_id)
        .map_err(|_| crate::error::AppError::BadRequest("Invalid user id".into()))?;
    let updated = user::update_username(&state.db, &user, &target_id, &req.username).await?;
    let token = auth::issue_token(&state.config.jwt_secret, &updated)?;
    Ok(Json(LoginResponse {
        token,
        user: UserInfo {
            id: updated.id.to_string(),
            username: updated.username,
            role: updated.role,
        },
    }))
}

#[utoipa::path(
    patch,
    path = "/users/{user_id}/password",
    tag = "Users",
    params(("user_id" = String, Path)),
    request_body = ChangePasswordRequest,
    responses((status = 204)),
    security(("bearer_auth" = []))
)]
pub async fn change_password(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(user_id): Path<String>,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<axum::http::StatusCode> {
    let target_id = Uuid::parse_str(&user_id)
        .map_err(|_| crate::error::AppError::BadRequest("Invalid user id".into()))?;
    user::change_password(
        &state.db,
        &user,
        &target_id,
        ChangePasswordParams {
            current_password: req.current_password,
            new_password: req.new_password,
        },
    )
    .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/users/{user_id}",
    tag = "Users",
    params(("user_id" = String, Path)),
    responses((status = 204)),
    security(("bearer_auth" = []))
)]
pub async fn delete_user(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(user_id): Path<String>,
) -> AppResult<axum::http::StatusCode> {
    let target_id = Uuid::parse_str(&user_id)
        .map_err(|_| crate::error::AppError::BadRequest("Invalid user id".into()))?;
    user::delete_user(&state.db, &user, &target_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
