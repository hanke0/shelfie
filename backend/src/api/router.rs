use crate::api::handlers;
use crate::api::middleware::auth::require_auth;
use crate::api::middleware::kosync_auth::require_kosync_auth;
use crate::api::middleware::opds_auth::require_opds_auth;
use crate::openapi::ApiDoc;
use crate::state::AppState;
use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, patch, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{DefaultMakeSpan, DefaultOnFailure, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn build_router(state: AppState) -> Router {
    let kosync_public = Router::new()
        .route("/healthcheck", get(handlers::koreader::kosync_healthcheck))
        .route("/users/auth", get(handlers::koreader::kosync_auth_user));

    let kosync_protected = Router::new()
        .route(
            "/syncs/progress",
            put(handlers::koreader::kosync_update_progress),
        )
        .route(
            "/syncs/progress/{document}",
            get(handlers::koreader::kosync_get_progress),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_kosync_auth,
        ));

    let opds = Router::new()
        .route("/opds", get(handlers::opds::opds_root))
        .route(
            "/opds/libraries/{library_id}",
            get(handlers::opds::opds_library),
        )
        .route(
            "/opds/libraries/{library_id}/books",
            get(handlers::opds::opds_library_books),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_opds_auth,
        ));

    let public = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/auth/login", post(handlers::auth::login))
        .merge(kosync_public)
        .merge(kosync_protected);

    let protected = Router::new()
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/me", get(handlers::auth::me))
        .route("/home", get(handlers::home::get_home))
        .route(
            "/books",
            get(handlers::books::list_books).post(handlers::books::upload_book),
        )
        .route(
            "/books/{id}",
            get(handlers::books::get_book)
                .patch(handlers::books::update_book)
                .delete(handlers::books::delete_book),
        )
        .route("/books/{id}/download", get(handlers::books::download_book))
        .route(
            "/books/{id}/progress",
            patch(handlers::books::update_progress),
        )
        .route("/books/{id}/cover", put(handlers::books::update_cover))
        .route("/search", get(handlers::search::search))
        .route(
            "/libraries",
            get(handlers::libraries::list_libraries).post(handlers::libraries::create_library),
        )
        .route(
            "/libraries/{id}",
            delete(handlers::libraries::delete_library),
        )
        .route(
            "/libraries/{id}/categories",
            get(handlers::categories::list_categories).post(handlers::categories::create_category),
        )
        .route(
            "/libraries/{id}/categories/{name}",
            delete(handlers::categories::delete_category),
        )
        .route(
            "/libraries/{id}/members",
            get(handlers::libraries::list_members).post(handlers::libraries::add_member),
        )
        .route(
            "/libraries/{id}/members/{user_id}/permissions",
            patch(handlers::libraries::update_permissions),
        )
        .route(
            "/libraries/{id}/members/{user_id}",
            delete(handlers::libraries::remove_member),
        )
        .route(
            "/libraries/{id}/refresh",
            post(handlers::sync::refresh_library),
        )
        .route(
            "/libraries/{id}/refresh/{job_id}",
            get(handlers::sync::get_refresh_job),
        )
        .route("/users", get(handlers::users::list_users))
        .route(
            "/users/me/reading-history",
            get(handlers::users::list_my_reading_history),
        )
        .route(
            "/users/{user_id}/memberships",
            get(handlers::users::list_user_memberships),
        )
        .route(
            "/users/{user_id}/username",
            patch(handlers::users::update_username),
        )
        .route(
            "/users/{user_id}/password",
            patch(handlers::users::change_password),
        )
        .route("/users/{user_id}", delete(handlers::users::delete_user))
        .route(
            "/koreader/progress",
            get(handlers::koreader::list_koreader_progress),
        )
        .route(
            "/koreader/links",
            get(handlers::koreader::list_koreader_links).put(handlers::koreader::set_koreader_link),
        )
        .route(
            "/koreader/links/{document}",
            axum::routing::delete(handlers::koreader::delete_koreader_link),
        )
        .route("/assets/covers/{id}", get(handlers::books::get_cover))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let api = Router::new().merge(public).merge(opds).merge(protected);

    /// 图书上传需支持较大 PDF/EPUB（默认 2MB 会导致 multipart 解析失败）
    const UPLOAD_LIMIT: usize = 512 * 1024 * 1024;

    let router = Router::new()
        .nest("/api/v1", api)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(DefaultBodyLimit::max(UPLOAD_LIMIT))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(
                    DefaultMakeSpan::new()
                        .level(Level::INFO)
                        .include_headers(false),
                )
                .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
        )
        .with_state(state);

    #[cfg(feature = "embed-frontend")]
    {
        return crate::infra::embed_frontend::merge(router);
    }

    #[cfg(not(feature = "embed-frontend"))]
    router
}
