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

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(v: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::IF_NONE_MATCH, v.parse().unwrap());
        h
    }

    #[test]
    fn if_none_match_handles_lists_and_wildcard() {
        let etag = "\"full-1-2\"";
        assert!(if_none_match(&headers(etag), etag));
        assert!(if_none_match(&headers("\"a\", \"full-1-2\""), etag));
        assert!(if_none_match(&headers("*"), etag));
        assert!(!if_none_match(&headers("\"thumb-1-2\""), etag));
        assert!(!if_none_match(&HeaderMap::new(), etag));
    }

    #[tokio::test]
    async fn etag_differs_between_full_and_thumb() {
        let path = std::env::temp_dir().join(format!("shelfie-etag-{}", uuid::Uuid::new_v4()));
        std::fs::write(&path, b"12345").unwrap();
        let full = cover_etag(&path, false).await.unwrap();
        let thumb = cover_etag(&path, true).await.unwrap();
        assert!(full.starts_with("\"full-") && full.ends_with("-5\""));
        assert_ne!(full, thumb);
        assert!(cover_etag(&path.with_extension("missing"), false)
            .await
            .is_err());
        let _ = std::fs::remove_file(path);
    }
}
