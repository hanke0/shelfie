use crate::api::handlers;
use crate::api::middleware::auth::require_auth;
use crate::openapi::ApiDoc;
use crate::state::AppState;
use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, patch, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn build_router(state: AppState) -> Router {
    let public = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/auth/login", post(handlers::auth::login));

    let protected = Router::new()
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/me", get(handlers::auth::me))
        .route("/home", get(handlers::home::get_home))
        .route("/books", get(handlers::books::list_books).post(handlers::books::upload_book))
        .route(
            "/books/{id}",
            get(handlers::books::get_book)
                .patch(handlers::books::update_book)
                .delete(handlers::books::delete_book),
        )
        .route("/books/{id}/download", get(handlers::books::download_book))
        .route("/books/{id}/progress", patch(handlers::books::update_progress))
        .route("/books/{id}/cover", put(handlers::books::update_cover))
        .route("/search", get(handlers::search::search))
        .route("/libraries", get(handlers::libraries::list_libraries).post(handlers::libraries::create_library))
        .route("/libraries/{id}/members", get(handlers::libraries::list_members).post(handlers::libraries::add_member))
        .route(
            "/libraries/{id}/members/{user_id}/permissions",
            patch(handlers::libraries::update_permissions),
        )
        .route("/libraries/{id}/refresh", post(handlers::sync::refresh_library))
        .route(
            "/libraries/{id}/refresh/{job_id}",
            get(handlers::sync::get_refresh_job),
        )
        .route("/users", get(handlers::users::list_users))
        .route("/assets/covers/{id}", get(handlers::books::get_cover))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let api = Router::new().merge(public).merge(protected);

    /// 图书上传需支持较大 PDF/EPUB（默认 2MB 会导致 multipart 解析失败）
    const UPLOAD_LIMIT: usize = 512 * 1024 * 1024;

    Router::new()
        .nest("/api/v1", api)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(DefaultBodyLimit::max(UPLOAD_LIMIT))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
