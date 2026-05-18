use crate::api::handlers;
use crate::domain::book::{BookCard, BookDetail, UpdateBookRequest, UpdateProgressRequest};
use crate::domain::home::HomeResponse;
use crate::domain::category::{CategoryDto, CreateCategoryRequest};
use crate::domain::library::{
    AddMemberRequest, CreateLibraryRequest, LibraryDto, LibraryMemberDto, PermissionFlags,
    UpdateMemberPermissionsRequest, UserLibraryMembershipDto,
};
use crate::domain::koreader::{
    KoreaderDocumentLink, KoreaderProgressRow, KosyncAuthResponse, KosyncProgressResponse,
    KosyncUpdateRequest, KosyncUpdateResponse, SetDocumentLinkRequest,
};
use crate::domain::sync::{RefreshDiff, RefreshJobResponse};
use crate::domain::user::UserDto;
use crate::error::ApiErrorBody;
use crate::infra::{BookMetadata, ReadingProgress};
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::health::health,
        handlers::auth::login,
        handlers::auth::register,
        handlers::home::get_home,
        handlers::books::list_books,
        handlers::books::get_book,
        handlers::books::upload_book,
        handlers::books::update_book,
        handlers::books::update_progress,
        handlers::books::delete_book,
        handlers::books::update_cover,
        handlers::books::download_book,
        handlers::books::get_cover,
        handlers::search::search,
        handlers::libraries::list_libraries,
        handlers::libraries::create_library,
        handlers::libraries::delete_library,
        handlers::libraries::list_members,
        handlers::libraries::add_member,
        handlers::libraries::update_permissions,
        handlers::libraries::remove_member,
        handlers::categories::list_categories,
        handlers::categories::create_category,
        handlers::categories::delete_category,
        handlers::sync::refresh_library,
        handlers::sync::get_refresh_job,
        handlers::users::list_users,
        handlers::users::list_user_memberships,
        handlers::users::update_username,
        handlers::users::change_password,
        handlers::users::delete_user,
        handlers::koreader::kosync_healthcheck,
        handlers::koreader::kosync_auth_user,
        handlers::koreader::kosync_update_progress,
        handlers::koreader::kosync_get_progress,
        handlers::koreader::list_koreader_progress,
        handlers::koreader::list_koreader_links,
        handlers::koreader::set_koreader_link,
        handlers::koreader::delete_koreader_link,
    ),
    components(schemas(
        handlers::health::HealthResponse,
        handlers::auth::LoginRequest,
        handlers::auth::LoginResponse,
        handlers::auth::UserInfo,
        handlers::auth::RegisterRequest,
        HomeResponse,
        BookCard,
        BookDetail,
        BookMetadata,
        ReadingProgress,
        UpdateBookRequest,
        UpdateProgressRequest,
        LibraryDto,
        LibraryMemberDto,
        PermissionFlags,
        CreateLibraryRequest,
        CategoryDto,
        CreateCategoryRequest,
        AddMemberRequest,
        UpdateMemberPermissionsRequest,
        UserLibraryMembershipDto,
        handlers::users::UpdateUsernameRequest,
        handlers::users::ChangePasswordRequest,
        UserDto,
        RefreshJobResponse,
        RefreshDiff,
        KosyncAuthResponse,
        KosyncProgressResponse,
        KosyncUpdateRequest,
        KosyncUpdateResponse,
        KoreaderProgressRow,
        KoreaderDocumentLink,
        SetDocumentLinkRequest,
        ApiErrorBody,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "Health", description = "Health check"),
        (name = "Auth", description = "Authentication"),
        (name = "Home", description = "Homepage feeds"),
        (name = "Books", description = "Book management"),
        (name = "Search", description = "Global search"),
        (name = "Libraries", description = "Library management"),
        (name = "Categories", description = "Book categories per library"),
        (name = "Sync", description = "Filesystem sync"),
        (name = "Users", description = "User administration"),
        (name = "Covers", description = "Cover images"),
        (name = "KOReader", description = "KOReader progress sync (KOSync protocol) and admin"),
    ),
    info(
        title = "Shelfie API",
        version = "0.1.0",
        description = "图书管理 API。Metadata 写路径：PATCH 更新 DB 后异步写回 metadata.json；refresh 默认以 FS 为准。"
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}
