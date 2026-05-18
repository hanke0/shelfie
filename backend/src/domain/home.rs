use crate::domain::auth::AuthUser;
use crate::domain::book::BookCard;
use crate::error::AppResult;
use crate::infra::BookMetadata;
use crate::state::AppState;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct HomeResponse {
    /// 最近阅读：按 last_read_at 降序，有阅读进度的优先展示
    pub recent: Vec<BookCard>,
    /// 新书速递：按 uploaded_at 降序
    pub new_arrivals: Vec<BookCard>,
    /// 啊哈时刻：随机推荐
    pub aha_moment: Vec<BookCard>,
}

fn map_rows(
    rows: Vec<(
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
    )>,
) -> Result<Vec<BookCard>, serde_json::Error> {
    rows.into_iter()
        .map(|r| {
            let meta: BookMetadata = serde_json::from_str(&r.6)?;
            let id = r.0.clone();
            Ok(BookCard {
                id: id.clone(),
                library_id: r.1,
                title: meta.title,
                author: meta.author,
                category: r.2,
                cover_url: format!("/api/v1/assets/covers/{id}"),
                reading_percent: meta.reading_progress.and_then(|p| p.percent),
                last_read_at: r.8,
            })
        })
        .collect()
}

async fn accessible_ids(state: &AppState, user: &AuthUser) -> AppResult<Vec<String>> {
    if user.role == "system_admin" {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM libraries")
            .fetch_all(&state.db)
            .await?;
        return Ok(rows.into_iter().map(|r| r.0).collect());
    }
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT library_id FROM library_members WHERE user_id = ? AND can_view = 1",
    )
    .bind(user.id.to_string())
    .fetch_all(&state.db)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

pub async fn get_home(
    state: &AppState,
    user: &AuthUser,
    library_id: Option<uuid::Uuid>,
    limit: i64,
    _seed: Option<i64>,
) -> AppResult<HomeResponse> {
    let mut libs = accessible_ids(state, user).await?;
    if libs.is_empty() {
        return Ok(HomeResponse {
            recent: vec![],
            new_arrivals: vec![],
            aha_moment: vec![],
        });
    }

    if let Some(lid) = library_id {
        let lid_str = lid.to_string();
        if !libs.contains(&lid_str) {
            return Err(crate::error::AppError::Forbidden(
                "No access to this library".into(),
            ));
        }
        libs = vec![lid_str];
    }

    let placeholders = libs.iter().map(|_| "?").collect::<Vec<_>>().join(",");

    let recent_sql = format!(
        r#"
        SELECT id, library_id, category, book_file_path, metadata_file_path, cover_path,
               metadata, sync_status, last_read_at, uploaded_at, updated_at
        FROM books
        WHERE library_id IN ({placeholders}) AND last_read_at IS NOT NULL
        ORDER BY last_read_at DESC
        LIMIT ?
        "#
    );
    let mut recent_q = sqlx::query_as::<_, (
        String, String, String, String, String, String, String, String, Option<String>, String, String,
    )>(&recent_sql);
    for id in &libs {
        recent_q = recent_q.bind(id);
    }
    recent_q = recent_q.bind(limit);
    let recent = map_rows(recent_q.fetch_all(&state.db).await?)?;

    let new_sql = format!(
        r#"
        SELECT id, library_id, category, book_file_path, metadata_file_path, cover_path,
               metadata, sync_status, last_read_at, uploaded_at, updated_at
        FROM books
        WHERE library_id IN ({placeholders})
        ORDER BY uploaded_at DESC
        LIMIT ?
        "#
    );
    let mut new_q = sqlx::query_as::<_, (
        String, String, String, String, String, String, String, String, Option<String>, String, String,
    )>(&new_sql);
    for id in &libs {
        new_q = new_q.bind(id);
    }
    new_q = new_q.bind(limit);
    let new_arrivals = map_rows(new_q.fetch_all(&state.db).await?)?;

    let random_sql = format!(
        r#"
        SELECT id, library_id, category, book_file_path, metadata_file_path, cover_path,
               metadata, sync_status, last_read_at, uploaded_at, updated_at
        FROM books
        WHERE library_id IN ({placeholders})
        ORDER BY RANDOM()
        LIMIT ?
        "#
    );

    let mut aha_q = sqlx::query_as::<_, (
        String, String, String, String, String, String, String, String, Option<String>, String, String,
    )>(&random_sql);
    for id in &libs {
        aha_q = aha_q.bind(id);
    }
    aha_q = aha_q.bind(limit);
    let aha_moment = map_rows(aha_q.fetch_all(&state.db).await?)?;

    Ok(HomeResponse {
        recent,
        new_arrivals,
        aha_moment,
    })
}
