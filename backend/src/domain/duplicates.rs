use crate::domain::auth::AuthUser;
use crate::domain::book::{list_row_to_card, BookCard, BookListRow};
use crate::domain::library::{require_view, resolve_permission};
use crate::error::AppResult;
use crate::infra::BookMetadata;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum DuplicateMatchKind {
    Isbn,
    Title,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DuplicateGroup {
    pub kind: DuplicateMatchKind,
    /// 归一化后的 ISBN 或书名，用于展示匹配依据
    pub key: String,
    pub books: Vec<BookCard>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FindDuplicatesResponse {
    pub groups: Vec<DuplicateGroup>,
}

pub fn normalize_isbn(raw: &str) -> Option<String> {
    let digits: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x')
        .collect();
    if digits.len() >= 10 {
        Some(digits.to_ascii_uppercase())
    } else {
        None
    }
}

pub fn normalize_title(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_meaningful_title(normalized: &str) -> bool {
    !normalized.is_empty()
        && normalized != "untitled"
        && normalized.chars().count() >= 2
}

pub async fn find_duplicates(
    state: &AppState,
    user: &AuthUser,
    library_id: &Uuid,
) -> AppResult<FindDuplicatesResponse> {
    let perm = resolve_permission(&state.db, user, library_id).await?;
    require_view(&perm)?;

    let rows: Vec<BookListRow> = sqlx::query_as(
        r#"
        SELECT b.id, b.library_id, b.category, b.book_file_path, b.metadata_file_path, b.cover_path,
               b.metadata, b.sync_status, b.last_read_at, b.uploaded_at, b.updated_at,
               urp.percent, urp.current_page, urp.last_position, urp.updated_at
        FROM books b
        LEFT JOIN user_reading_progress urp
            ON urp.book_id = b.id AND urp.user_id = ?
        WHERE b.library_id = ?
        "#,
    )
    .bind(user.id.to_string())
    .bind(library_id.to_string())
    .fetch_all(&state.db)
    .await?;

    let mut by_isbn: HashMap<String, Vec<BookCard>> = HashMap::new();
    let mut by_title: HashMap<String, Vec<BookCard>> = HashMap::new();

    for row in rows {
        let metadata_json = row.6.clone();
        let card = list_row_to_card(row)?;
        let meta = BookMetadata::from_json(&metadata_json)?;

        if let Some(isbn_key) = normalize_isbn(&meta.isbn) {
            by_isbn.entry(isbn_key).or_default().push(card.clone());
        }

        let title_key = normalize_title(&meta.title);
        if is_meaningful_title(&title_key) {
            by_title.entry(title_key).or_default().push(card);
        }
    }

    let mut groups = Vec::new();

    let mut isbn_keys: Vec<_> = by_isbn.keys().cloned().collect();
    isbn_keys.sort();
    for key in isbn_keys {
        let books = by_isbn.remove(&key).unwrap_or_default();
        if books.len() >= 2 {
            groups.push(DuplicateGroup {
                kind: DuplicateMatchKind::Isbn,
                key,
                books,
            });
        }
    }

    let mut title_keys: Vec<_> = by_title.keys().cloned().collect();
    title_keys.sort();
    for key in title_keys {
        let books = by_title.remove(&key).unwrap_or_default();
        if books.len() >= 2 {
            groups.push(DuplicateGroup {
                kind: DuplicateMatchKind::Title,
                key: books[0].title.clone(),
                books,
            });
        }
    }

    groups.sort_by(|a, b| {
        b.books
            .len()
            .cmp(&a.books.len())
            .then_with(|| a.kind.cmp(&b.kind))
            .then_with(|| a.key.cmp(&b.key))
    });

    Ok(FindDuplicatesResponse { groups })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_isbn_strips_hyphens() {
        assert_eq!(
            normalize_isbn("978-0-306-40615-7").as_deref(),
            Some("9780306406157")
        );
    }

    #[test]
    fn normalize_isbn_rejects_short_values() {
        assert_eq!(normalize_isbn("123"), None);
    }

    #[test]
    fn normalize_title_collapses_whitespace() {
        assert_eq!(normalize_title("  Hello   World  "), "hello world");
    }
}
