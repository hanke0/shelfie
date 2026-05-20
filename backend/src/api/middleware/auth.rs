use crate::domain::auth::{self, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use base64::Engine;

#[derive(Clone, Debug)]
pub struct AuthContext(pub AuthUser);

pub async fn resolve_user_from_authorization(
    state: &AppState,
    auth_header: Option<&axum::http::HeaderValue>,
) -> AppResult<AuthUser> {
    let header = auth_header
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

    if let Some(token) = header.strip_prefix("Bearer ") {
        return auth::authenticate_bearer_token(&state.db, &state.config.jwt_secret, token.trim())
            .await;
    }

    if let Some(encoded) = header.strip_prefix("Basic ") {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded.trim())
            .map_err(|_| AppError::Unauthorized("Invalid Basic credentials".into()))?;
        let creds = String::from_utf8(decoded)
            .map_err(|_| AppError::Unauthorized("Invalid Basic credentials".into()))?;
        let (username, password) = creds
            .split_once(':')
            .ok_or_else(|| AppError::Unauthorized("Invalid Basic credentials".into()))?;
        return auth::authenticate_basic_credentials(&state.db, username, password).await;
    }

    Err(AppError::Unauthorized(
        "Use Authorization: Bearer or Basic".into(),
    ))
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user = resolve_user_from_authorization(&state, req.headers().get(AUTHORIZATION)).await?;
    req.extensions_mut().insert(AuthContext(user));
    Ok(next.run(req).await)
}

pub fn auth_user(req: &Request) -> AppResult<&AuthUser> {
    req.extensions()
        .get::<AuthContext>()
        .map(|c| &c.0)
        .ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))
}
