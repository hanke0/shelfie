//! BookOrbit 同步管理（网页）：当前用户的同步设备、按书汇总、重置设备同步状态。

use crate::domain::auth::AuthUser;
use crate::domain::book;
use crate::error::{AppError, AppResult};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// 同步状态表中的 device_id 为 `{设备 id}:{32 位文档哈希}`
const DEVICE_OF: &str = "substr(device_id, 1, length(device_id) - 33)";

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct BookOrbitDevice {
    pub device_id: String,
    /// 有同步记录的图书数
    pub books: i64,
    /// 已同步到该设备的有效标注数
    pub annotations: i64,
    pub bookmarks: i64,
    /// 上报过的页面阅读统计条数
    pub page_stats: i64,
    /// 最近一次同步产生变更的时间（RFC3339）
    pub last_change_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BookOrbitBookSummary {
    pub book_id: String,
    pub title: String,
    pub author: String,
    pub annotations: i64,
    pub bookmarks: i64,
    /// reading / complete / abandoned
    pub status: Option<String>,
    pub total_seconds: i64,
    /// Unix 秒
    pub last_read_at: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResetDeviceResponse {
    pub annotation_states: u64,
    pub bookmark_links: u64,
}

pub async fn list_devices(db: &SqlitePool, user: &AuthUser) -> AppResult<Vec<BookOrbitDevice>> {
    let uid = user.id.to_string();
    let mut devices: BTreeMap<String, BookOrbitDevice> = BTreeMap::new();

    let annotation_rows: Vec<(String, i64, i64, Option<String>)> =
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT {DEVICE_OF} AS d, COUNT(DISTINCT a.book_id),
                   SUM(a.deleted_at IS NULL AND s.last_applied_version > 0), MAX(s.updated_at)
            FROM annotation_sync_states s JOIN book_annotations a ON a.id = s.annotation_id
            WHERE a.user_id = ? GROUP BY d
            "#
        )))
        .bind(&uid)
        .fetch_all(db)
        .await?;
    for (id, books, annotations, last) in annotation_rows {
        let d = devices.entry(id.clone()).or_default();
        d.device_id = id;
        d.books = d.books.max(books);
        d.annotations = annotations;
        d.last_change_at = d.last_change_at.take().max(last);
    }

    let bookmark_rows: Vec<(String, i64, i64, Option<String>)> =
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT {DEVICE_OF} AS d, COUNT(DISTINCT b.book_id), SUM(b.deleted_at IS NULL),
                   MAX(l.applied_at)
            FROM bookmark_sync_links l JOIN book_bookmarks b ON b.id = l.bookmark_id
            WHERE b.user_id = ? GROUP BY d
            "#
        )))
        .bind(&uid)
        .fetch_all(db)
        .await?;
    for (id, books, bookmarks, last) in bookmark_rows {
        let d = devices.entry(id.clone()).or_default();
        d.device_id = id;
        d.books = d.books.max(books);
        d.bookmarks = bookmarks;
        d.last_change_at = d.last_change_at.take().max(last);
    }

    let stats_rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT device_id, COUNT(DISTINCT book_id), COUNT(*) FROM reading_page_stats \
         WHERE user_id = ? GROUP BY device_id",
    )
    .bind(&uid)
    .fetch_all(db)
    .await?;
    for (id, books, page_stats) in stats_rows {
        let d = devices.entry(id.clone()).or_default();
        d.device_id = id;
        d.books = d.books.max(books);
        d.page_stats = page_stats;
    }

    let mut out: Vec<BookOrbitDevice> = devices.into_values().collect();
    out.sort_by(|a, b| b.last_change_at.cmp(&a.last_change_at));
    Ok(out)
}

/// 清除该设备的标注/书签同步状态；下次同步时服务端会把全部内容重新下发给它。阅读统计保留
pub async fn reset_device(
    db: &SqlitePool,
    user: &AuthUser,
    device_id: &str,
) -> AppResult<ResetDeviceResponse> {
    if device_id.is_empty() {
        return Err(AppError::BadRequest("device id is required".into()));
    }
    let uid = user.id.to_string();
    let mut tx = db.begin().await?;
    let annotation_states = sqlx::query(sqlx::AssertSqlSafe(format!(
        "DELETE FROM annotation_sync_states WHERE {DEVICE_OF} = ? AND annotation_id IN \
         (SELECT id FROM book_annotations WHERE user_id = ?)"
    )))
    .bind(device_id)
    .bind(&uid)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    let bookmark_links = sqlx::query(sqlx::AssertSqlSafe(format!(
        "DELETE FROM bookmark_sync_links WHERE {DEVICE_OF} = ? AND bookmark_id IN \
         (SELECT id FROM book_bookmarks WHERE user_id = ?)"
    )))
    .bind(device_id)
    .bind(&uid)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    Ok(ResetDeviceResponse {
        annotation_states,
        bookmark_links,
    })
}

type BookSummaryRow = (
    String,
    Option<String>,
    Option<String>,
    String,
    i64,
    i64,
    Option<String>,
    i64,
    Option<i64>,
);

/// 当前用户有标注、书签、阅读状态或阅读统计的图书（仅限可见图书馆）
pub async fn list_books(
    db: &SqlitePool,
    user: &AuthUser,
    library_id: Option<Uuid>,
) -> AppResult<Vec<BookOrbitBookSummary>> {
    let accessible = book::accessible_library_ids(db, user).await?;
    let rows: Vec<BookSummaryRow> = sqlx::query_as(
        r#"
        WITH mine(book_id) AS (
            SELECT book_id FROM book_annotations WHERE user_id = ?1 AND deleted_at IS NULL
            UNION SELECT book_id FROM book_bookmarks WHERE user_id = ?1 AND deleted_at IS NULL
            UNION SELECT book_id FROM user_book_status WHERE user_id = ?1
            UNION SELECT book_id FROM reading_page_stats WHERE user_id = ?1
        )
        SELECT b.id, json_extract(b.metadata, '$.title'), json_extract(b.metadata, '$.author'),
               b.library_id,
               (SELECT COUNT(*) FROM book_annotations
                WHERE user_id = ?1 AND book_id = b.id AND deleted_at IS NULL),
               (SELECT COUNT(*) FROM book_bookmarks
                WHERE user_id = ?1 AND book_id = b.id AND deleted_at IS NULL),
               (SELECT status FROM user_book_status WHERE user_id = ?1 AND book_id = b.id),
               (SELECT COALESCE(SUM(duration_seconds), 0) FROM reading_page_stats
                WHERE user_id = ?1 AND book_id = b.id),
               (SELECT MAX(start_time) FROM reading_page_stats
                WHERE user_id = ?1 AND book_id = b.id)
        FROM mine JOIN books b ON b.id = mine.book_id
        ORDER BY 9 DESC, 2
        LIMIT 500
        "#,
    )
    .bind(user.id.to_string())
    .fetch_all(db)
    .await?;

    let filter = library_id.map(|l| l.to_string());
    Ok(rows
        .into_iter()
        .filter(|r| accessible.contains(&r.3) && filter.as_ref().is_none_or(|f| f == &r.3))
        .map(|r| BookOrbitBookSummary {
            book_id: r.0,
            title: r.1.unwrap_or_default(),
            author: r.2.unwrap_or_default(),
            annotations: r.4,
            bookmarks: r.5,
            status: r.6,
            total_seconds: r.7,
            last_read_at: r.8,
        })
        .collect())
}
