use crate::api::middleware::auth::AuthContext;
use crate::domain::home;
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
pub struct HomeQuery {
    /// 当前图书馆 ID，仅返回该馆数据
    pub library_id: Option<Uuid>,
    #[param(default = 12)]
    pub limit: Option<i64>,
    pub seed: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/home",
    tag = "Home",
    params(HomeQuery),
    responses((status = 200, body = home::HomeResponse)),
    security(("bearer_auth" = []))
)]
pub async fn get_home(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Query(q): Query<HomeQuery>,
) -> AppResult<Json<home::HomeResponse>> {
    let limit = q.limit.unwrap_or(12);
    Ok(Json(
        home::get_home(&state, &user, q.library_id, limit, q.seed).await?,
    ))
}
