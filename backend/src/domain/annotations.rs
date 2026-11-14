//! 标注 / 笔记 / 书签的存储视图：网页列表、metadata.json 快照写回与从快照恢复。
//! 同步协议见 `domain/bookorbit.rs`。

use crate::domain::auth::AuthUser;
use crate::domain::{book, library};
use crate::error::{AppError, AppResult};
use crate::infra::{fs, BookMetadata, MetadataAnnotation, MetadataBookmark};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use sqlx::SqlitePool;
use std::path::Path;
use utoipa::ToSchema;
use uuid::Uuid;

/// 服务端时间戳；固定格式，可直接按字符串比较先后
pub(crate) fn now_ts() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BookAnnotationDto {
    pub id: i64,
    pub text: String,
    pub note: Option<String>,
    pub chapter: Option<String>,
    /// KOReader 颜色名（yellow / red / green / blue / purple / orange / cyan / olive / gray）
    pub color: Option<String>,
    /// lighten（高亮）/ underscore（下划线）/ strikeout（删除线）/ invert（反色）
    pub drawer: String,
    pub pageno: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BookBookmarkDto {
    pub id: i64,
    pub title: String,
    pub note: Option<String>,
    pub chapter: Option<String>,
    pub pageno: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BookAnnotationsResponse {
    pub annotations: Vec<BookAnnotationDto>,
    pub bookmarks: Vec<BookBookmarkDto>,
}

type AnnotationListRow = (
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<i64>,
    String,
    String,
    String,
);

type BookmarkListRow = (
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<i64>,
    String,
    String,
);

/// XPointer 中 `DocFragment[N]` 的章节序号，用于按阅读顺序粗排
fn fragment_index(pos: &str) -> u32 {
    pos.split("DocFragment[")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(u32::MAX)
}

pub async fn list_for_user(
    db: &SqlitePool,
    user: &AuthUser,
    book_id: &Uuid,
) -> AppResult<BookAnnotationsResponse> {
    let row = book::fetch_row_internal(db, book_id).await?;
    let library_id =
        Uuid::parse_str(&row.library_id).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(db, user, &library_id).await?;
    library::require_view(&perm)?;

    let mut annotations: Vec<AnnotationListRow> = sqlx::query_as(
        r#"
        SELECT id, text, note, chapter, color, drawer, pageno, created_at, updated_at, pos0
        FROM book_annotations
        WHERE user_id = ? AND book_id = ? AND deleted_at IS NULL
        ORDER BY device_datetime, id
        "#,
    )
    .bind(user.id.to_string())
    .bind(book_id.to_string())
    .fetch_all(db)
    .await?;
    annotations.sort_by_key(|r| fragment_index(&r.9));

    let mut bookmarks: Vec<BookmarkListRow> = sqlx::query_as(
        r#"
        SELECT id, title, note, chapter, pageno, created_at, pos
        FROM book_bookmarks
        WHERE user_id = ? AND book_id = ? AND deleted_at IS NULL
        ORDER BY created_at, id
        "#,
    )
    .bind(user.id.to_string())
    .bind(book_id.to_string())
    .fetch_all(db)
    .await?;
    bookmarks.sort_by_key(|r| fragment_index(&r.6));

    Ok(BookAnnotationsResponse {
        annotations: annotations
            .into_iter()
            .map(|r| BookAnnotationDto {
                id: r.0,
                text: r.1,
                note: r.2,
                chapter: r.3,
                color: r.4,
                drawer: r.5,
                pageno: r.6,
                created_at: r.7,
                updated_at: r.8,
            })
            .collect(),
        bookmarks: bookmarks
            .into_iter()
            .map(|r| BookBookmarkDto {
                id: r.0,
                title: r.1,
                note: r.2,
                chapter: r.3,
                pageno: r.4,
                created_at: r.5,
            })
            .collect(),
    })
}

type SnapshotAnnotationRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
    Option<String>,
    Option<i64>,
    String,
    Option<String>,
    String,
    String,
);

type SnapshotBookmarkRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<i64>,
    Option<String>,
    String,
    String,
);

/// 以数据库中的有效标注/书签重写该书的元数据快照（DB 与 metadata.json）
pub async fn refresh_snapshot(db: &SqlitePool, book_id: &Uuid) -> AppResult<()> {
    let row = book::fetch_row_internal(db, book_id).await?;
    let mut meta = BookMetadata::from_json(&row.metadata)?;

    let annotations: Vec<SnapshotAnnotationRow> = sqlx::query_as(
        r#"
        SELECT u.username, a.text, a.note, a.chapter, a.color, a.drawer, a.pos_format, a.pos0,
               a.pos1, a.pageno, a.device_datetime, a.device_updated, a.created_at, a.updated_at
        FROM book_annotations a JOIN users u ON u.id = a.user_id
        WHERE a.book_id = ? AND a.deleted_at IS NULL
        ORDER BY u.username, a.device_datetime, a.id
        "#,
    )
    .bind(book_id.to_string())
    .fetch_all(db)
    .await?;
    let bookmarks: Vec<SnapshotBookmarkRow> = sqlx::query_as(
        r#"
        SELECT u.username, b.title, b.note, b.chapter, b.pos, b.pageno, b.device_datetime,
               b.created_at, b.updated_at
        FROM book_bookmarks b JOIN users u ON u.id = b.user_id
        WHERE b.book_id = ? AND b.deleted_at IS NULL
        ORDER BY u.username, b.created_at, b.id
        "#,
    )
    .bind(book_id.to_string())
    .fetch_all(db)
    .await?;

    meta.annotations = annotations
        .into_iter()
        .map(|r| MetadataAnnotation {
            username: r.0,
            text: r.1,
            note: r.2,
            chapter: r.3,
            color: r.4,
            drawer: r.5,
            pos_format: r.6,
            pos0: r.7,
            pos1: r.8,
            pageno: r.9,
            datetime: r.10,
            datetime_updated: r.11,
            created_at: r.12,
            updated_at: r.13,
        })
        .collect();
    meta.bookmarks = bookmarks
        .into_iter()
        .map(|r| MetadataBookmark {
            username: r.0,
            title: r.1,
            note: r.2,
            chapter: r.3,
            pos: r.4,
            pageno: r.5,
            datetime: r.6,
            created_at: r.7,
            updated_at: r.8,
        })
        .collect();

    persist_metadata(db, book_id, &row.metadata_file_path, &meta).await
}

/// 写 `books.metadata` 与同目录的 metadata.json
pub(crate) async fn persist_metadata(
    db: &SqlitePool,
    book_id: &Uuid,
    metadata_file_path: &str,
    meta: &BookMetadata,
) -> AppResult<()> {
    sqlx::query("UPDATE books SET metadata = ? WHERE id = ?")
        .bind(meta.to_json()?)
        .bind(book_id.to_string())
        .execute(db)
        .await?;
    let meta_path = Path::new(metadata_file_path);
    if let (Some(dir), Some(base)) = (meta_path.parent(), fs::file_stem(meta_path)) {
        fs::write_metadata_file(dir, &base, meta).await?;
    }
    Ok(())
}

/// 当前用户在该书上的阅读状态与页面阅读统计汇总
#[derive(Debug, Serialize, ToSchema)]
pub struct BookReadingSummary {
    /// reading / complete / abandoned
    pub status: Option<String>,
    /// 状态修改日期 `YYYY-MM-DD`
    pub status_modified: Option<String>,
    pub total_seconds: i64,
    /// 读过的不同页数
    pub pages_read: i64,
    /// 最近一次统计上报时的总页数
    pub total_pages: Option<i64>,
    /// 有阅读记录的天数（UTC）
    pub reading_days: i64,
    /// Unix 秒
    pub first_read_at: Option<i64>,
    pub last_read_at: Option<i64>,
}

pub async fn reading_summary(
    db: &SqlitePool,
    user: &AuthUser,
    book_id: &Uuid,
) -> AppResult<BookReadingSummary> {
    let row = book::fetch_row_internal(db, book_id).await?;
    let library_id =
        Uuid::parse_str(&row.library_id).map_err(|e| AppError::Internal(e.to_string()))?;
    let perm = library::resolve_permission(db, user, &library_id).await?;
    library::require_view(&perm)?;

    let status: Option<(String, String)> = sqlx::query_as(
        "SELECT status, status_modified FROM user_book_status WHERE user_id = ? AND book_id = ?",
    )
    .bind(user.id.to_string())
    .bind(book_id.to_string())
    .fetch_optional(db)
    .await?;
    let (total_seconds, pages_read, reading_days, first_read_at, last_read_at): (
        i64,
        i64,
        i64,
        Option<i64>,
        Option<i64>,
    ) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(duration_seconds), 0), COUNT(DISTINCT page),
               COUNT(DISTINCT date(start_time, 'unixepoch')), MIN(start_time), MAX(start_time)
        FROM reading_page_stats WHERE user_id = ? AND book_id = ?
        "#,
    )
    .bind(user.id.to_string())
    .bind(book_id.to_string())
    .fetch_one(db)
    .await?;
    let total_pages: Option<(i64,)> = sqlx::query_as(
        "SELECT total_pages FROM reading_page_stats WHERE user_id = ? AND book_id = ? \
         ORDER BY start_time DESC LIMIT 1",
    )
    .bind(user.id.to_string())
    .bind(book_id.to_string())
    .fetch_optional(db)
    .await?;

    let (status, status_modified) = status.unzip();
    Ok(BookReadingSummary {
        status,
        status_modified,
        total_seconds,
        pages_read,
        total_pages: total_pages.map(|(t,)| t),
        reading_days,
        first_read_at,
        last_read_at,
    })
}

async fn user_id_by_name(db: &SqlitePool, username: &str) -> AppResult<Option<String>> {
    Ok(
        sqlx::query_as::<_, (String,)>("SELECT id FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(db)
            .await?
            .map(|(id,)| id),
    )
}

/// 从 metadata.json 快照补回数据库中缺失的标注/书签（已删除的不会复活：墓碑保留）。
/// 快照中用户名不存在的条目跳过。
pub async fn restore_from_metadata(
    db: &SqlitePool,
    book_id: &Uuid,
    meta: &BookMetadata,
) -> AppResult<()> {
    for a in &meta.annotations {
        let Some(user_id) = user_id_by_name(db, &a.username).await? else {
            continue;
        };
        let exists: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT id FROM book_annotations
            WHERE user_id = ? AND book_id = ? AND device_datetime = ? AND pos0 = ?
            "#,
        )
        .bind(&user_id)
        .bind(book_id.to_string())
        .bind(&a.datetime)
        .bind(&a.pos0)
        .fetch_optional(db)
        .await?;
        if exists.is_some() {
            continue;
        }
        sqlx::query(
            r#"
            INSERT INTO book_annotations (user_id, book_id, text, note, chapter, drawer, color,
                pos_format, pos0, pos1, pageno, device_datetime, device_updated, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&user_id)
        .bind(book_id.to_string())
        .bind(&a.text)
        .bind(&a.note)
        .bind(&a.chapter)
        .bind(&a.drawer)
        .bind(&a.color)
        .bind(if a.pos_format == "pdf" { "pdf" } else { "xpointer" })
        .bind(&a.pos0)
        .bind(&a.pos1)
        .bind(a.pageno)
        .bind(&a.datetime)
        .bind(&a.datetime_updated)
        .bind(&a.created_at)
        .bind(&a.updated_at)
        .execute(db)
        .await?;
    }

    for b in &meta.bookmarks {
        let Some(user_id) = user_id_by_name(db, &b.username).await? else {
            continue;
        };
        let exists: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM book_bookmarks WHERE user_id = ? AND book_id = ? AND pos = ?",
        )
        .bind(&user_id)
        .bind(book_id.to_string())
        .bind(&b.pos)
        .fetch_optional(db)
        .await?;
        if exists.is_some() {
            continue;
        }
        sqlx::query(
            r#"
            INSERT INTO book_bookmarks (user_id, book_id, pos, pageno, chapter, note, title,
                device_datetime, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&user_id)
        .bind(book_id.to_string())
        .bind(&b.pos)
        .bind(b.pageno)
        .bind(&b.chapter)
        .bind(&b.note)
        .bind(&b.title)
        .bind(&b.datetime)
        .bind(&b.created_at)
        .bind(&b.updated_at)
        .execute(db)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::fragment_index;

    #[test]
    fn fragment_index_orders_by_docfragment() {
        assert_eq!(
            fragment_index("/body/DocFragment[12]/body/p[3]/text().4"),
            12
        );
        assert_eq!(fragment_index("12"), u32::MAX);
    }
}
