use crate::api::middleware::auth::AuthContext;
use crate::domain::duplicates::FindDuplicatesResponse;
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

#[utoipa::path(delete, path = "/libraries/{id}", tag = "Libraries", params(("id" = Uuid, Path)), responses((status = 204), (status = 403), (status = 404), (status = 409)), security(("bearer_auth" = [])))]
pub async fn delete_library(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> AppResult<axum::http::StatusCode> {
    library::delete_library(&state, &user, &id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/libraries", tag = "Libraries", request_body = CreateLibraryRequest, responses((status = 201, body = LibraryDto)), security(("bearer_auth" = [])))]
pub async fn create_library(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Json(req): Json<CreateLibraryRequest>,
) -> AppResult<(axum::http::StatusCode, Json<LibraryDto>)> {
    crate::domain::user::require_system_admin(&user)?;
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
    Ok(Json(
        library::list_members_for_user(&state.db, &user, &id).await?,
    ))
}

#[utoipa::path(post, path = "/libraries/{id}/members", tag = "Libraries", params(("id" = Uuid, Path)), request_body = AddMemberRequest, responses((status = 204)), security(("bearer_auth" = [])))]
pub async fn add_member(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> AppResult<axum::http::StatusCode> {
    let perm = library::resolve_permission(&state.db, &user, &id).await?;
    library::require_view(&perm)?;
    if !library::can_manage_members(&state.db, &user, &id).await? {
        return Err(crate::error::AppError::Forbidden(
            "Cannot manage library members".into(),
        ));
    }
    library::add_member(&state.db, &id, &user, req).await?;
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
    library::require_view(&perm)?;
    if !library::can_edit_member_permissions(&user) {
        return Err(crate::error::AppError::Forbidden(
            "Only system admin can edit member permissions".into(),
        ));
    }
    library::update_member_permissions(&state.db, &id, &user_id, req).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(delete, path = "/libraries/{id}/members/{user_id}", tag = "Libraries", params(("id" = Uuid, Path), ("user_id" = String, Path)), responses((status = 204)), security(("bearer_auth" = [])))]
pub async fn remove_member(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path((id, user_id)): Path<(Uuid, String)>,
) -> AppResult<axum::http::StatusCode> {
    let perm = library::resolve_permission(&state.db, &user, &id).await?;
    library::require_view(&perm)?;
    library::remove_member(&state.db, &id, &user, &user_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/libraries/{id}/duplicates",
    tag = "Libraries",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = FindDuplicatesResponse)),
    security(("bearer_auth" = []))
)]
pub async fn find_duplicate_books(
    State(state): State<AppState>,
    Extension(AuthContext(user)): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<FindDuplicatesResponse>> {
    Ok(Json(
        crate::domain::duplicates::find_duplicates(&state, &user, &id).await?,
    ))
}
