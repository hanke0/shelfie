use crate::domain::auth::{decode_token, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct AuthContext(pub AuthUser);

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid Authorization scheme".into()))?;

    let claims = decode_token(&state.config.jwt_secret, token)?;
    let user = AuthUser {
        id: Uuid::parse_str(&claims.sub).map_err(|e| AppError::Unauthorized(e.to_string()))?,
        username: claims.username,
        role: claims.role,
    };

    req.extensions_mut().insert(AuthContext(user));
    Ok(next.run(req).await)
}

pub fn auth_user(req: &Request) -> AppResult<&AuthUser> {
    req.extensions()
        .get::<AuthContext>()
        .map(|c| &c.0)
        .ok_or_else(|| AppError::Unauthorized("Not authenticated".into()))
}
