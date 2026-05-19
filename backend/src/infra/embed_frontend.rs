//! 编译期嵌入 `frontend/dist`，由后端统一提供静态资源（SPA fallback）。

use axum::{
    http::{header, StatusCode, Uri},
    response::IntoResponse,
    routing::get,
    Router,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../frontend/dist/"]
struct FrontendAssets;

fn asset_path(uri_path: &str) -> String {
    let path = uri_path.trim_start_matches('/');
    if path.is_empty() {
        return "index.html".into();
    }
    path.to_string()
}

fn response_for_path(path: &str) -> Option<(StatusCode, [(header::HeaderName, String); 1], Vec<u8>)> {
    let file = FrontendAssets::get(path)?;
    let mime = mime_guess::from_path(path)
        .first()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "application/octet-stream".into());
    Some((
        StatusCode::OK,
        [(header::CONTENT_TYPE, mime)],
        file.data.into_owned(),
    ))
}

async fn serve_embedded(uri: Uri) -> impl IntoResponse {
    let path = asset_path(uri.path());

    if let Some(resp) = response_for_path(&path) {
        return resp.into_response();
    }

    if path != "index.html" {
        if let Some(resp) = response_for_path("index.html") {
            return resp.into_response();
        }
    }

    StatusCode::NOT_FOUND.into_response()
}

/// 未匹配路由时提供前端静态资源；API 等已注册路由不受影响。
pub fn merge(router: Router) -> Router {
    router.fallback(get(serve_embedded))
}
