use crate::error::{AppError, AppResult};
use image::ImageFormat;
use std::io::Cursor;
use std::path::{Path, PathBuf};

const THUMB_MAX_EDGE: u32 = 320;

pub fn thumbnail_path(cover_path: &Path) -> PathBuf {
    let parent = cover_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = cover_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("cover");
    parent.join(format!("{stem}.thumb.jpg"))
}

pub async fn read_or_create_thumbnail(cover_path: &Path) -> AppResult<Vec<u8>> {
    let cover_path = cover_path.to_path_buf();
    tokio::task::spawn_blocking(move || read_or_create_thumbnail_sync(&cover_path))
        .await
        .map_err(|e| AppError::Internal(format!("thumbnail task failed: {e}")))?
}

fn read_or_create_thumbnail_sync(cover_path: &Path) -> AppResult<Vec<u8>> {
    if !cover_path.is_file() {
        return Err(AppError::NotFound("Cover not found".into()));
    }

    let thumb_path = thumbnail_path(cover_path);
    if thumb_path.is_file() && is_thumbnail_fresh(cover_path, &thumb_path)? {
        return std::fs::read(&thumb_path).map_err(Into::into);
    }

    let bytes = std::fs::read(cover_path)?;
    let img = image::load_from_memory(&bytes)
        .map_err(|e| AppError::Internal(format!("invalid cover image: {e}")))?;
    let thumb = img.thumbnail(THUMB_MAX_EDGE, THUMB_MAX_EDGE);

    let mut out = Cursor::new(Vec::new());
    thumb
        .write_to(&mut out, ImageFormat::Jpeg)
        .map_err(|e| AppError::Internal(format!("encode thumbnail failed: {e}")))?;
    let encoded = out.into_inner();

    if let Some(parent) = thumb_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&thumb_path, &encoded)?;

    Ok(encoded)
}

fn is_thumbnail_fresh(cover_path: &Path, thumb_path: &Path) -> AppResult<bool> {
    let cover_meta = std::fs::metadata(cover_path)?;
    let thumb_meta = std::fs::metadata(thumb_path)?;
    Ok(thumb_meta.modified()? >= cover_meta.modified()?)
}

pub async fn remove_for_cover(cover_path: &Path) {
    let thumb_path = thumbnail_path(cover_path);
    if thumb_path.is_file() {
        tokio::fs::remove_file(thumb_path).await.ok();
    }
}

/// Move cached thumbnail when the cover file path changes (category move or rename).
pub async fn relocate_for_cover(old_cover: &Path, new_cover: &Path) {
    if old_cover == new_cover {
        return;
    }
    let old_thumb = thumbnail_path(old_cover);
    let new_thumb = thumbnail_path(new_cover);
    if old_thumb == new_thumb {
        return;
    }
    remove_for_cover(new_cover).await;
    if !old_thumb.is_file() {
        return;
    }
    if let Some(parent) = new_thumb.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::rename(&old_thumb, &new_thumb).await.ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    #[test]
    fn thumbnail_path_uses_cover_stem() {
        let cover = Path::new("/data/lib/fiction/book_title.jpg");
        assert_eq!(
            thumbnail_path(cover),
            Path::new("/data/lib/fiction/book_title.thumb.jpg")
        );
    }

    #[test]
    fn creates_smaller_jpeg_thumbnail() {
        let dir = std::env::temp_dir().join(format!("shelfie-thumb-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cover = dir.join("sample.png");
        let img = ImageBuffer::from_fn(800, 1200, |x, y| {
            Rgb([(x % 255) as u8, (y % 255) as u8, ((x + y) % 255) as u8])
        });
        img.save(&cover).unwrap();

        let thumb = read_or_create_thumbnail_sync(&cover).unwrap();
        assert!(thumb.len() < std::fs::read(&cover).unwrap().len());

        let decoded = image::load_from_memory(&thumb).unwrap();
        assert!(decoded.width() <= THUMB_MAX_EDGE);
        assert!(decoded.height() <= THUMB_MAX_EDGE);
        assert!(thumbnail_path(&cover).is_file());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn relocate_for_cover_moves_thumb_with_cover() {
        let dir = std::env::temp_dir().join(format!(
            "shelfie-thumb-relocate-{}",
            std::process::id()
        ));
        let old_cat = dir.join("fiction");
        let new_cat = dir.join("sci-fi");
        std::fs::create_dir_all(&old_cat).unwrap();
        std::fs::create_dir_all(&new_cat).unwrap();

        let cover = old_cat.join("book.jpg");
        std::fs::write(&cover, b"not-a-real-jpeg").unwrap();
        let thumb = thumbnail_path(&cover);
        std::fs::write(&thumb, b"cached-thumb").unwrap();

        let new_cover = new_cat.join("book.jpg");
        relocate_for_cover(&cover, &new_cover).await;

        assert!(!thumb.is_file());
        assert_eq!(
            std::fs::read(thumbnail_path(&new_cover)).unwrap(),
            b"cached-thumb"
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
