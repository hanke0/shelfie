use crate::api::middleware::auth::AuthContext;
use crate::domain::book::{self, BookCard};
use crate::error::AppResult;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Deserialize, IntoParams)]
pub struct SearchQuery {
    pub q: String,
    pub library_id: Option<Uuid>,
    #[param(default = 20)]
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/search",
    tag = "Search",
    params(SearchQuery),
    responses((status = 200, body = [BookCard])),
    security(("bearer_auth" = []))
)]
pub async fn search(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Query(q): Query<SearchQuery>,
) -> AppResult<Json<Vec<BookCard>>> {
    let limit = q.limit.unwrap_or(20);
    Ok(Json(
        book::search_books(&state.db, &user, &q.q, q.library_id, limit).await?,
    ))
}
