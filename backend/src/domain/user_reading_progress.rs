use crate::error::AppResult;
use crate::infra::ReadingProgress;
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

type ProgressDbRow = (Option<f64>, Option<i32>, Option<String>, String);

#[derive(Debug, Clone, Default)]
pub struct UserProgressView {
    pub progress: Option<ReadingProgress>,
    pub updated_at: Option<String>,
}

impl UserProgressView {
    pub fn reading_percent(&self) -> Option<f64> {
        self.progress.as_ref().and_then(|p| p.percent)
    }
}

pub fn progress_from_columns(
    percent: Option<f64>,
    current_page: Option<i32>,
    last_position: Option<String>,
) -> Option<ReadingProgress> {
    if percent.is_none()
        && current_page.is_none()
        && last_position.as_deref().unwrap_or("").is_empty()
    {
        return None;
    }
    Some(ReadingProgress {
        percent,
        current_page,
        last_position: last_position.filter(|s| !s.is_empty()),
    })
}

#[allow(clippy::type_complexity)]
pub async fn get(db: &SqlitePool, user_id: &Uuid, book_id: &Uuid) -> AppResult<UserProgressView> {
    let row: Option<ProgressDbRow> = sqlx::query_as(
        r#"
        SELECT percent, current_page, last_position, updated_at
        FROM user_reading_progress
        WHERE user_id = ? AND book_id = ?
        "#,
    )
    .bind(user_id.to_string())
    .bind(book_id.to_string())
    .fetch_optional(db)
    .await?;

    let Some((percent, current_page, last_position, updated_at)) = row else {
        return Ok(UserProgressView::default());
    };

    Ok(UserProgressView {
        progress: progress_from_columns(percent, current_page, last_position),
        updated_at: Some(updated_at),
    })
}

pub async fn upsert(
    db: &SqlitePool,
    user_id: &Uuid,
    book_id: &Uuid,
    library_id: &Uuid,
    progress: &ReadingProgress,
) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO user_reading_progress
            (user_id, book_id, library_id, percent, current_page, last_position, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(user_id, book_id) DO UPDATE SET
            library_id = excluded.library_id,
            percent = excluded.percent,
            current_page = excluded.current_page,
            last_position = excluded.last_position,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(user_id.to_string())
    .bind(book_id.to_string())
    .bind(library_id.to_string())
    .bind(progress.percent)
    .bind(progress.current_page)
    .bind(progress.last_position.as_deref())
    .bind(&now)
    .execute(db)
    .await?;
    Ok(())
}
