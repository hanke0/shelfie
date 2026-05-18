use crate::error::AppResult;
use std::path::Path;

pub fn md5_hex(bytes: &[u8]) -> String {
    format!("{:x}", md5::compute(bytes))
}

pub async fn md5_hex_file(path: &Path) -> AppResult<String> {
    let bytes = tokio::fs::read(path).await?;
    Ok(md5_hex(&bytes))
}
