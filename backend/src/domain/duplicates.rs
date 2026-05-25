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
    Md5,
    Sha256,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DuplicateGroup {
    pub kind: DuplicateMatchKind,
    /// 归一化后的 ISBN、书名或文件哈希，用于展示匹配依据
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
    !normalized.is_empty() && normalized != "untitled" && normalized.chars().count() >= 2
}

pub fn normalize_md5(raw: &str) -> Option<String> {
    let s = raw.trim().to_lowercase();
    if s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(s)
    } else {
        None
    }
}

pub fn normalize_sha256(raw: &str) -> Option<String> {
    let s = raw.trim().to_lowercase();
    if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(s)
    } else {
        None
    }
}

fn push_groups(
    groups: &mut Vec<DuplicateGroup>,
    kind: DuplicateMatchKind,
    mut map: HashMap<String, Vec<BookCard>>,
    display_key: impl Fn(&str, &Vec<BookCard>) -> String,
) {
    let mut keys: Vec<_> = map.keys().cloned().collect();
    keys.sort();
    for key in keys {
        let books = map.remove(&key).unwrap_or_default();
        if books.len() >= 2 {
            groups.push(DuplicateGroup {
                kind,
                key: display_key(&key, &books),
                books,
            });
        }
    }
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
    let mut by_md5: HashMap<String, Vec<BookCard>> = HashMap::new();
    let mut by_sha256: HashMap<String, Vec<BookCard>> = HashMap::new();

    for row in rows {
        let metadata_json = row.6.clone();
        let card = list_row_to_card(row)?;
        let meta = BookMetadata::from_json(&metadata_json)?;

        if let Some(isbn_key) = normalize_isbn(&meta.isbn) {
            by_isbn.entry(isbn_key).or_default().push(card.clone());
        }

        if let Some(md5) = meta.file_md5.as_deref().and_then(normalize_md5) {
            by_md5.entry(md5).or_default().push(card.clone());
        }

        if let Some(sha) = meta.file_sha256.as_deref().and_then(normalize_sha256) {
            by_sha256.entry(sha).or_default().push(card.clone());
        }

        let title_key = normalize_title(&meta.title);
        if is_meaningful_title(&title_key) {
            by_title.entry(title_key).or_default().push(card);
        }
    }

    let mut groups = Vec::new();

    push_groups(&mut groups, DuplicateMatchKind::Isbn, by_isbn, |key, _| {
        key.to_string()
    });
    push_groups(
        &mut groups,
        DuplicateMatchKind::Title,
        by_title,
        |_, books| books[0].title.clone(),
    );
    push_groups(&mut groups, DuplicateMatchKind::Md5, by_md5, |key, _| {
        key.to_string()
    });
    push_groups(
        &mut groups,
        DuplicateMatchKind::Sha256,
        by_sha256,
        |key, _| key.to_string(),
    );

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

    #[test]
    fn normalize_md5_requires_32_hex_chars() {
        assert!(normalize_md5("a".repeat(32).as_str()).is_some());
        assert!(normalize_md5("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_none());
    }

    #[test]
    fn normalize_sha256_requires_64_hex_chars() {
        assert!(normalize_sha256(&"a".repeat(64)).is_some());
        assert!(normalize_sha256(&"a".repeat(32)).is_none());
    }
}
