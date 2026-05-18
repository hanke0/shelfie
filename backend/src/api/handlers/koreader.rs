use crate::api::middleware::auth::AuthContext;
use crate::api::middleware::kosync_auth::KosyncAuth;
use crate::domain::koreader::{
    self, KoreaderDocumentLink, KoreaderProgressRow, KosyncAuthResponse, KosyncProgressResponse,
    KosyncUpdateRequest, KosyncUpdateResponse, SetDocumentLinkRequest,
};
use crate::error::AppResult;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    Extension, Json,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct LibraryFilterQuery {
    pub library_id: Option<Uuid>,
}

fn ensure_koreader_accept(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("application/vnd.koreader"))
        .unwrap_or(true)
}

#[utoipa::path(
    get,
    path = "/healthcheck",
    tag = "KOReader",
    responses((status = 200, body = serde_json::Value))
)]
pub async fn kosync_healthcheck() -> Json<serde_json::Value> {
    Json(koreader::kosync_healthcheck().await)
}

#[utoipa::path(
    get,
    path = "/users/auth",
    tag = "KOReader",
    responses((status = 200, body = KosyncAuthResponse), (status = 401))
)]
pub async fn kosync_auth_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<KosyncAuthResponse>> {
    let _ = ensure_koreader_accept(&headers);
    let username = headers
        .get("x-auth-user")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let auth_key = headers
        .get("x-auth-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let body = koreader::kosync_auth_user(&state.db, username, auth_key).await?;
    Ok(Json(body))
}

#[utoipa::path(
    put,
    path = "/syncs/progress",
    tag = "KOReader",
    request_body = KosyncUpdateRequest,
    responses((status = 200, body = KosyncUpdateResponse), (status = 401))
)]
pub async fn kosync_update_progress(
    State(state): State<AppState>,
    Extension(KosyncAuth(user)): Extension<KosyncAuth>,
    headers: HeaderMap,
    Json(req): Json<KosyncUpdateRequest>,
) -> AppResult<Json<KosyncUpdateResponse>> {
    let _ = ensure_koreader_accept(&headers);
    let body = koreader::kosync_update_progress(&state, &user, req).await?;
    Ok(Json(body))
}

#[utoipa::path(
    get,
    path = "/syncs/progress/{document}",
    tag = "KOReader",
    responses((status = 200, body = KosyncProgressResponse), (status = 401))
)]
pub async fn kosync_get_progress(
    State(state): State<AppState>,
    Extension(KosyncAuth(user)): Extension<KosyncAuth>,
    Path(document): Path<String>,
    headers: HeaderMap,
) -> AppResult<Json<KosyncProgressResponse>> {
    let _ = ensure_koreader_accept(&headers);
    let body = koreader::kosync_get_progress(&state, &user, &document).await?;
    Ok(Json(body))
}

#[utoipa::path(
    get,
    path = "/koreader/progress",
    tag = "KOReader",
    params(("library_id" = Option<Uuid>, Query, description = "Filter by library")),
    responses((status = 200, body = [KoreaderProgressRow])),
    security(("bearer_auth" = []))
)]
pub async fn list_koreader_progress(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Query(q): Query<LibraryFilterQuery>,
) -> AppResult<Json<Vec<KoreaderProgressRow>>> {
    Ok(Json(koreader::list_progress(&state, &user, q.library_id).await?))
}

#[utoipa::path(
    get,
    path = "/koreader/links",
    tag = "KOReader",
    params(("library_id" = Option<Uuid>, Query, description = "Filter by library")),
    responses((status = 200, body = [KoreaderDocumentLink])),
    security(("bearer_auth" = []))
)]
pub async fn list_koreader_links(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Query(q): Query<LibraryFilterQuery>,
) -> AppResult<Json<Vec<KoreaderDocumentLink>>> {
    Ok(Json(
        koreader::list_document_links(&state, &user, q.library_id).await?,
    ))
}

#[utoipa::path(
    put,
    path = "/koreader/links",
    tag = "KOReader",
    request_body = SetDocumentLinkRequest,
    responses((status = 200, body = KoreaderDocumentLink)),
    security(("bearer_auth" = []))
)]
pub async fn set_koreader_link(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Json(req): Json<SetDocumentLinkRequest>,
) -> AppResult<Json<KoreaderDocumentLink>> {
    Ok(Json(koreader::set_document_link(&state, &user, req).await?))
}

#[utoipa::path(
    delete,
    path = "/koreader/links/{document}",
    tag = "KOReader",
    responses((status = 204)),
    security(("bearer_auth" = []))
)]
pub async fn delete_koreader_link(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(document): Path<String>,
) -> AppResult<StatusCode> {
    koreader::delete_document_link(&state, &user, &document).await?;
    Ok(StatusCode::NO_CONTENT)
}
