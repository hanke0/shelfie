use crate::api::middleware::auth::AuthContext;
use crate::domain::library::{
    self, AddMemberRequest, CreateLibraryRequest, LibraryDto, LibraryMemberDto,
    UpdateMemberPermissionsRequest,
};
use crate::error::AppResult;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use uuid::Uuid;

#[utoipa::path(get, path = "/libraries", tag = "Libraries", responses((status = 200, body = [LibraryDto])), security(("bearer_auth" = [])))]
pub async fn list_libraries(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
) -> AppResult<Json<Vec<LibraryDto>>> {
    Ok(Json(library::list_libraries(&state.db, &user).await?))
}

#[utoipa::path(post, path = "/libraries", tag = "Libraries", request_body = CreateLibraryRequest, responses((status = 201, body = LibraryDto)), security(("bearer_auth" = [])))]
pub async fn create_library(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Json(req): Json<CreateLibraryRequest>,
) -> AppResult<(axum::http::StatusCode, Json<LibraryDto>)> {
    let lib = library::create_library(&state, &user, req).await?;
    Ok((axum::http::StatusCode::CREATED, Json(lib)))
}

#[utoipa::path(get, path = "/libraries/{id}/members", tag = "Libraries", params(("id" = Uuid, Path)), responses((status = 200, body = [LibraryMemberDto])), security(("bearer_auth" = [])))]
pub async fn list_members(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<LibraryMemberDto>>> {
    let perm = library::resolve_permission(&state.db, &user, &id).await?;
    library::require_view(&perm)?;
    Ok(Json(library::list_members(&state.db, &id).await?))
}

#[utoipa::path(post, path = "/libraries/{id}/members", tag = "Libraries", params(("id" = Uuid, Path)), request_body = AddMemberRequest, responses((status = 204)), security(("bearer_auth" = [])))]
pub async fn add_member(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> AppResult<axum::http::StatusCode> {
    let perm = library::resolve_permission(&state.db, &user, &id).await?;
    if user.role != "system_admin" && perm.can_edit == false {
        return Err(crate::error::AppError::Forbidden("Cannot manage members".into()));
    }
    library::add_member(&state.db, &id, req).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(patch, path = "/libraries/{id}/members/{user_id}/permissions", tag = "Libraries", params(("id" = Uuid, Path), ("user_id" = String, Path)), request_body = UpdateMemberPermissionsRequest, responses((status = 204)), security(("bearer_auth" = [])))]
pub async fn update_permissions(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path((id, user_id)): Path<(Uuid, String)>,
    Json(req): Json<UpdateMemberPermissionsRequest>,
) -> AppResult<axum::http::StatusCode> {
    let perm = library::resolve_permission(&state.db, &user, &id).await?;
    if user.role != "system_admin" && !perm.can_edit {
        return Err(crate::error::AppError::Forbidden("Cannot manage members".into()));
    }
    library::update_member_permissions(&state.db, &id, &user_id, req).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
