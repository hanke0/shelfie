use crate::domain::auth::AuthUser;
use crate::domain::koreader;
use crate::error::AppError;
use crate::state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
#[derive(Clone)]
pub struct KosyncAuth(pub AuthUser);

pub async fn require_kosync_auth(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let username = req
        .headers()
        .get("x-auth-user")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let auth_key = req
        .headers()
        .get("x-auth-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match koreader::authenticate_kosync(&state.db, username, auth_key).await {
        Ok(user) => {
            req.extensions_mut().insert(KosyncAuth(user));
            next.run(req).await
        }
        Err(e) => {
            let status = match &e {
                AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
                AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (
                status,
                Json(serde_json::json!({ "message": e.to_string() })),
            )
                .into_response()
        }
    }
}
