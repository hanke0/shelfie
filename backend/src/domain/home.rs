use crate::domain::auth::AuthUser;
use crate::domain::book::{list_row_to_card, BookListRow};
use crate::error::AppResult;
use crate::state::AppState;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct HomeResponse {
    /// 最近阅读：按当前用户阅读时间降序
    pub recent: Vec<crate::domain::book::BookCard>,
    /// 新书速递：按 uploaded_at 降序
    pub new_arrivals: Vec<crate::domain::book::BookCard>,
    /// 啊哈时刻：随机推荐
    pub aha_moment: Vec<crate::domain::book::BookCard>,
}

async fn accessible_ids(state: &AppState, user: &AuthUser) -> AppResult<Vec<String>> {
    if user.role == "system_admin" {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM libraries")
            .fetch_all(&state.db)
            .await?;
        return Ok(rows.into_iter().map(|r| r.0).collect());
    }
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT library_id FROM library_members WHERE user_id = ? AND can_view = 1")
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
    let user_id = user.id.to_string();

    let recent_sql = format!(
        r#"
        SELECT b.id, b.library_id, b.category, b.book_file_path, b.metadata_file_path, b.cover_path,
               b.metadata, b.sync_status, b.last_read_at, b.uploaded_at, b.updated_at,
               urp.percent, urp.current_page, urp.last_position, urp.updated_at
        FROM user_reading_progress urp
        JOIN books b ON b.id = urp.book_id
        WHERE urp.user_id = ? AND urp.library_id IN ({placeholders})
        ORDER BY urp.updated_at DESC
        LIMIT ?
        "#
    );
    let mut recent_q = sqlx::query_as::<_, BookListRow>(&recent_sql);
    recent_q = recent_q.bind(&user_id);
    for id in &libs {
        recent_q = recent_q.bind(id);
    }
    recent_q = recent_q.bind(limit);
    let recent = recent_q
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .map(list_row_to_card)
        .collect::<Result<Vec<_>, _>>()?;

    let list_sql = format!(
        r#"
        SELECT b.id, b.library_id, b.category, b.book_file_path, b.metadata_file_path, b.cover_path,
               b.metadata, b.sync_status, b.last_read_at, b.uploaded_at, b.updated_at,
               urp.percent, urp.current_page, urp.last_position, urp.updated_at
        FROM books b
        LEFT JOIN user_reading_progress urp
            ON urp.book_id = b.id AND urp.user_id = ?
        WHERE b.library_id IN ({placeholders})
        "#
    );

    let new_sql = format!("{list_sql} ORDER BY b.uploaded_at DESC LIMIT ?");
    let mut new_q = sqlx::query_as::<_, BookListRow>(&new_sql);
    new_q = new_q.bind(&user_id);
    for id in &libs {
        new_q = new_q.bind(id);
    }
    new_q = new_q.bind(limit);
    let new_arrivals = new_q
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .map(list_row_to_card)
        .collect::<Result<Vec<_>, _>>()?;

    let random_sql = format!("{list_sql} ORDER BY RANDOM() LIMIT ?");
    let mut aha_q = sqlx::query_as::<_, BookListRow>(&random_sql);
    aha_q = aha_q.bind(&user_id);
    for id in &libs {
        aha_q = aha_q.bind(id);
    }
    aha_q = aha_q.bind(limit);
    let aha_moment = aha_q
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .map(list_row_to_card)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(HomeResponse {
        recent,
        new_arrivals,
        aha_moment,
    })
}
