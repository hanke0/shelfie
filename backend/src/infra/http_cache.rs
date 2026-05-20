use crate::error::AppResult;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 封面/缩略图浏览器私有缓存时长（秒）
pub const COVER_CACHE_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60;

pub fn cover_cache_control_value() -> String {
    format!("private, max-age={COVER_CACHE_MAX_AGE_SECS}")
}

pub async fn cover_etag(path: &Path, thumb: bool) -> AppResult<String> {
    let meta = tokio::fs::metadata(path).await?;
    let modified = meta
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let variant = if thumb { "thumb" } else { "full" };
    Ok(format!("\"{variant}-{modified}-{}\"", meta.len()))
}

pub fn if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .map(str::trim)
                .any(|candidate| candidate == etag || candidate == "*")
        })
}

pub fn not_modified_response(etag: &str, mime: &str) -> Response {
    Response::builder()
        .status(StatusCode::NOT_MODIFIED)
        .header(header::CACHE_CONTROL, cover_cache_control_value())
        .header(header::ETAG, etag)
        .header(header::CONTENT_TYPE, mime)
        .body(axum::body::Body::empty())
        .unwrap()
}
