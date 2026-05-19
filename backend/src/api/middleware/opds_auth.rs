use crate::api::middleware::auth::AuthContext;
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

pub async fn require_opds_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user = authenticate_opds_request(&state, req.headers().get(AUTHORIZATION)).await?;
    req.extensions_mut().insert(AuthContext(user));
    Ok(next.run(req).await)
}

async fn authenticate_opds_request(
    state: &AppState,
    auth_header: Option<&axum::http::HeaderValue>,
) -> AppResult<AuthUser> {
    let header = auth_header
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

    if let Some(token) = header.strip_prefix("Bearer ") {
        let claims = auth::decode_token(&state.config.jwt_secret, token.trim())?;
        return Ok(AuthUser {
            id: uuid::Uuid::parse_str(&claims.sub)
                .map_err(|e| AppError::Unauthorized(e.to_string()))?,
            username: claims.username,
            role: claims.role,
        });
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
        let (_, user) =
            auth::login(&state.db, &state.config.jwt_secret, username, password).await?;
        return Ok(user);
    }

    Err(AppError::Unauthorized(
        "Use Authorization: Basic or Bearer".into(),
    ))
}
