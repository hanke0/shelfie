use crate::api::middleware::auth::AuthContext;
use crate::domain::opds;
use crate::error::AppResult;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OpdsBooksQuery {
    pub category: Option<String>,
}

fn opds_response(xml: String) -> Response {
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "application/atom+xml;profile=opds-catalog;charset=utf-8",
        )],
        xml,
    )
        .into_response()
}

fn request_base_url(state: &AppState, headers: &HeaderMap) -> String {
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok());
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok());
    opds::resolve_base_url(state.config.public_base_url.as_deref(), proto, host)
}

pub async fn opds_root(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let base = request_base_url(&state, &headers);
    let xml = opds::root_navigation_feed(&state, &user, &base).await?;
    Ok(opds_response(xml))
}

pub async fn opds_library(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    headers: HeaderMap,
    axum::extract::Path(library_id): axum::extract::Path<String>,
) -> AppResult<Response> {
    let id = opds::parse_library_id(&library_id)?;
    let base = request_base_url(&state, &headers);
    let xml = opds::library_navigation_feed(&state, &user, &id, &base).await?;
    Ok(opds_response(xml))
}

pub async fn opds_library_books(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    headers: HeaderMap,
    axum::extract::Path(library_id): axum::extract::Path<String>,
    Query(query): Query<OpdsBooksQuery>,
) -> AppResult<Response> {
    let id = opds::parse_library_id(&library_id)?;
    let base = request_base_url(&state, &headers);
    let category = query.category.as_deref();
    let xml = opds::library_acquisition_feed(&state, &user, &id, category, &base).await?;
    Ok(opds_response(xml))
}
