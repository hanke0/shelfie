use crate::domain::auth::AuthUser;
use crate::domain::book;
use crate::error::AppResult;
use crate::infra::ReadingProgress;
use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadingHistoryEntry {
    pub id: String,
    pub book_id: String,
    pub book_title: String,
    pub book_author: String,
    pub library_id: String,
    pub library_name: String,
    pub cover_url: String,
    pub percent: Option<f64>,
    pub current_page: Option<i32>,
    pub source: String,
    pub recorded_at: String,
}

pub fn progress_changed(old: Option<&ReadingProgress>, new: &ReadingProgress) -> bool {
    let old = old.cloned().unwrap_or_default();
    !percent_close(old.percent, new.percent)
        || old.current_page != new.current_page
        || str_opt(&old.last_position) != str_opt(&new.last_position)
}

fn str_opt(s: &Option<String>) -> &str {
    s.as_deref().unwrap_or("")
}

fn percent_close(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => (x - y).abs() <= 0.01,
        _ => false,
    }
}

pub async fn append_if_changed(
    db: &SqlitePool,
    user_id: &Uuid,
    book_id: &Uuid,
    library_id: &Uuid,
    old: Option<&ReadingProgress>,
    new: &ReadingProgress,
    source: &str,
) -> AppResult<()> {
    if !progress_changed(old, new) {
        return Ok(());
    }

    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO reading_progress_history
            (id, user_id, book_id, library_id, percent, current_page, last_position, source, recorded_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(user_id.to_string())
    .bind(book_id.to_string())
    .bind(library_id.to_string())
    .bind(new.percent)
    .bind(new.current_page)
    .bind(new.last_position.as_deref())
    .bind(source)
    .bind(&now)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn list_for_user(
    db: &SqlitePool,
    user: &AuthUser,
    library_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<ReadingHistoryEntry>> {
    let accessible = book::accessible_library_ids(db, user).await?;
    if accessible.is_empty() {
        return Ok(vec![]);
    }

    if let Some(lid) = library_id {
        let lid_str = lid.to_string();
        if !accessible.contains(&lid_str) {
            return Ok(vec![]);
        }
    }

    let placeholders = accessible.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let mut sql = format!(
        r#"
        SELECT h.id, h.book_id,
               json_extract(b.metadata, '$.title') AS book_title,
               json_extract(b.metadata, '$.author') AS book_author,
               h.library_id, l.name AS library_name,
               h.percent, h.current_page, h.source, h.recorded_at
        FROM reading_progress_history h
        JOIN books b ON b.id = h.book_id
        JOIN libraries l ON l.id = h.library_id
        WHERE h.user_id = ?
          AND h.library_id IN ({placeholders})
        "#
    );

    if library_id.is_some() {
        sql.push_str(" AND h.library_id = ?");
    }
    sql.push_str(" ORDER BY h.recorded_at DESC LIMIT ? OFFSET ?");

    let mut q = sqlx::query_as::<_, HistoryRow>(&sql);
    q = q.bind(user.id.to_string());
    for id in &accessible {
        q = q.bind(id);
    }
    if let Some(lid) = library_id {
        q = q.bind(lid.to_string());
    }
    q = q.bind(limit).bind(offset);

    let rows = q.fetch_all(db).await?;
    Ok(rows.into_iter().map(row_to_entry).collect())
}

#[derive(sqlx::FromRow)]
struct HistoryRow {
    id: String,
    book_id: String,
    book_title: Option<String>,
    book_author: Option<String>,
    library_id: String,
    library_name: String,
    percent: Option<f64>,
    current_page: Option<i32>,
    source: String,
    recorded_at: String,
}

fn row_to_entry(r: HistoryRow) -> ReadingHistoryEntry {
    ReadingHistoryEntry {
        cover_url: format!("/api/v1/assets/covers/{}", r.book_id),
        book_title: r.book_title.unwrap_or_default(),
        book_author: r.book_author.unwrap_or_default(),
        id: r.id,
        book_id: r.book_id,
        library_id: r.library_id,
        library_name: r.library_name,
        percent: r.percent,
        current_page: r.current_page,
        source: r.source,
        recorded_at: r.recorded_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_progress_skips() {
        let p = ReadingProgress {
            current_page: Some(10),
            percent: Some(25.0),
            last_position: Some("10".into()),
        };
        assert!(!progress_changed(Some(&p), &p));
    }

    #[test]
    fn percent_delta_triggers() {
        let old = ReadingProgress {
            percent: Some(10.0),
            ..Default::default()
        };
        let new = ReadingProgress {
            percent: Some(10.5),
            ..Default::default()
        };
        assert!(progress_changed(Some(&old), &new));
    }

    #[test]
    fn page_change_triggers() {
        let old = ReadingProgress {
            current_page: Some(1),
            ..Default::default()
        };
        let new = ReadingProgress {
            current_page: Some(2),
            ..Default::default()
        };
        assert!(progress_changed(Some(&old), &new));
    }
}
