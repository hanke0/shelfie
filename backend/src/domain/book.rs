use crate::domain::auth::AuthUser;
use crate::domain::library;
use crate::error::{AppError, AppResult};
use crate::infra::{fs, hash, thumbnail, BookMetadata, ReadingProgress};
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct BookCard {
    pub id: String,
    pub library_id: String,
    pub title: String,
    pub author: String,
    pub category: String,
    pub cover_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reading_percent: Option<f64>,
    pub last_read_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BookDetail {
    #[serde(flatten)]
    pub card: BookCard,
    pub metadata: BookMetadata,
    pub book_file_path: String,
    pub sync_status: String,
    pub uploaded_at: String,
    pub updated_at: String,
    pub permissions: crate::domain::library::PermissionFlags,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateBookRequest {
    pub metadata: BookMetadata,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProgressRequest {
    pub reading_progress: ReadingProgress,
}

struct BookRow {
    id: String,
    library_id: String,
    category: String,
    book_file_path: String,
    metadata_file_path: String,
    cover_path: String,
    metadata: String,
    sync_status: String,
    last_read_at: Option<String>,
    uploaded_at: String,
    updated_at: String,
}

pub(crate) type BookDbRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
);

fn book_row_from_db(r: BookDbRow) -> BookRow {
    BookRow {
        id: r.0,
        library_id: r.1,
        category: r.2,
        book_file_path: r.3,
        metadata_file_path: r.4,
        cover_path: r.5,
        metadata: r.6,
        sync_status: r.7,
        last_read_at: r.8,
        uploaded_at: r.9,
        updated_at: r.10,
    }
}

pub(crate) struct BookRowInternal {
    pub library_id: String,
    pub metadata: String,
    pub metadata_file_path: String,
}

pub(crate) async fn fetch_row_internal(
    db: &SqlitePool,
    book_id: &Uuid,
) -> AppResult<BookRowInternal> {
    let row = fetch_row(db, book_id).await?;
    Ok(BookRowInternal {
        library_id: row.library_id,
        metadata: row.metadata,
        metadata_file_path: row.metadata_file_path,
    })
}

async fn fetch_row(db: &SqlitePool, book_id: &Uuid) -> AppResult<BookRow> {
    let row: Option<BookDbRow> = sqlx::query_as(
        r#"
        SELECT id, library_id, category, book_file_path, metadata_file_path, cover_path,
               metadata, sync_status, last_read_at, uploaded_at, updated_at
        FROM books WHERE id = ?
        "#,
    )
    .bind(book_id.to_string())
    .fetch_optional(db)
    .await?;

    let r = row.ok_or_else(|| AppError::NotFound("Book not found".into()))?;
    Ok(book_row_from_db(r))
}

fn row_to_card(row: &BookRow) -> AppResult<BookCard> {
    let meta = BookMetadata::from_json(&row.metadata)?;
    Ok(BookCard {
        id: row.id.clone(),
        library_id: row.library_id.clone(),
        title: meta.title,
        author: meta.author,
        category: row.category.clone(),
        cover_url: format!("/api/v1/assets/covers/{}", row.id),
        reading_percent: meta.reading_progress.as_ref().and_then(|p| p.percent),
        last_read_at: row.last_read_at.clone(),
    })
}

pub async fn get_book(state: &AppState, user: &AuthUser, book_id: &Uuid) -> AppResult<BookDetail> {
    let row = fetch_row(&state.db, book_id).await?;
    let library_id =
        Uuid::parse_str(&row.library_id).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(&state.db, user, &library_id).await?;
    library::require_view(&perm)?;

    let meta = BookMetadata::from_json(&row.metadata)?;
    let card = row_to_card(&row)?;

    Ok(BookDetail {
        card,
        metadata: meta,
        book_file_path: row.book_file_path,
        sync_status: row.sync_status,
        uploaded_at: row.uploaded_at,
        updated_at: row.updated_at,
        permissions: library::PermissionFlags {
            can_view: perm.can_view,
            can_edit: perm.can_edit,
            can_delete: perm.can_delete,
        },
    })
}

#[derive(Debug, Clone)]
pub struct OpdsBookRow {
    pub id: String,
    pub title: String,
    pub author: String,
    pub category: String,
    pub book_file_path: String,
    pub updated_at: String,
    pub summary: Option<String>,
    pub language: Option<String>,
}

pub async fn list_books_for_opds(
    db: &SqlitePool,
    user: &AuthUser,
    library_id: Uuid,
    category: Option<&str>,
    limit: i64,
) -> AppResult<Vec<OpdsBookRow>> {
    let accessible = accessible_library_ids(db, user).await?;
    if !accessible.contains(&library_id.to_string()) {
        return Err(AppError::Forbidden("No access to library".into()));
    }

    let mut query = r#"
        SELECT id, category, book_file_path, metadata, updated_at
        FROM books
        WHERE library_id = ?
    "#
    .to_string();

    if category.is_some() {
        query.push_str(" AND category = ?");
    }
    query.push_str(" ORDER BY json_extract(metadata, '$.title') COLLATE NOCASE LIMIT ?");

    let mut q = sqlx::query_as::<_, (String, String, String, String, String)>(&query);
    q = q.bind(library_id.to_string());
    if let Some(cat) = category {
        q = q.bind(cat);
    }
    q = q.bind(limit);

    let rows = q.fetch_all(db).await?;
    rows.into_iter()
        .map(|(id, category, book_file_path, metadata, updated_at)| {
            let meta = BookMetadata::from_json(&metadata)?;
            Ok(OpdsBookRow {
                id,
                title: meta.title,
                author: meta.author,
                category,
                book_file_path,
                updated_at,
                summary: (!meta.notes.is_empty()).then_some(meta.notes),
                language: (!meta.language.is_empty()).then_some(meta.language),
            })
        })
        .collect()
}

pub async fn list_books(
    db: &SqlitePool,
    user: &AuthUser,
    library_id: Option<Uuid>,
    category: Option<&str>,
    sort: Option<&str>,
    limit: i64,
) -> AppResult<Vec<BookCard>> {
    let accessible = accessible_library_ids(db, user).await?;
    if accessible.is_empty() {
        return Ok(vec![]);
    }

    let placeholders = accessible.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let order = match sort {
        Some("recent") => "last_read_at DESC NULLS LAST",
        Some("new") => "uploaded_at DESC",
        _ => "uploaded_at DESC",
    };

    let mut query = format!(
        "SELECT id, library_id, category, book_file_path, metadata_file_path, cover_path, metadata, sync_status, last_read_at, uploaded_at, updated_at FROM books WHERE library_id IN ({placeholders})"
    );

    if let Some(lid) = library_id {
        if !accessible.contains(&lid.to_string()) {
            return Err(AppError::Forbidden("No access to library".into()));
        }
        query = format!("{query} AND library_id = ?");
    }

    if category.is_some() {
        query = format!("{query} AND category = ?");
    }

    query = format!("{query} ORDER BY {order} LIMIT ?");

    let mut q = sqlx::query_as::<_, BookDbRow>(&query);

    for id in &accessible {
        q = q.bind(id);
    }
    if let Some(lid) = library_id {
        q = q.bind(lid.to_string());
    }
    if let Some(cat) = category {
        q = q.bind(cat);
    }
    q = q.bind(limit);

    let rows = q.fetch_all(db).await?;
    rows.into_iter()
        .map(|r| row_to_card(&book_row_from_db(r)))
        .collect()
}

pub(crate) async fn accessible_library_ids(
    db: &SqlitePool,
    user: &AuthUser,
) -> AppResult<Vec<String>> {
    if user.role == "system_admin" {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM libraries")
            .fetch_all(db)
            .await?;
        return Ok(rows.into_iter().map(|r| r.0).collect());
    }
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT library_id FROM library_members WHERE user_id = ? AND can_view = 1")
            .bind(user.id.to_string())
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

pub struct UploadBookInput {
    pub library_id: Uuid,
    pub category: String,
    pub book_bytes: Vec<u8>,
    pub book_ext: String,
    pub cover_bytes: Option<Vec<u8>>,
    pub cover_ext: Option<String>,
    pub metadata: BookMetadata,
}

pub async fn upload_book(
    state: &AppState,
    user: &AuthUser,
    input: UploadBookInput,
) -> AppResult<BookDetail> {
    let UploadBookInput {
        library_id,
        category,
        book_bytes,
        book_ext,
        cover_bytes,
        cover_ext,
        mut metadata,
    } = input;
    fs::validate_book_extension(&book_ext)?;
    if let Some(ref ext) = cover_ext {
        fs::validate_cover_extension(ext)?;
    }

    let perm = library::resolve_permission(&state.db, user, &library_id).await?;
    library::require_edit(&perm)?;

    let book_id = Uuid::new_v4();
    let lib_root = library::get_library_root(&state.db, &library_id).await?;
    let category = crate::domain::category::canonical_category_name(&category)?;
    crate::domain::category::require_category_exists(&state.db, &library_id, &lib_root, &category)
        .await?;
    metadata.category = category.clone();
    if metadata.title.is_empty() {
        metadata.title = format!("Untitled {}", &book_id.to_string()[..8]);
    }
    metadata.normalize_fields()?;
    fs::validate_book_path_fields(&metadata.title, &metadata.author, &category)?;

    let dir = fs::category_dir(&lib_root, &category)?;
    metadata.file_md5 = Some(hash::md5_hex(&book_bytes));

    let base = fs::ensure_unique_base(&dir, &fs::book_base_name(&metadata.title, &metadata.author));
    let book_path = fs::write_book_file(&dir, &base, &book_ext, &book_bytes).await?;
    let cover_path = match (cover_bytes.as_ref(), cover_ext.as_ref()) {
        (Some(bytes), Some(ext)) if !bytes.is_empty() => {
            fs::write_cover_file(&dir, &base, ext, bytes).await?
        }
        (None, None) => PathBuf::new(),
        _ => return Err(AppError::BadRequest("Invalid cover upload".into())),
    };
    let metadata_path = fs::write_metadata_file(&dir, &base, &metadata).await?;
    let cover_path_db = cover_path.to_string_lossy().into_owned();

    let metadata_json = metadata.to_json()?;
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO books (id, library_id, category, book_file_path, metadata_file_path, cover_path, metadata, uploaded_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(book_id.to_string())
    .bind(library_id.to_string())
    .bind(&category)
    .bind(book_path.to_string_lossy().as_ref())
    .bind(metadata_path.to_string_lossy().as_ref())
    .bind(&cover_path_db)
    .bind(&metadata_json)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    get_book(state, user, &book_id).await
}

pub async fn update_book(
    state: &AppState,
    user: &AuthUser,
    book_id: &Uuid,
    metadata: BookMetadata,
) -> AppResult<BookDetail> {
    let row = fetch_row(&state.db, book_id).await?;
    let library_id =
        Uuid::parse_str(&row.library_id).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(&state.db, user, &library_id).await?;
    library::require_edit(&perm)?;

    let mut metadata = metadata;
    metadata.normalize_fields()?;

    let lib_root = library::get_library_root(&state.db, &library_id).await?;
    let target_category = if metadata.category.is_empty() {
        row.category.clone()
    } else {
        crate::domain::category::canonical_category_name(&metadata.category)?
    };
    fs::validate_book_path_fields(&metadata.title, &metadata.author, &target_category)?;
    if target_category != row.category {
        crate::domain::category::require_category_exists(
            &state.db,
            &library_id,
            &lib_root,
            &target_category,
        )
        .await?;
    }
    let target_dir = fs::category_dir(&lib_root, &target_category)?;
    fs::ensure_dir(&target_dir).await?;

    let mut book_path = PathBuf::from(&row.book_file_path);
    let mut metadata_path = PathBuf::from(&row.metadata_file_path);
    let mut cover_path = PathBuf::from(&row.cover_path);
    let has_cover = fs::has_stored_cover(&cover_path);

    if book_path.parent() != Some(target_dir.as_path()) {
        let name = book_path
            .file_name()
            .ok_or_else(|| AppError::Internal("Invalid book path".into()))?;
        let new_book = target_dir.join(name);
        if new_book.exists() {
            tokio::fs::remove_file(&new_book).await.ok();
        }
        tokio::fs::rename(&book_path, &new_book).await?;
        book_path = new_book;

        let meta_name = metadata_path.file_name().unwrap();
        let new_meta = target_dir.join(meta_name);
        if new_meta.exists() {
            tokio::fs::remove_file(&new_meta).await.ok();
        }
        tokio::fs::rename(&metadata_path, &new_meta).await?;
        metadata_path = new_meta;

        if has_cover {
            let cover_name = cover_path
                .file_name()
                .ok_or_else(|| AppError::Internal("Invalid cover path".into()))?;
            let new_cover = target_dir.join(cover_name);
            if new_cover.exists() {
                tokio::fs::remove_file(&new_cover).await.ok();
            }
            tokio::fs::rename(&cover_path, &new_cover).await?;
            cover_path = new_cover;
        }
    }

    let (new_book, new_meta, new_cover) = fs::rename_book_assets(
        &target_dir,
        &metadata,
        &book_path,
        &metadata_path,
        &cover_path,
    )
    .await?;
    let base = fs::file_stem(&new_book).unwrap_or_default();
    fs::write_metadata_file(&target_dir, &base, &metadata).await?;

    let metadata_json = metadata.to_json()?;
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        UPDATE books SET category = ?, metadata = ?, book_file_path = ?, metadata_file_path = ?, cover_path = ?,
            updated_at = ?, sync_status = 'synced'
        WHERE id = ?
        "#,
    )
    .bind(&target_category)
    .bind(&metadata_json)
    .bind(new_book.to_string_lossy().as_ref())
    .bind(new_meta.to_string_lossy().as_ref())
    .bind(new_cover.to_string_lossy().as_ref())
    .bind(&now)
    .bind(book_id.to_string())
    .execute(&state.db)
    .await?;

    get_book(state, user, book_id).await
}

pub async fn update_progress(
    state: &AppState,
    user: &AuthUser,
    book_id: &Uuid,
    progress: ReadingProgress,
) -> AppResult<BookDetail> {
    let mut detail = get_book(state, user, book_id).await?;
    if !detail.permissions.can_edit {
        return Err(AppError::Forbidden("Edit permission required".into()));
    }

    detail.metadata.reading_progress = Some(progress);
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE books SET metadata = ?, last_read_at = ?, updated_at = ?, sync_status = 'pending' WHERE id = ?",
    )
    .bind(detail.metadata.to_json()?)
    .bind(&now)
    .bind(&now)
    .bind(book_id.to_string())
    .execute(&state.db)
    .await?;

    let row = fetch_row(&state.db, book_id).await?;
    let meta_path = PathBuf::from(&row.metadata_file_path);
    if let Some(dir) = meta_path.parent() {
        if let Some(base) = meta_path.file_stem().and_then(|s| s.to_str()) {
            let _ = fs::write_metadata_file(dir, base, &detail.metadata).await;
        }
    }

    get_book(state, user, book_id).await
}

pub async fn update_cover(
    state: &AppState,
    user: &AuthUser,
    book_id: &Uuid,
    cover_bytes: Vec<u8>,
    cover_ext: String,
) -> AppResult<BookDetail> {
    fs::validate_cover_extension(&cover_ext)?;
    let row = fetch_row(&state.db, book_id).await?;
    let library_id =
        Uuid::parse_str(&row.library_id).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(&state.db, user, &library_id).await?;
    library::require_edit(&perm)?;

    let book_path = PathBuf::from(&row.book_file_path);
    let dir = book_path
        .parent()
        .ok_or_else(|| AppError::Internal("Invalid book path".into()))?
        .to_path_buf();

    let meta = BookMetadata::from_json(&row.metadata)?;
    let base =
        fs::file_stem(&book_path).unwrap_or_else(|| fs::book_base_name(&meta.title, &meta.author));

    if let Some(old_cover) = fs::find_cover_in_dir(&dir, Some(&base)) {
        if old_cover != dir.join(format!("{base}.{cover_ext}")) {
            tokio::fs::remove_file(&old_cover).await.ok();
        }
    }
    thumbnail::remove_for_cover(&PathBuf::from(&row.cover_path)).await;

    let cover_path = fs::write_cover_file(&dir, &base, &cover_ext, &cover_bytes).await?;
    let now = Utc::now().to_rfc3339();

    sqlx::query("UPDATE books SET cover_path = ?, updated_at = ? WHERE id = ?")
        .bind(cover_path.to_string_lossy().as_ref())
        .bind(&now)
        .bind(book_id.to_string())
        .execute(&state.db)
        .await?;

    get_book(state, user, book_id).await
}

pub async fn delete_book(state: &AppState, user: &AuthUser, book_id: &Uuid) -> AppResult<()> {
    let row = fetch_row(&state.db, book_id).await?;
    let library_id =
        Uuid::parse_str(&row.library_id).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(&state.db, user, &library_id).await?;
    library::require_delete(&perm)?;

    fs::remove_book_files(
        Path::new(&row.book_file_path),
        Path::new(&row.metadata_file_path),
        Path::new(&row.cover_path),
    )
    .await?;

    sqlx::query("DELETE FROM books WHERE id = ?")
        .bind(book_id.to_string())
        .execute(&state.db)
        .await?;
    Ok(())
}

pub async fn get_cover_path(
    state: &AppState,
    user: &AuthUser,
    book_id: &Uuid,
) -> AppResult<PathBuf> {
    let row = fetch_row(&state.db, book_id).await?;
    let library_id =
        Uuid::parse_str(&row.library_id).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(&state.db, user, &library_id).await?;
    library::require_view(&perm)?;
    let path = PathBuf::from(&row.cover_path);
    if fs::has_stored_cover(&path) {
        Ok(path)
    } else {
        Err(AppError::NotFound("Cover not found".into()))
    }
}

pub async fn get_download_path(
    state: &AppState,
    user: &AuthUser,
    book_id: &Uuid,
) -> AppResult<(PathBuf, String)> {
    let row = fetch_row(&state.db, book_id).await?;
    let library_id =
        Uuid::parse_str(&row.library_id).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(&state.db, user, &library_id).await?;
    library::require_view(&perm)?;

    let path = PathBuf::from(&row.book_file_path);
    if !path.is_file() {
        return Err(AppError::NotFound("Book file not found on disk".into()));
    }
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("book")
        .to_string();
    Ok((path, filename))
}

pub async fn search_books(
    db: &SqlitePool,
    user: &AuthUser,
    q: &str,
    library_id: Option<Uuid>,
    limit: i64,
) -> AppResult<Vec<BookCard>> {
    let accessible = accessible_library_ids(db, user).await?;
    if accessible.is_empty() {
        return Ok(vec![]);
    }

    let pattern = format!("%{q}%");
    let mut results = Vec::new();

    for lid in accessible {
        if let Some(filter) = library_id {
            if filter.to_string() != lid {
                continue;
            }
        }

        let rows: Vec<BookDbRow> = sqlx::query_as(
            r#"
            SELECT id, library_id, category, book_file_path, metadata_file_path, cover_path,
                   metadata, sync_status, last_read_at, uploaded_at, updated_at
            FROM books
            WHERE library_id = ?
              AND (
                json_extract(metadata, '$.title') LIKE ?
                OR json_extract(metadata, '$.author') LIKE ?
                OR json_extract(metadata, '$.language') LIKE ?
                OR json_extract(metadata, '$.isbn') LIKE ?
              )
            LIMIT ?
            "#,
        )
        .bind(&lid)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(db)
        .await?;

        for r in rows {
            results.push(row_to_card(&book_row_from_db(r))?);
        }
    }

    results.truncate(limit as usize);
    Ok(results)
}

pub async fn upsert_from_fs(
    db: &SqlitePool,
    library_id: &Uuid,
    category: &str,
    book_id: &Uuid,
    book_file: &std::path::Path,
) -> AppResult<()> {
    let dir = book_file
        .parent()
        .ok_or_else(|| AppError::Internal("Invalid book file path".into()))?;
    let book_stem = fs::file_stem(book_file);
    let metadata_path = fs::find_metadata_in_dir(dir, book_stem.as_deref())
        .unwrap_or_else(|| dir.join("metadata.json"));
    let cover_file = fs::find_cover_in_dir(dir, book_stem.as_deref())
        .filter(|p| p.is_file())
        .unwrap_or_default();

    let mut metadata = if metadata_path.is_file() {
        fs::read_metadata_file(&metadata_path).await?
    } else {
        let stem = book_stem.unwrap_or_else(|| book_id.to_string());
        BookMetadata {
            title: stem.replace('_', " "),
            category: category.to_string(),
            ..Default::default()
        }
    };

    if book_file.is_file() {
        metadata.file_md5 = Some(hash::md5_hex_file(book_file).await?);
    }

    let metadata_json = metadata.to_json()?;
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO books (id, library_id, category, book_file_path, metadata_file_path, cover_path, metadata, sync_status, uploaded_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 'synced', ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            category = excluded.category,
            book_file_path = excluded.book_file_path,
            metadata_file_path = excluded.metadata_file_path,
            cover_path = excluded.cover_path,
            metadata = excluded.metadata,
            sync_status = 'synced',
            updated_at = excluded.updated_at
        "#,
    )
    .bind(book_id.to_string())
    .bind(library_id.to_string())
    .bind(category)
    .bind(book_file.to_string_lossy().as_ref())
    .bind(metadata_path.to_string_lossy().as_ref())
    .bind(cover_file.to_string_lossy().to_string())
    .bind(&metadata_json)
    .bind(&now)
    .bind(&now)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn mark_orphan(db: &SqlitePool, book_id: &Uuid) -> AppResult<()> {
    sqlx::query("UPDATE books SET sync_status = 'orphan', updated_at = ? WHERE id = ?")
        .bind(Utc::now().to_rfc3339())
        .bind(book_id.to_string())
        .execute(db)
        .await?;
    Ok(())
}
