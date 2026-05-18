use crate::error::{AppError, AppResult};
use std::path::Path;

const MAX_LIBRARY_NAME_LEN: usize = 80;
const MAX_BOOK_FIELD_LEN: usize = 120;
const MAX_CATEGORY_LEN: usize = 64;

const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// 与 `[\p{L}\p{N}\s\-_.,()（）【】《》「」『』·':;+&]` 等价的字符级校验（支持 i18n）
fn is_allowed_segment_char(c: char) -> bool {
    if c.is_control() || "/\\:*?\"<>|".contains(c) {
        return false;
    }
    c.is_alphanumeric()
        || c.is_whitespace()
        || matches!(
            c,
            '-' | '_'
                | '.'
                | ','
                | '('
                | ')'
                | '（'
                | '）'
                | '【'
                | '】'
                | '《'
                | '》'
                | '「'
                | '」'
                | '『'
                | '』'
                | '·'
                | '\''
                | ':'
                | ';'
                | '+'
                | '&'
        )
}

fn is_valid_slug_char(c: char) -> bool {
    matches!(c, 'a'..='z' | '0'..='9' | '-' | '_')
}

fn validate_segment(value: &str, field: &str, max_len: usize) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return Err(AppError::BadRequest(format!("{field} cannot be empty")));
    }
    if trimmed.chars().count() > max_len {
        return Err(AppError::BadRequest(format!(
            "{field} must be at most {max_len} characters"
        )));
    }
    if !trimmed.chars().all(is_allowed_segment_char) {
        return Err(AppError::BadRequest(format!(
            "{field} contains invalid characters (use letters, numbers, common punctuation; no path separators)"
        )));
    }
    let upper = trimmed.to_ascii_uppercase();
    if WINDOWS_RESERVED.contains(&upper.as_str()) {
        return Err(AppError::BadRequest(format!("{field} is a reserved name")));
    }
    Ok(())
}

pub fn validate_library_name(name: &str) -> AppResult<()> {
    validate_segment(name, "library name", MAX_LIBRARY_NAME_LEN)
}

pub fn validate_slug(slug: &str) -> AppResult<()> {
    let trimmed = slug.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err(AppError::BadRequest(
            "slug must be 1–64 characters".into(),
        ));
    }
    let mut chars = trimmed.chars();
    let first = chars.next().ok_or_else(|| AppError::BadRequest("slug cannot be empty".into()))?;
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(AppError::BadRequest(
            "slug must start with a lowercase letter or digit".into(),
        ));
    }
    if !chars.all(is_valid_slug_char) {
        return Err(AppError::BadRequest(
            "slug must contain only lowercase letters, digits, hyphen, underscore".into(),
        ));
    }
    Ok(())
}

pub fn validate_book_title(title: &str) -> AppResult<()> {
    validate_segment(title, "title", MAX_BOOK_FIELD_LEN)
}

pub fn validate_book_author(author: &str) -> AppResult<()> {
    let trimmed = author.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    validate_segment(trimmed, "author", MAX_BOOK_FIELD_LEN)
}

pub fn validate_category(category: &str) -> AppResult<()> {
    validate_segment(category, "category", MAX_CATEGORY_LEN)
}

/// 将用户输入转为安全的路径段，保留 i18n 字母与数字
pub fn sanitize_path_segment(s: &str) -> String {
    let mut out = String::new();
    let mut prev_underscore = false;

    for c in s.trim().chars() {
        if c.is_control() || "/\\:*?\"<>|".contains(c) {
            continue;
        }
        if c.is_whitespace() {
            if !prev_underscore && !out.is_empty() {
                out.push('_');
                prev_underscore = true;
            }
            continue;
        }
        prev_underscore = false;
        out.push(c);
    }

    let trimmed = out.trim_matches('_').trim_matches('.').to_string();
    if trimmed.is_empty() {
        return String::new();
    }
    truncate_chars(&trimmed, MAX_BOOK_FIELD_LEN)
}

fn truncate_chars(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }
    s.chars().take(max_len).collect()
}

/// 图书馆磁盘目录名（与图书馆名称一致，必要时加后缀保证唯一）
pub fn library_dir_name(name: &str) -> AppResult<String> {
    validate_library_name(name)?;
    let base = sanitize_path_segment(name);
    if base.is_empty() {
        return Err(AppError::BadRequest(
            "library name cannot be empty after sanitization".into(),
        ));
    }
    Ok(base)
}

pub fn allocate_library_root(data_root: &Path, name: &str) -> AppResult<std::path::PathBuf> {
    let base = library_dir_name(name)?;
    let unique = ensure_unique_dir_under(data_root, &base);
    Ok(data_root.join(unique))
}

fn ensure_unique_dir_under(parent: &Path, base: &str) -> String {
    if !parent.join(base).exists() {
        return base.to_string();
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{base}_{n}");
        if !parent.join(&candidate).exists() {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_cjk_library_name() {
        assert!(validate_library_name("默认图书馆").is_ok());
        assert!(validate_library_name("科幻").is_ok());
    }

    #[test]
    fn rejects_path_separators() {
        assert!(validate_book_title("foo/bar").is_err());
    }

    #[test]
    fn sanitize_preserves_cjk() {
        assert_eq!(sanitize_path_segment("三体"), "三体");
        assert_eq!(sanitize_path_segment("Hello World"), "Hello_World");
    }
}
