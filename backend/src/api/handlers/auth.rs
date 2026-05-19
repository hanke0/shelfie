use crate::api::middleware::auth::AuthContext;
use crate::domain::auth::{self, AuthUser};
use crate::domain::user::{self, CreateUserParams};
use crate::error::AppResult;
use crate::state::AppState;
use axum::{extract::State, Extension, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Serialize, ToSchema)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    #[serde(default = "default_role")]
    pub role: String,
    /// 普通用户必填：加入的图书馆
    #[serde(default)]
    pub library_id: Option<uuid::Uuid>,
    /// 在图书馆内的角色：admin（馆管理员）或 member（成员）
    #[serde(default)]
    pub library_role: Option<String>,
    #[serde(default = "default_true")]
    pub can_view: bool,
    #[serde(default)]
    pub can_edit: bool,
    #[serde(default)]
    pub can_delete: bool,
}

fn default_true() -> bool {
    true
}

fn default_role() -> String {
    "user".into()
}

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses((status = 200, body = LoginResponse))
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let (token, user) = auth::login(
        &state.db,
        &state.config.jwt_secret,
        &req.username,
        &req.password,
    )
    .await?;
    Ok(Json(LoginResponse {
        token,
        user: to_user_info(&user),
    }))
}

#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses((status = 201, body = UserInfo)),
    security(("bearer_auth" = []))
)]
pub async fn register(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<(axum::http::StatusCode, Json<UserInfo>)> {
    let created = user::create_user(
        &state,
        &user,
        CreateUserParams {
            username: req.username,
            password: req.password,
            role: req.role,
            library_id: req.library_id,
            library_role: req.library_role,
            can_view: req.can_view,
            can_edit: req.can_edit,
            can_delete: req.can_delete,
        },
    )
    .await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(to_user_info(&created)),
    ))
}

fn to_user_info(user: &AuthUser) -> UserInfo {
    UserInfo {
        id: user.id.to_string(),
        username: user.username.clone(),
        role: user.role.clone(),
    }
}

pub async fn me(Extension(AuthContext(user)): Extension<AuthContext>) -> Json<UserInfo> {
    Json(to_user_info(&user))
}
