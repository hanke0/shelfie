use super::thumbnail;
use crate::error::{AppError, AppResult};
use crate::infra::metadata::BookMetadata;
use crate::infra::safe_name;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const BOOK_EXTENSIONS: &[&str] = &["pdf", "epub", "mobi"];
const COVER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];

/// 磁盘上是否存在已保存的封面文件
pub fn has_stored_cover(cover_path: &Path) -> bool {
    !cover_path.as_os_str().is_empty() && cover_path.is_file()
}

/// 图书馆根目录：`{data_root}/{图书馆名}`（名称经安全化处理，冲突时加 `_2` 等后缀）
pub fn library_root_for_name(data_root: &Path, library_name: &str) -> AppResult<PathBuf> {
    safe_name::allocate_library_root(data_root, library_name)
}

/// 分类目录：该分类下所有图书文件平铺存放
pub fn category_dir(library_root: &Path, category: &str) -> AppResult<PathBuf> {
    safe_name::validate_category(category)?;
    let seg = safe_name::sanitize_path_segment(category);
    if seg.is_empty() {
        return Err(AppError::BadRequest("category cannot be empty".into()));
    }
    Ok(library_root.join(seg))
}

/// 图书文件名基底：`{书名}_{作者}`（元数据可含任意非控制字符，磁盘名经消毒）
pub fn book_base_name(title: &str, author: &str) -> String {
    let title = safe_name::sanitize_path_segment(title);
    let author = safe_name::sanitize_path_segment(author);
    let base = match (title.is_empty(), author.is_empty()) {
        (true, true) => "untitled".to_string(),
        (true, false) => author,
        (false, true) => title,
        (false, false) => format!("{title}_{author}"),
    };
    let base = safe_name::avoid_windows_reserved_base(&base);
    if base.is_empty() {
        return "untitled".to_string();
    }
    truncate_base(&base, 180)
}

fn truncate_base(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }
    s.chars().take(max_len).collect()
}

pub fn validate_book_path_fields(title: &str, author: &str, category: &str) -> AppResult<()> {
    safe_name::validate_book_title(title)?;
    safe_name::validate_book_author(author)?;
    safe_name::validate_category(category)?;
    Ok(())
}

/// 若目录中已存在同名基底文件，追加 `_2`、`_3` …
pub fn ensure_unique_base(dir: &Path, base: &str) -> String {
    if !dir.exists() {
        return base.to_string();
    }
    let mut candidate = base.to_string();
    let mut n = 2u32;
    while base_name_exists_in_dir(dir, &candidate) {
        candidate = format!("{base}_{n}");
        n += 1;
    }
    candidate
}

fn base_name_exists_in_dir(dir: &Path, base: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem == base {
                    return true;
                }
            }
        }
    }
    false
}

pub fn validate_book_extension(ext: &str) -> AppResult<()> {
    let ext = ext.to_lowercase();
    if BOOK_EXTENSIONS.contains(&ext.as_str()) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "Unsupported book format. Allowed: {}",
            BOOK_EXTENSIONS.join(", ")
        )))
    }
}

pub fn validate_cover_extension(ext: &str) -> AppResult<()> {
    let ext = ext.to_lowercase();
    if COVER_EXTENSIONS.contains(&ext.as_str()) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "Unsupported cover format. Allowed: {}",
            COVER_EXTENSIONS.join(", ")
        )))
    }
}

pub async fn ensure_dir(path: &Path) -> AppResult<()> {
    tokio::fs::create_dir_all(path).await?;
    Ok(())
}

pub async fn write_book_file(
    dir: &Path,
    base: &str,
    ext: &str,
    bytes: &[u8],
) -> AppResult<PathBuf> {
    ensure_dir(dir).await?;
    let path = dir.join(format!("{}.{}", base, ext.to_lowercase()));
    tokio::fs::write(&path, bytes).await?;
    Ok(path)
}

pub async fn write_cover_file(
    dir: &Path,
    base: &str,
    ext: &str,
    bytes: &[u8],
) -> AppResult<PathBuf> {
    ensure_dir(dir).await?;
    let path = dir.join(format!("{}.{}", base, ext.to_lowercase()));
    tokio::fs::write(&path, bytes).await?;
    Ok(path)
}

pub async fn write_metadata_file(
    dir: &Path,
    base: &str,
    metadata: &BookMetadata,
) -> AppResult<PathBuf> {
    ensure_dir(dir).await?;
    let path = dir.join(format!("{base}.json"));
    tokio::fs::write(&path, metadata.to_json()?).await?;
    Ok(path)
}

pub async fn read_metadata_file(path: &Path) -> AppResult<BookMetadata> {
    let content = tokio::fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&content)?)
}

/// 删除一本图书的三个文件（不删除分类目录）
pub async fn remove_book_files(book: &Path, metadata: &Path, cover: &Path) -> AppResult<()> {
    for path in [book, metadata, cover] {
        if path.is_file() {
            tokio::fs::remove_file(path).await.ok();
        }
    }
    if !cover.as_os_str().is_empty() {
        thumbnail::remove_for_cover(cover).await;
    }
    Ok(())
}

pub fn extension_from_filename(name: &str) -> Option<String> {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
}

/// 当浏览器未提供文件名扩展名时，根据 MIME 推断
pub fn extension_from_mime(mime: &str) -> Option<String> {
    let mime = mime.to_lowercase();
    if mime.contains("pdf") {
        return Some("pdf".into());
    }
    if mime.contains("epub") {
        return Some("epub".into());
    }
    if mime.contains("mobi") {
        return Some("mobi".into());
    }
    if mime.contains("jpeg") || mime.contains("jpg") {
        return Some("jpg".into());
    }
    if mime.contains("png") {
        return Some("png".into());
    }
    None
}

pub fn resolve_book_extension(
    filename: Option<&str>,
    content_type: Option<&str>,
) -> Option<String> {
    filename
        .and_then(extension_from_filename)
        .or_else(|| content_type.and_then(extension_from_mime))
}

pub fn resolve_cover_extension(
    filename: Option<&str>,
    content_type: Option<&str>,
) -> Option<String> {
    filename
        .and_then(extension_from_filename)
        .or_else(|| content_type.and_then(extension_from_mime))
}

pub fn is_book_extension(ext: &str) -> bool {
    BOOK_EXTENSIONS.contains(&ext.to_lowercase().as_str())
}

pub fn file_stem(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

pub fn find_metadata_in_dir(dir: &Path, book_stem: Option<&str>) -> Option<PathBuf> {
    if let Some(stem) = book_stem {
        let named = dir.join(format!("{stem}.json"));
        if named.is_file() {
            return Some(named);
        }
    }
    let legacy = dir.join("metadata.json");
    if legacy.is_file() {
        return Some(legacy);
    }
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .find_map(|entry| {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|x| x.to_str()) == Some("json") {
                return Some(path);
            }
            None
        })
}

pub fn find_cover_in_dir(dir: &Path, book_stem: Option<&str>) -> Option<PathBuf> {
    if let Some(stem) = book_stem {
        for ext in COVER_EXTENSIONS {
            let p = dir.join(format!("{stem}.{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .find_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let name = path.file_name().and_then(|n| n.to_str())?;
            if name.starts_with("cover.") {
                return Some(path);
            }
            if let Some(stem) = book_stem {
                if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if file_stem == stem {
                        if let Some(ext) = path.extension().and_then(|x| x.to_str()) {
                            if COVER_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                                return Some(path);
                            }
                        }
                    }
                }
            }
            None
        })
}

/// 标题/作者变更时重命名目录内图书、metadata、封面（保持扩展名）
pub async fn rename_book_assets(
    dir: &Path,
    metadata: &BookMetadata,
    book_path: &Path,
    metadata_path: &Path,
    cover_path: &Path,
) -> AppResult<(PathBuf, PathBuf, PathBuf)> {
    let book_ext = book_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| AppError::Internal("Book file has no extension".into()))?;
    let new_base = ensure_unique_base_for_rename(
        dir,
        &book_base_name(&metadata.title, &metadata.author),
        book_path,
    );

    let new_book = dir.join(format!("{new_base}.{book_ext}"));
    let new_meta = dir.join(format!("{new_base}.json"));

    if book_path != new_book {
        if new_book.exists() {
            tokio::fs::remove_file(&new_book).await.ok();
        }
        tokio::fs::rename(book_path, &new_book).await?;
    }
    if metadata_path != new_meta {
        if new_meta.exists() {
            tokio::fs::remove_file(&new_meta).await.ok();
        }
        tokio::fs::rename(metadata_path, &new_meta).await?;
    }

    if !has_stored_cover(cover_path) {
        return Ok((new_book, new_meta, PathBuf::new()));
    }

    let cover_ext = cover_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| AppError::Internal("Cover file has no extension".into()))?;
    let new_cover = dir.join(format!("{new_base}.{cover_ext}"));

    if cover_path != new_cover {
        if new_cover.exists() {
            tokio::fs::remove_file(&new_cover).await.ok();
        }
        thumbnail::relocate_for_cover(cover_path, &new_cover).await;
        tokio::fs::rename(cover_path, &new_cover).await?;
    }

    Ok((new_book, new_meta, new_cover))
}

fn ensure_unique_base_for_rename(dir: &Path, base: &str, current_book: &Path) -> String {
    let current_stem = file_stem(current_book).unwrap_or_default();
    if base == current_stem {
        return base.to_string();
    }
    let mut candidate = base.to_string();
    let mut n = 2u32;
    while base_name_exists_in_dir(dir, &candidate) && candidate != current_stem {
        candidate = format!("{base}_{n}");
        n += 1;
    }
    candidate
}

#[derive(Debug, Clone)]
pub struct ScannedBookFile {
    pub category: String,
    pub book_file: PathBuf,
}

/// 列出图书馆根目录下的分类子文件夹名
pub async fn list_library_categories(library_root: &Path) -> AppResult<Vec<String>> {
    let mut names = Vec::new();
    if !library_root.exists() {
        return Ok(names);
    }
    let mut entries = tokio::fs::read_dir(library_root).await?;
    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        names.push(name);
    }
    names.sort();
    Ok(names)
}

/// 目录下是否存在任意文件（含子目录）
pub async fn dir_has_any_files(dir: &Path) -> AppResult<bool> {
    if !dir.exists() {
        return Ok(false);
    }
    let path = dir.to_path_buf();
    tokio::task::spawn_blocking(move || dir_has_any_files_sync(&path))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

fn dir_has_any_files_sync(dir: &Path) -> AppResult<bool> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            return Ok(true);
        }
        if path.is_dir() && dir_has_any_files_sync(&path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// 图书馆目录下是否存在任意文件（含子目录；空目录或仅空分类目录视为无文件）
pub async fn library_has_any_files(library_root: &Path) -> AppResult<bool> {
    if !library_root.exists() {
        return Ok(false);
    }
    let root = library_root.to_path_buf();
    tokio::task::spawn_blocking(move || library_has_any_files_sync(&root))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}

fn library_has_any_files_sync(dir: &Path) -> AppResult<bool> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            return Ok(true);
        }
        if path.is_dir() && library_has_any_files_sync(&path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// 扫描图书馆：每个分类目录下平铺的图书文件
pub async fn scan_library_books(library_root: &Path) -> AppResult<Vec<ScannedBookFile>> {
    let mut results = Vec::new();
    if !library_root.exists() {
        return Ok(results);
    }

    let mut categories = tokio::fs::read_dir(library_root).await?;
    while let Some(cat_entry) = categories.next_entry().await? {
        if !cat_entry.file_type().await?.is_dir() {
            continue;
        }
        let category = cat_entry.file_name().to_string_lossy().to_string();
        let cat_path = cat_entry.path();

        let mut entries = tokio::fs::read_dir(&cat_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if !entry.file_type().await?.is_file() {
                continue;
            }
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if is_book_extension(ext) {
                    results.push(ScannedBookFile {
                        category: category.clone(),
                        book_file: path,
                    });
                }
            }
        }
    }

    Ok(results)
}

/// 删除分类目录内无对应图书文件的 metadata、封面与缩略图缓存。
pub async fn cleanup_dangling_files(
    library_root: &Path,
    scanned: &[ScannedBookFile],
) -> AppResult<Vec<PathBuf>> {
    let mut stems_by_category: HashMap<String, HashSet<String>> = HashMap::new();
    for entry in scanned {
        if let Some(stem) = file_stem(&entry.book_file) {
            stems_by_category
                .entry(entry.category.clone())
                .or_default()
                .insert(stem);
        }
    }

    let mut removed = Vec::new();
    if !library_root.exists() {
        return Ok(removed);
    }

    let mut categories = tokio::fs::read_dir(library_root).await?;
    while let Some(cat_entry) = categories.next_entry().await? {
        if !cat_entry.file_type().await?.is_dir() {
            continue;
        }
        let category = cat_entry.file_name().to_string_lossy().to_string();
        if category.starts_with('.') {
            continue;
        }
        let cat_path = cat_entry.path();
        let valid_stems = stems_by_category.get(&category);
        let book_count = valid_stems.map(|s| s.len()).unwrap_or(0);

        let mut entries = tokio::fs::read_dir(&cat_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if !entry.file_type().await?.is_file() {
                continue;
            }
            let path = entry.path();
            if is_owned_book_asset(&path, valid_stems, book_count) {
                continue;
            }
            if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(is_book_extension)
            {
                continue;
            }
            if is_cover_path(&path) {
                thumbnail::remove_for_cover(&path).await;
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(base) = name.strip_suffix(".thumb.jpg") {
                    for ext in COVER_EXTENSIONS {
                        thumbnail::remove_for_cover(&cat_path.join(format!("{base}.{ext}"))).await;
                    }
                }
            }
            if tokio::fs::remove_file(&path).await.is_ok() {
                removed.push(path);
            }
        }
    }

    Ok(removed)
}

fn is_owned_book_asset(
    path: &Path,
    valid_stems: Option<&HashSet<String>>,
    book_count: usize,
) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some(valid) = valid_stems else {
        return false;
    };

    if name.ends_with(".thumb.jpg") {
        return name
            .strip_suffix(".thumb.jpg")
            .is_some_and(|base| valid.contains(base));
    }

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if ext == "json" {
            if name == "metadata.json" {
                return book_count == 1 && valid.len() == 1;
            }
            return path
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|stem| valid.contains(stem));
        }
        if COVER_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
            if name.starts_with("cover.") {
                return book_count == 1 && valid.len() == 1;
            }
            return path
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|stem| valid.contains(stem));
        }
    }

    false
}

fn is_cover_path(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if name.ends_with(".thumb.jpg") {
        return false;
    }
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| COVER_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_name_from_title_author() {
        assert_eq!(book_base_name("三体", "刘慈欣"), "三体_刘慈欣");
        assert_eq!(
            book_base_name("Hello World", "Author"),
            "Hello_World_Author"
        );
        assert_eq!(book_base_name("A/B", "C:D"), "A_B_C_D");
        assert_eq!(
            book_base_name("三体—续", "作者@example"),
            "三体—续_作者@example"
        );
    }

    #[test]
    fn category_dir_preserves_cjk() {
        let root = std::path::PathBuf::from("/tmp/lib");
        let dir = category_dir(&root, "科幻").unwrap();
        assert!(dir.ends_with("科幻"));
    }

    #[tokio::test]
    async fn library_has_any_files_detects_nested_file() {
        let tmp = std::env::temp_dir().join(format!(
            "shelfie_fs_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let cat = tmp.join("科幻");
        std::fs::create_dir_all(&cat).unwrap();
        assert!(!library_has_any_files(&tmp).await.unwrap());
        std::fs::write(cat.join("book.pdf"), b"x").unwrap();
        assert!(library_has_any_files(&tmp).await.unwrap());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn cleanup_dangling_files_removes_orphan_sidecars() {
        let tmp = std::env::temp_dir().join(format!(
            "shelfie_cleanup_test_{}",
            std::process::id()
        ));
        let cat = tmp.join("fiction");
        std::fs::create_dir_all(&cat).unwrap();
        std::fs::write(cat.join("book.pdf"), b"pdf").unwrap();
        std::fs::write(cat.join("book.json"), b"{}").unwrap();
        std::fs::write(cat.join("orphan.json"), b"{}").unwrap();
        std::fs::write(cat.join("orphan.jpg"), b"jpg").unwrap();
        std::fs::write(cat.join("orphan.thumb.jpg"), b"thumb").unwrap();

        let scanned = vec![ScannedBookFile {
            category: "fiction".to_string(),
            book_file: cat.join("book.pdf"),
        }];
        let removed = cleanup_dangling_files(&tmp, &scanned).await.unwrap();
        let removed_names: HashSet<_> = removed
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()))
            .collect();

        assert!(cat.join("book.pdf").is_file());
        assert!(cat.join("book.json").is_file());
        assert!(!cat.join("orphan.json").exists());
        assert!(!cat.join("orphan.jpg").exists());
        assert!(!cat.join("orphan.thumb.jpg").exists());
        assert!(removed_names.contains("orphan.json"));
        assert!(removed_names.contains("orphan.jpg"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
