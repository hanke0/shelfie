use crate::error::AppResult;
use sha2::{Digest, Sha256};
use std::path::Path;

pub fn md5_hex(bytes: &[u8]) -> String {
    format!("{:x}", md5::compute(bytes))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub async fn md5_hex_file(path: &Path) -> AppResult<String> {
    let bytes = tokio::fs::read(path).await?;
    Ok(md5_hex(&bytes))
}

pub async fn sha256_hex_file(path: &Path) -> AppResult<String> {
    let bytes = tokio::fs::read(path).await?;
    Ok(sha256_hex(&bytes))
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
}
