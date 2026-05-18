use crate::api::middleware::auth::AuthContext;
use crate::domain::category::{self, CreateCategoryRequest, CategoryDto};
use crate::error::AppResult;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/libraries/{id}/categories",
    tag = "Categories",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = [CategoryDto])),
    security(("bearer_auth" = []))
)]
pub async fn list_categories(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<CategoryDto>>> {
    Ok(Json(category::list_categories(&state, &user, &id).await?))
}

#[utoipa::path(
    post,
    path = "/libraries/{id}/categories",
    tag = "Categories",
    params(("id" = Uuid, Path)),
    request_body = CreateCategoryRequest,
    responses((status = 201, body = CategoryDto)),
    security(("bearer_auth" = []))
)]
pub async fn create_category(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateCategoryRequest>,
) -> AppResult<(axum::http::StatusCode, Json<CategoryDto>)> {
    let cat = category::create_category(&state, &user, &id, req).await?;
    Ok((axum::http::StatusCode::CREATED, Json(cat)))
}

#[utoipa::path(
    delete,
    path = "/libraries/{id}/categories/{name}",
    tag = "Categories",
    params(("id" = Uuid, Path), ("name" = String, Path)),
    responses((status = 204), (status = 409)),
    security(("bearer_auth" = []))
)]
pub async fn delete_category(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path((id, name)): Path<(Uuid, String)>,
) -> AppResult<axum::http::StatusCode> {
    category::delete_category(&state, &user, &id, &name).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
