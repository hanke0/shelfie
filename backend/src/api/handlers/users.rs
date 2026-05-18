use crate::api::middleware::auth::AuthContext;
use crate::domain::user::{self, UserDto};
use crate::error::AppResult;
use crate::state::AppState;
use axum::{extract::State, Extension, Json};

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
