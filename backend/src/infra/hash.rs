use crate::error::AppResult;
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn md5_hex(bytes: &[u8]) -> String {
    format!("{:x}", md5::compute(bytes))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub async fn md5_hex_file(path: &Path) -> AppResult<String> {
    let bytes = tokio::fs::read(path).await?;
    Ok(md5_hex(&bytes))
}

pub async fn sha256_hex_file(path: &Path) -> AppResult<String> {
    let bytes = tokio::fs::read(path).await?;
    Ok(sha256_hex(&bytes))
}

/// KOReader partial MD5 读取的 `[start, end)` 字节区间：offset 为 0 与 1024·4^i（i=0..=10），每段 1024 字节
fn partial_md5_ranges(file_size: u64) -> Vec<(u64, u64)> {
    (-1i32..=10)
        .map(|i| if i < 0 { 0 } else { 1024u64 << (2 * i) })
        .take_while(|&start| start < file_size)
        .map(|start| (start, (start + 1024).min(file_size)))
        .collect()
}

/// KOReader 默认的 document 标识（partial MD5，小写 hex）
pub fn partial_md5_hex(bytes: &[u8]) -> String {
    let mut ctx = md5::Context::new();
    for (start, end) in partial_md5_ranges(bytes.len() as u64) {
        ctx.consume(&bytes[start as usize..end as usize]);
    }
    format!("{:x}", ctx.finalize())
}

pub async fn partial_md5_hex_file(path: &Path) -> AppResult<String> {
    use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

    let mut file = tokio::fs::File::open(path).await?;
    let size = file.metadata().await?.len();
    let mut ctx = md5::Context::new();
    let mut buf = vec![0u8; 1024];
    for (start, end) in partial_md5_ranges(size) {
        let chunk = &mut buf[..(end - start) as usize];
        file.seek(SeekFrom::Start(start)).await?;
        file.read_exact(chunk).await?;
        ctx.consume(&*chunk);
    }
    Ok(format!("{:x}", ctx.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn partial_md5_ranges_match_koreader() {
        assert_eq!(partial_md5_ranges(0), vec![]);
        assert_eq!(partial_md5_ranges(500), vec![(0, 500)]);
        assert_eq!(
            partial_md5_ranges(5000),
            vec![(0, 1024), (1024, 2048), (4096, 5000)]
        );
    }

    #[test]
    fn partial_md5_of_small_file_equals_full_md5() {
        assert_eq!(partial_md5_hex(b"abc"), md5_hex(b"abc"));
    }

    #[tokio::test]
    async fn partial_md5_file_matches_bytes() {
        let bytes: Vec<u8> = (0..20_000u32).map(|i| (i % 251) as u8).collect();
        let path = std::env::temp_dir().join(format!("shelfie-pmd5-{}", std::process::id()));
        tokio::fs::write(&path, &bytes).await.unwrap();
        let from_file = partial_md5_hex_file(&path).await.unwrap();
        tokio::fs::remove_file(&path).await.ok();
        assert_eq!(from_file, partial_md5_hex(&bytes));
        assert_ne!(from_file, md5_hex(&bytes));
    }
}
