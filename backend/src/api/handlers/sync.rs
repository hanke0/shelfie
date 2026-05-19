use crate::api::middleware::auth::AuthContext;
use crate::domain::sync::{self, RefreshJobResponse};
use crate::error::AppResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Deserialize, IntoParams)]
pub struct RefreshQuery {
    #[param(default = false)]
    pub prefer_db: Option<bool>,
}

#[utoipa::path(post, path = "/libraries/{id}/refresh", tag = "Sync", params(("id" = Uuid, Path), RefreshQuery), responses((status = 200, body = RefreshJobResponse)), security(("bearer_auth" = [])))]
pub async fn refresh_library(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Query(q): Query<RefreshQuery>,
) -> AppResult<Json<RefreshJobResponse>> {
    let prefer_db = q.prefer_db.unwrap_or(false);
    Ok(Json(
        sync::start_refresh(&state, &user, &id, prefer_db).await?,
    ))
}

#[utoipa::path(get, path = "/libraries/{id}/refresh/{job_id}", tag = "Sync", params(("id" = Uuid, Path), ("job_id" = Uuid, Path)), responses((status = 200, body = RefreshJobResponse)), security(("bearer_auth" = [])))]
pub async fn get_refresh_job(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path((id, job_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<RefreshJobResponse>> {
    Ok(Json(
        sync::get_refresh_job(&state, &user, &id, &job_id).await?,
    ))
}
