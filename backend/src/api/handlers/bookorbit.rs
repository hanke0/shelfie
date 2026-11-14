use crate::api::middleware::kosync_auth::KosyncAuth;
use crate::domain::bookorbit::{
    self, AckResponse, AnnotationAckRequest, AnnotationExchangeRequest, AnnotationExchangeResponse,
    BookStatesRequest, BookStatesResponse, BookmarkAckRequest, BookmarkExchangeRequest,
    BookmarkExchangeResponse, PageStatsRequest, PageStatsResponse, PluginMatchCheckRequest,
    PluginMatchCheckResponse, PluginVersionInfo,
};
use crate::error::AppResult;
use crate::state::AppState;
use axum::{extract::State, Extension, Json};

#[utoipa::path(
    get,
    path = "/koreader/plugin/version",
    tag = "KOReader",
    responses((status = 200, body = PluginVersionInfo), (status = 401))
)]
pub async fn plugin_version(
    Extension(KosyncAuth(_user)): Extension<KosyncAuth>,
) -> Json<PluginVersionInfo> {
    Json(bookorbit::plugin_version())
}

#[utoipa::path(
    post,
    path = "/koreader/plugin/match-check",
    tag = "KOReader",
    request_body = PluginMatchCheckRequest,
    responses((status = 200, body = PluginMatchCheckResponse), (status = 400), (status = 401))
)]
pub async fn match_check(
    State(state): State<AppState>,
    Extension(KosyncAuth(user)): Extension<KosyncAuth>,
    Json(req): Json<PluginMatchCheckRequest>,
) -> AppResult<Json<PluginMatchCheckResponse>> {
    Ok(Json(
        bookorbit::plugin_match_check(&state.db, &user, req).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/koreader/plugin/annotations/exchange",
    tag = "KOReader",
    request_body = AnnotationExchangeRequest,
    responses((status = 200, body = AnnotationExchangeResponse), (status = 400), (status = 401))
)]
pub async fn exchange_annotations(
    State(state): State<AppState>,
    Extension(KosyncAuth(user)): Extension<KosyncAuth>,
    Json(req): Json<AnnotationExchangeRequest>,
) -> AppResult<Json<AnnotationExchangeResponse>> {
    Ok(Json(
        bookorbit::exchange_annotations(&state.db, &user, req).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/koreader/plugin/annotations/exchange-ack",
    tag = "KOReader",
    request_body = AnnotationAckRequest,
    responses((status = 200, body = AckResponse), (status = 400), (status = 401))
)]
pub async fn ack_annotations(
    State(state): State<AppState>,
    Extension(KosyncAuth(user)): Extension<KosyncAuth>,
    Json(req): Json<AnnotationAckRequest>,
) -> AppResult<Json<AckResponse>> {
    Ok(Json(
        bookorbit::ack_annotations(&state.db, &user, req).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/koreader/plugin/bookmarks/exchange",
    tag = "KOReader",
    request_body = BookmarkExchangeRequest,
    responses((status = 200, body = BookmarkExchangeResponse), (status = 400), (status = 401))
)]
pub async fn exchange_bookmarks(
    State(state): State<AppState>,
    Extension(KosyncAuth(user)): Extension<KosyncAuth>,
    Json(req): Json<BookmarkExchangeRequest>,
) -> AppResult<Json<BookmarkExchangeResponse>> {
    Ok(Json(
        bookorbit::exchange_bookmarks(&state.db, &user, req).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/koreader/plugin/bookmarks/exchange-ack",
    tag = "KOReader",
    request_body = BookmarkAckRequest,
    responses((status = 200, body = AckResponse), (status = 400), (status = 401))
)]
pub async fn ack_bookmarks(
    State(state): State<AppState>,
    Extension(KosyncAuth(user)): Extension<KosyncAuth>,
    Json(req): Json<BookmarkAckRequest>,
) -> AppResult<Json<AckResponse>> {
    Ok(Json(bookorbit::ack_bookmarks(&state.db, &user, req).await?))
}

#[utoipa::path(
    post,
    path = "/koreader/plugin/page-stats",
    tag = "KOReader",
    request_body = PageStatsRequest,
    responses((status = 200, body = PageStatsResponse), (status = 400), (status = 401))
)]
pub async fn upload_page_stats(
    State(state): State<AppState>,
    Extension(KosyncAuth(user)): Extension<KosyncAuth>,
    Json(req): Json<PageStatsRequest>,
) -> AppResult<Json<PageStatsResponse>> {
    Ok(Json(
        bookorbit::upload_page_stats(&state.db, &user, req).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/koreader/plugin/book-states",
    tag = "KOReader",
    request_body = BookStatesRequest,
    responses((status = 200, body = BookStatesResponse), (status = 400), (status = 401))
)]
pub async fn upload_book_states(
    State(state): State<AppState>,
    Extension(KosyncAuth(user)): Extension<KosyncAuth>,
    Json(req): Json<BookStatesRequest>,
) -> AppResult<Json<BookStatesResponse>> {
    Ok(Json(
        bookorbit::upload_book_states(&state.db, &user, req).await?,
    ))
}
