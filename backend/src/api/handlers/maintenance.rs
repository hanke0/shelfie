use crate::api::middleware::auth::AuthContext;
use crate::domain::maintenance::{self, MaintenanceToolDto, StartTaskRequest};
use crate::domain::tasks::TaskDto;
use crate::error::AppResult;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Deserialize, IntoParams)]
pub struct ListTasksQuery {
    /// 仅返回运行中的任务
    #[param(default = false)]
    pub active_only: Option<bool>,
}

#[utoipa::path(get, path = "/maintenance/tools", tag = "Maintenance", responses((status = 200, body = Vec<MaintenanceToolDto>)), security(("bearer_auth" = [])))]
pub async fn list_maintenance_tools() -> Json<Vec<MaintenanceToolDto>> {
    Json(maintenance::list_tools())
}

#[utoipa::path(get, path = "/maintenance/tasks", tag = "Maintenance", params(ListTasksQuery), responses((status = 200, body = Vec<TaskDto>)), security(("bearer_auth" = [])))]
pub async fn list_maintenance_tasks(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Query(q): Query<ListTasksQuery>,
) -> AppResult<Json<Vec<TaskDto>>> {
    Ok(Json(
        maintenance::list_tasks(&state, &user, q.active_only.unwrap_or(false)).await?,
    ))
}

#[utoipa::path(post, path = "/maintenance/tasks", tag = "Maintenance", request_body = StartTaskRequest, responses((status = 202, body = TaskDto), (status = 409, description = "同一图书馆已有维护任务在运行")), security(("bearer_auth" = [])))]
pub async fn start_maintenance_task(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Json(req): Json<StartTaskRequest>,
) -> AppResult<(StatusCode, Json<TaskDto>)> {
    let task = maintenance::start_task(&state, &user, req).await?;
    Ok((StatusCode::ACCEPTED, Json(task)))
}

#[utoipa::path(get, path = "/maintenance/tasks/{id}", tag = "Maintenance", params(("id" = Uuid, Path)), responses((status = 200, body = TaskDto)), security(("bearer_auth" = [])))]
pub async fn get_maintenance_task(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<TaskDto>> {
    Ok(Json(maintenance::get_task(&state, &user, &id).await?))
}

#[utoipa::path(post, path = "/maintenance/tasks/{id}/cancel", tag = "Maintenance", params(("id" = Uuid, Path)), responses((status = 200, body = TaskDto), (status = 409, description = "任务已结束")), security(("bearer_auth" = [])))]
pub async fn cancel_maintenance_task(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<TaskDto>> {
    Ok(Json(maintenance::cancel_task(&state, &user, &id).await?))
}
