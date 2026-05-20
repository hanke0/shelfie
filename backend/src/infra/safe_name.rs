use crate::error::{AppError, AppResult};
use std::path::Path;
use std::sync::LazyLock;

const MAX_LIBRARY_NAME_LEN: usize = 80;
const MAX_BOOK_FIELD_LEN: usize = 120;
const MAX_CATEGORY_LEN: usize = 64;

const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// 与 `[\p{L}\p{N}\s\-_.,()（）【】《》「」『』·':;+&]` 等价的字符级校验（图书馆名、分类）
static SEGMENT_CHAR_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^[\p{L}\p{N}\-_.,()（）【】《》「」『』·':;+&]$")
        .expect("segment char regex")
});

/// 磁盘文件名中必须剔除的字符（Windows / POSIX）
const FILENAME_UNSAFE_CHARS: &str = "/\\:*?\"<>|";

fn is_allowed_segment_char(c: char) -> bool {
    if c.is_control() || FILENAME_UNSAFE_CHARS.contains(c) {
        return false;
    }
    if c.is_whitespace() {
        return true;
    }
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    SEGMENT_CHAR_RE.is_match(s)
}

fn is_valid_slug_char(c: char) -> bool {
    matches!(c, 'a'..='z' | '0'..='9' | '-' | '_')
}

fn has_control_char(s: &str) -> bool {
    s.chars().any(|c| c.is_control())
}

fn validate_book_metadata_text(value: &str, field: &str, allow_empty: bool) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        if allow_empty {
            return Ok(());
        }
        return Err(AppError::BadRequest(format!("{field} cannot be empty")));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(AppError::BadRequest(format!("{field} cannot be empty")));
    }
    if trimmed.chars().count() > MAX_BOOK_FIELD_LEN {
        return Err(AppError::BadRequest(format!(
            "{field} must be at most {MAX_BOOK_FIELD_LEN} characters"
        )));
    }
    if has_control_char(trimmed) {
        return Err(AppError::BadRequest(format!(
            "{field} contains invalid control characters"
        )));
    }
    Ok(())
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
        return Err(AppError::BadRequest("slug must be 1–64 characters".into()));
    }
    let mut chars = trimmed.chars();
    let first = chars
        .next()
        .ok_or_else(|| AppError::BadRequest("slug cannot be empty".into()))?;
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

/// 书名元数据：允许除控制字符外的任意 Unicode（含 `@`、`—` 等）
pub fn validate_book_title(title: &str) -> AppResult<()> {
    validate_book_metadata_text(title, "title", false)
}

/// 作者元数据：允许为空；非空时同书名规则
pub fn validate_book_author(author: &str) -> AppResult<()> {
    validate_book_metadata_text(author, "author", true)
}

pub fn validate_category(category: &str) -> AppResult<()> {
    validate_segment(category, "category", MAX_CATEGORY_LEN)
}

/// 将用户输入转为安全的文件名/路径段（剔除路径非法字符，空白与非法符号变为 `_`）
pub fn sanitize_path_segment(s: &str) -> String {
    let mut out = String::new();
    let mut prev_underscore = false;

    for c in s.trim().chars() {
        if c.is_control() || FILENAME_UNSAFE_CHARS.contains(c) {
            if !prev_underscore && !out.is_empty() {
                out.push('_');
                prev_underscore = true;
            }
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

/// 避免 Windows 保留设备名作为文件基底
pub fn avoid_windows_reserved_base(base: &str) -> String {
    if base.is_empty() {
        return String::new();
    }
    let upper = base.to_ascii_uppercase();
    let stem = upper.split('.').next().unwrap_or(&upper);
    if WINDOWS_RESERVED.contains(&stem) {
        return "untitled".to_string();
    }
    base.to_string()
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
    fn book_title_allows_special_chars_in_metadata() {
        assert!(validate_book_title("hello@world").is_ok());
        assert!(validate_book_title("三体—续").is_ok());
        assert!(validate_book_title("A&B: Vol.1").is_ok());
        assert!(validate_book_title("ः").is_ok());
    }

    #[test]
    fn book_title_rejects_control_chars() {
        assert!(validate_book_title("foo\u{0000}bar").is_err());
        assert!(validate_book_title("line\nbreak").is_err());
    }

    #[test]
    fn book_title_still_rejects_path_like_empty() {
        assert!(validate_book_title("   ").is_err());
    }

    #[test]
    fn sanitize_strips_unsafe_path_chars() {
        assert_eq!(sanitize_path_segment("三体"), "三体");
        assert_eq!(sanitize_path_segment("Hello World"), "Hello_World");
        assert_eq!(sanitize_path_segment("A/B:C"), "A_B_C");
    }

    #[test]
    fn sanitize_avoids_windows_reserved() {
        assert_eq!(avoid_windows_reserved_base("CON"), "untitled");
        assert_eq!(avoid_windows_reserved_base("三体"), "三体");
    }

    #[test]
    fn category_still_rejects_path_separators() {
        assert!(validate_category("foo/bar").is_err());
    }
}
