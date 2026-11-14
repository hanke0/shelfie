use crate::api::handlers;
use crate::domain::annotations::{
    BookAnnotationDto, BookAnnotationsResponse, BookBookmarkDto, BookReadingSummary,
};
use crate::domain::book::{
    BatchMoveCategoryRequest, BatchMoveCategoryResponse, BookCard, BookDetail, UpdateBookRequest,
    UpdateProgressRequest,
};
use crate::domain::bookorbit;
use crate::domain::category::{CategoryDto, CreateCategoryRequest};
use crate::domain::duplicates::{DuplicateGroup, DuplicateMatchKind, FindDuplicatesResponse};
use crate::domain::home::HomeResponse;
use crate::domain::koreader::{
    KoreaderDocumentLink, KoreaderProgressRow, KosyncAuthResponse, KosyncProgressResponse,
    KosyncUpdateRequest, KosyncUpdateResponse, SetDocumentLinkRequest,
};
use crate::domain::library::{
    AddMemberRequest, CreateLibraryRequest, LibraryDto, LibraryMemberDto, PermissionFlags,
    UpdateMemberPermissionsRequest, UserLibraryMembershipDto,
};
use crate::domain::maintenance::{
    MaintenanceTool, MaintenanceToolDto, StartTaskRequest, ToolParamDto, ToolParamKind,
    ToolPermission,
};
use crate::domain::reading_history::ReadingHistoryEntry;
use crate::domain::sync::{
    LibraryScanReport, MaintenanceActionReport, MissingFileBook, UntrackedBookFile,
};
use crate::domain::tasks::{TaskDto, TaskProgress, TaskStatus};
use crate::domain::user::UserDto;
use crate::error::ApiErrorBody;
use crate::infra::{BookMetadata, MetadataAnnotation, MetadataBookmark, ReadingProgress};
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
        handlers::books::batch_move_category,
        handlers::books::update_book,
        handlers::books::update_progress,
        handlers::books::list_book_annotations,
        handlers::books::get_reading_summary,
        handlers::books::delete_book,
        handlers::books::update_cover,
        handlers::books::download_book,
        handlers::books::get_cover,
        handlers::search::search,
        handlers::libraries::list_libraries,
        handlers::libraries::create_library,
        handlers::libraries::delete_library,
        handlers::libraries::find_duplicate_books,
        handlers::libraries::list_members,
        handlers::libraries::add_member,
        handlers::libraries::update_permissions,
        handlers::libraries::remove_member,
        handlers::categories::list_categories,
        handlers::categories::create_category,
        handlers::categories::delete_category,
        handlers::maintenance::list_maintenance_tools,
        handlers::maintenance::list_maintenance_tasks,
        handlers::maintenance::start_maintenance_task,
        handlers::maintenance::get_maintenance_task,
        handlers::maintenance::cancel_maintenance_task,
        handlers::users::list_users,
        handlers::users::list_my_reading_history,
        handlers::users::list_user_memberships,
        handlers::users::update_username,
        handlers::users::change_password,
        handlers::users::delete_user,
        handlers::koreader::kosync_healthcheck,
        handlers::koreader::kosync_auth_user,
        handlers::koreader::kosync_update_progress,
        handlers::koreader::kosync_get_progress,
        handlers::bookorbit::plugin_version,
        handlers::bookorbit::match_check,
        handlers::bookorbit::exchange_annotations,
        handlers::bookorbit::ack_annotations,
        handlers::bookorbit::exchange_bookmarks,
        handlers::bookorbit::ack_bookmarks,
        handlers::bookorbit::upload_page_stats,
        handlers::bookorbit::upload_book_states,
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
        MetadataAnnotation,
        MetadataBookmark,
        ReadingProgress,
        BookAnnotationsResponse,
        BookAnnotationDto,
        BookBookmarkDto,
        BookReadingSummary,
        UpdateBookRequest,
        UpdateProgressRequest,
        BatchMoveCategoryRequest,
        BatchMoveCategoryResponse,
        crate::domain::book::BatchMoveCategoryFailure,
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
        ReadingHistoryEntry,
        UserDto,
        MaintenanceTool,
        MaintenanceToolDto,
        ToolParamDto,
        ToolParamKind,
        ToolPermission,
        StartTaskRequest,
        TaskDto,
        TaskProgress,
        TaskStatus,
        LibraryScanReport,
        MissingFileBook,
        UntrackedBookFile,
        MaintenanceActionReport,
        FindDuplicatesResponse,
        DuplicateGroup,
        DuplicateMatchKind,
        KosyncAuthResponse,
        KosyncProgressResponse,
        KosyncUpdateRequest,
        KosyncUpdateResponse,
        bookorbit::PluginVersionInfo,
        bookorbit::PluginMatchCheckRequest,
        bookorbit::PluginMatch,
        bookorbit::PluginMatchCheckResponse,
        bookorbit::ExchangeKey,
        bookorbit::KoAnnotation,
        bookorbit::AnnotationExchangeBook,
        bookorbit::AnnotationExchangeRequest,
        bookorbit::ExchangeAddEntry,
        bookorbit::ExchangeEditEntry,
        bookorbit::ExchangeDeleteEntry,
        bookorbit::AnnotationToApply,
        bookorbit::AnnotationExchangeResult,
        bookorbit::AnnotationExchangeResponse,
        bookorbit::AnnotationAckApplied,
        bookorbit::AckDeleted,
        bookorbit::AnnotationAckBook,
        bookorbit::AnnotationAckRequest,
        bookorbit::AckResponse,
        bookorbit::KoBookmark,
        bookorbit::BookmarkExchangeBook,
        bookorbit::BookmarkExchangeRequest,
        bookorbit::BookmarkAddEntry,
        bookorbit::BookmarkToApply,
        bookorbit::BookmarkExchangeResult,
        bookorbit::BookmarkExchangeResponse,
        bookorbit::BookmarkAckApplied,
        bookorbit::BookmarkAckBook,
        bookorbit::BookmarkAckRequest,
        bookorbit::PageStatsEvent,
        bookorbit::PageStatsBook,
        bookorbit::PageStatsRequest,
        bookorbit::PageStatsResult,
        bookorbit::PageStatsResponse,
        bookorbit::BookStateEntry,
        bookorbit::BookStatesRequest,
        bookorbit::BookStateResult,
        bookorbit::BookStatesResponse,
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
        (name = "Maintenance", description = "System maintenance tools and background tasks"),
        (name = "Users", description = "User administration"),
        (name = "Covers", description = "Cover images"),
        (name = "KOReader", description = "KOReader progress sync (KOSync protocol, also mounted under /koreader for BookOrbit clients), BookOrbit plugin sync (annotations, bookmarks, stats, status) and admin"),
    ),
    info(
        title = "Shelfie API",
        version = "0.5.2",
        description = "图书管理 API。Metadata 写路径：PATCH 更新 DB 后异步写回 metadata.json；目录与数据库的对比与修复通过 /maintenance 后台任务执行。"
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
