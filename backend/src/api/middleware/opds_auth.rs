use crate::api::middleware::auth::{resolve_user_from_authorization, AuthContext};
use crate::error::AppError;
use crate::state::AppState;
use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};

pub async fn require_opds_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user = resolve_user_from_authorization(&state, req.headers().get(AUTHORIZATION)).await?;
    req.extensions_mut().insert(AuthContext(user));
    Ok(next.run(req).await)
}
