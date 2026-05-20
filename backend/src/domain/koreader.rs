use crate::domain::auth::AuthUser;
use crate::domain::book;
use crate::domain::library;
use crate::error::{AppError, AppResult};
use crate::infra::ReadingProgress;
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use utoipa::ToSchema;
use uuid::Uuid;

type KosyncProgressDbRow = (f64, String, Option<String>, Option<String>, i64);

type KoreaderProgressListRow = (
    String,
    String,
    String,
    String,
    f64,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);

#[derive(Debug, Serialize, ToSchema)]
pub struct KosyncAuthResponse {
    pub authorized: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KosyncProgressResponse {
    pub document: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KosyncUpdateResponse {
    pub document: String,
    pub timestamp: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct KosyncUpdateRequest {
    pub document: String,
    pub progress: String,
    pub percentage: f64,
    pub device: String,
    pub device_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KoreaderProgressRow {
    pub user_id: String,
    pub username: String,
    pub document: String,
    pub progress: String,
    pub percentage: f64,
    pub device: Option<String>,
    pub book_id: Option<String>,
    pub book_title: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct KoreaderDocumentLink {
    pub document: String,
    pub book_id: String,
    pub book_title: String,
    pub book_author: String,
    pub library_id: String,
    pub link_source: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetDocumentLinkRequest {
    pub document: String,
    pub book_id: String,
}

fn is_valid_document(document: &str) -> bool {
    !document.is_empty() && !document.contains(':')
}

pub async fn authenticate_kosync(
    db: &SqlitePool,
    username: &str,
    auth_key: &str,
) -> AppResult<AuthUser> {
    if username.is_empty() || auth_key.is_empty() {
        return Err(AppError::Unauthorized("Invalid credentials".into()));
    }

    let row: Option<(String, Option<String>, String)> =
        sqlx::query_as("SELECT id, password_md5, role FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(db)
            .await?;

    let (id, password_md5, role) =
        row.ok_or_else(|| AppError::Unauthorized("Invalid credentials".into()))?;

    let stored = password_md5.filter(|k| !k.is_empty()).ok_or_else(|| {
        AppError::Unauthorized("KOReader sync not enabled: log in via web once".into())
    })?;

    if stored != auth_key {
        return Err(AppError::Unauthorized("Invalid credentials".into()));
    }

    Ok(AuthUser {
        id: Uuid::parse_str(&id).map_err(|e| AppError::Internal(e.to_string()))?,
        username: username.to_string(),
        role,
    })
}

pub async fn kosync_auth_user(
    db: &SqlitePool,
    username: &str,
    auth_key: &str,
) -> AppResult<KosyncAuthResponse> {
    authenticate_kosync(db, username, auth_key).await?;
    Ok(KosyncAuthResponse {
        authorized: "OK".into(),
    })
}

pub async fn kosync_get_progress(
    state: &AppState,
    user: &AuthUser,
    document: &str,
) -> AppResult<KosyncProgressResponse> {
    if !is_valid_document(document) {
        return Err(AppError::BadRequest("invalid document".into()));
    }

    let row: Option<KosyncProgressDbRow> = sqlx::query_as(
        r#"
        SELECT percentage, progress, device, device_id, timestamp
        FROM koreader_progress
        WHERE user_id = ? AND document = ?
        "#,
    )
    .bind(user.id.to_string())
    .bind(document)
    .fetch_optional(&state.db)
    .await?;

    let Some((percentage, progress, device, device_id, timestamp)) = row else {
        return Ok(KosyncProgressResponse {
            document: document.to_string(),
            percentage: None,
            progress: None,
            device: None,
            device_id: None,
            timestamp: None,
        });
    };

    Ok(KosyncProgressResponse {
        document: document.to_string(),
        percentage: Some(percentage),
        progress: Some(progress),
        device,
        device_id,
        timestamp: Some(timestamp),
    })
}

pub async fn kosync_update_progress(
    state: &AppState,
    user: &AuthUser,
    req: KosyncUpdateRequest,
) -> AppResult<KosyncUpdateResponse> {
    if !is_valid_document(&req.document) {
        return Err(AppError::BadRequest("invalid document".into()));
    }
    if req.device.is_empty() || req.progress.is_empty() {
        return Err(AppError::BadRequest("invalid fields".into()));
    }

    let timestamp = Utc::now().timestamp();
    let metadata_json = req
        .metadata
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;

    let book_id = resolve_book_for_document(&state.db, user, &req.document).await?;

    sqlx::query(
        r#"
        INSERT INTO koreader_progress (user_id, document, progress, percentage, device, device_id, timestamp, book_id, metadata_json, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
        ON CONFLICT(user_id, document) DO UPDATE SET
            progress = excluded.progress,
            percentage = excluded.percentage,
            device = excluded.device,
            device_id = excluded.device_id,
            timestamp = excluded.timestamp,
            book_id = excluded.book_id,
            metadata_json = excluded.metadata_json,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(user.id.to_string())
    .bind(&req.document)
    .bind(&req.progress)
    .bind(req.percentage)
    .bind(&req.device)
    .bind(&req.device_id)
    .bind(timestamp)
    .bind(book_id.as_ref().map(|id| id.to_string()))
    .bind(metadata_json)
    .execute(&state.db)
    .await?;

    if let Some(bid) = book_id {
        apply_progress_to_book(state, user, &bid, &req.progress, req.percentage).await?;
    }

    Ok(KosyncUpdateResponse {
        document: req.document,
        timestamp,
    })
}

async fn resolve_book_for_document(
    db: &SqlitePool,
    user: &AuthUser,
    document: &str,
) -> AppResult<Option<Uuid>> {
    if let Some(book_id) = lookup_manual_link(db, user, document).await? {
        return Ok(Some(book_id));
    }

    if let Some(book_id) = lookup_book_by_file_md5(db, user, document).await? {
        upsert_document_link(db, document, &book_id, "md5").await?;
        return Ok(Some(book_id));
    }

    Ok(None)
}

async fn lookup_manual_link(
    db: &SqlitePool,
    user: &AuthUser,
    document: &str,
) -> AppResult<Option<Uuid>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT book_id FROM koreader_document_links WHERE document = ?")
            .bind(document)
            .fetch_optional(db)
            .await?;

    let Some((book_id,)) = row else {
        return Ok(None);
    };

    let book_id = Uuid::parse_str(&book_id).map_err(|e| AppError::Internal(e.to_string()))?;
    if user_can_view_book(db, user, &book_id).await? {
        Ok(Some(book_id))
    } else {
        Ok(None)
    }
}

async fn lookup_book_by_file_md5(
    db: &SqlitePool,
    user: &AuthUser,
    document: &str,
) -> AppResult<Option<Uuid>> {
    let accessible = book::accessible_library_ids(db, user).await?;
    if accessible.is_empty() {
        return Ok(None);
    }

    let doc_lower = document.to_lowercase();
    for lid in accessible {
        let row: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT id FROM books
            WHERE library_id = ?
              AND lower(json_extract(metadata, '$.file_md5')) = ?
            LIMIT 1
            "#,
        )
        .bind(&lid)
        .bind(&doc_lower)
        .fetch_optional(db)
        .await?;

        if let Some((id,)) = row {
            return Ok(Some(
                Uuid::parse_str(&id).map_err(|e| AppError::Internal(e.to_string()))?,
            ));
        }
    }
    Ok(None)
}

async fn user_can_view_book(db: &SqlitePool, user: &AuthUser, book_id: &Uuid) -> AppResult<bool> {
    let row: Option<(String,)> = sqlx::query_as("SELECT library_id FROM books WHERE id = ?")
        .bind(book_id.to_string())
        .fetch_optional(db)
        .await?;

    let Some((library_id,)) = row else {
        return Ok(false);
    };

    let library_id = Uuid::parse_str(&library_id).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(db, user, &library_id).await?;
    Ok(perm.can_view)
}

async fn upsert_document_link(
    db: &SqlitePool,
    document: &str,
    book_id: &Uuid,
    source: &str,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO koreader_document_links (document, book_id, link_source, updated_at)
        VALUES (?, ?, ?, datetime('now'))
        ON CONFLICT(document) DO UPDATE SET
            book_id = excluded.book_id,
            link_source = excluded.link_source,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(document)
    .bind(book_id.to_string())
    .bind(source)
    .execute(db)
    .await?;
    Ok(())
}

async fn apply_progress_to_book(
    state: &AppState,
    user: &AuthUser,
    book_id: &Uuid,
    progress_str: &str,
    percentage: f64,
) -> AppResult<()> {
    let row = book::fetch_row_internal(&state.db, book_id).await?;
    let library_id =
        Uuid::parse_str(&row.library_id).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(&state.db, user, &library_id).await?;
    if !perm.can_edit {
        return Ok(());
    }

    let old_up = crate::domain::user_reading_progress::get(&state.db, &user.id, book_id).await?;
    let shelfie_percent = percentage * 100.0;
    let current_page = progress_str.parse::<i32>().ok();
    let new_progress = ReadingProgress {
        current_page,
        percent: Some(shelfie_percent),
        last_position: Some(progress_str.to_string()),
    };

    crate::domain::user_reading_progress::upsert(
        &state.db,
        &user.id,
        book_id,
        &library_id,
        &new_progress,
    )
    .await?;

    crate::domain::reading_history::append_if_changed(
        &state.db,
        &user.id,
        book_id,
        &library_id,
        old_up.progress.as_ref(),
        &new_progress,
        "koreader",
    )
    .await?;

    Ok(())
}

// --- Admin APIs ---

pub async fn list_progress(
    state: &AppState,
    user: &AuthUser,
    library_id: Option<Uuid>,
) -> AppResult<Vec<KoreaderProgressRow>> {
    let accessible = book::accessible_library_ids(&state.db, user).await?;
    if accessible.is_empty() && user.role != "system_admin" {
        return Ok(vec![]);
    }

    let rows: Vec<KoreaderProgressListRow> = sqlx::query_as(
        r#"
        SELECT kp.user_id, u.username, kp.document, kp.progress, kp.percentage, kp.device,
               kp.book_id,
               json_extract(b.metadata, '$.title'),
               json_extract(b.metadata, '$.author'),
               kp.updated_at
        FROM koreader_progress kp
        JOIN users u ON u.id = kp.user_id
        LEFT JOIN books b ON b.id = kp.book_id
        ORDER BY kp.updated_at DESC
        LIMIT 500
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let mut out = Vec::new();
    for r in rows {
        let is_own = r.0 == user.id.to_string();
        let book_lib = if let Some(ref bid) = r.6 {
            let lib: Option<(String,)> =
                sqlx::query_as("SELECT library_id FROM books WHERE id = ?")
                    .bind(bid)
                    .fetch_optional(&state.db)
                    .await?;
            lib.map(|(l,)| l)
        } else {
            None
        };

        if user.role != "system_admin" {
            let allowed = is_own
                || book_lib
                    .as_ref()
                    .is_some_and(|lid| accessible.contains(lid));
            if !allowed {
                continue;
            }
        }

        if let Some(filter) = library_id {
            match book_lib {
                Some(lid) if lid == filter.to_string() => {}
                _ => continue,
            }
        }

        out.push(KoreaderProgressRow {
            user_id: r.0,
            username: r.1,
            document: r.2,
            progress: r.3,
            percentage: r.4,
            device: r.5,
            book_id: r.6,
            book_title: r.7,
            updated_at: r.9,
        });
    }
    Ok(out)
}

pub async fn list_document_links(
    state: &AppState,
    user: &AuthUser,
    library_id: Option<Uuid>,
) -> AppResult<Vec<KoreaderDocumentLink>> {
    let accessible = book::accessible_library_ids(&state.db, user).await?;

    let rows: Vec<(String, String, String, String, String, String, String)> = sqlx::query_as(
        r#"
        SELECT l.document, l.book_id,
               json_extract(b.metadata, '$.title'),
               json_extract(b.metadata, '$.author'),
               b.library_id, l.link_source, l.updated_at
        FROM koreader_document_links l
        JOIN books b ON b.id = l.book_id
        ORDER BY l.updated_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let mut out = Vec::new();
    for r in rows {
        if let Some(filter) = library_id {
            if r.4 != filter.to_string() {
                continue;
            }
        }
        if user.role != "system_admin" && !accessible.contains(&r.4) {
            continue;
        }
        out.push(KoreaderDocumentLink {
            document: r.0,
            book_id: r.1,
            book_title: r.2,
            book_author: r.3,
            library_id: r.4,
            link_source: r.5,
            updated_at: r.6,
        });
    }
    Ok(out)
}

pub async fn set_document_link(
    state: &AppState,
    user: &AuthUser,
    req: SetDocumentLinkRequest,
) -> AppResult<KoreaderDocumentLink> {
    if !is_valid_document(&req.document) {
        return Err(AppError::BadRequest("invalid document".into()));
    }

    let book_id = Uuid::parse_str(&req.book_id)
        .map_err(|_| AppError::BadRequest("invalid book_id".into()))?;

    let row: (String, String, String) = sqlx::query_as(
        r#"
        SELECT library_id, json_extract(metadata, '$.title'), json_extract(metadata, '$.author')
        FROM books WHERE id = ?
        "#,
    )
    .bind(book_id.to_string())
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Book not found".into()))?;

    let library_id = Uuid::parse_str(&row.0).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(&state.db, user, &library_id).await?;
    library::require_edit(&perm)?;

    upsert_document_link(&state.db, &req.document, &book_id, "manual").await?;

    // Backfill koreader_progress.book_id for this user+document
    sqlx::query("UPDATE koreader_progress SET book_id = ? WHERE document = ?")
        .bind(book_id.to_string())
        .bind(&req.document)
        .execute(&state.db)
        .await?;

    if let Some((progress, percentage)) = sqlx::query_as::<_, (String, f64)>(
        "SELECT progress, percentage FROM koreader_progress WHERE document = ? AND user_id = ?",
    )
    .bind(&req.document)
    .bind(user.id.to_string())
    .fetch_optional(&state.db)
    .await?
    {
        apply_progress_to_book(state, user, &book_id, &progress, percentage).await?;
    }

    let link = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
        r#"
        SELECT l.document, l.book_id,
               json_extract(b.metadata, '$.title'),
               json_extract(b.metadata, '$.author'),
               b.library_id, l.link_source, l.updated_at
        FROM koreader_document_links l
        JOIN books b ON b.id = l.book_id
        WHERE l.document = ?
        "#,
    )
    .bind(&req.document)
    .fetch_one(&state.db)
    .await?;

    Ok(KoreaderDocumentLink {
        document: link.0,
        book_id: link.1,
        book_title: link.2,
        book_author: link.3,
        library_id: link.4,
        link_source: link.5,
        updated_at: link.6,
    })
}

pub async fn delete_document_link(
    state: &AppState,
    user: &AuthUser,
    document: &str,
) -> AppResult<()> {
    if !is_valid_document(document) {
        return Err(AppError::BadRequest("invalid document".into()));
    }

    let row: Option<(String,)> =
        sqlx::query_as("SELECT book_id FROM koreader_document_links WHERE document = ?")
            .bind(document)
            .fetch_optional(&state.db)
            .await?;

    let Some((book_id,)) = row else {
        return Err(AppError::NotFound("Link not found".into()));
    };

    let library_id: (String,) = sqlx::query_as("SELECT library_id FROM books WHERE id = ?")
        .bind(&book_id)
        .fetch_one(&state.db)
        .await?;
    let library_id =
        Uuid::parse_str(&library_id.0).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(&state.db, user, &library_id).await?;
    library::require_edit(&perm)?;

    sqlx::query("DELETE FROM koreader_document_links WHERE document = ?")
        .bind(document)
        .execute(&state.db)
        .await?;

    sqlx::query("UPDATE koreader_progress SET book_id = NULL WHERE document = ?")
        .bind(document)
        .execute(&state.db)
        .await?;

    Ok(())
}

pub async fn kosync_healthcheck() -> serde_json::Value {
    serde_json::json!({ "state": "OK" })
}
