use crate::infra::hash;
use serde::{Deserialize, Serialize};
use std::path::Path;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ReadingProgress {
    pub current_page: Option<i32>,
    pub percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_position: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct BookMetadata {
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub language: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub translator: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub publisher: String,
    /// 出版日期，格式 `YYYY-MM`（如 2024-02）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub publish_date: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub isbn: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_count: Option<i32>,
    /// 评分 1–5；未评分时不序列化
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(minimum = 1, maximum = 5)]
    pub rating: Option<u8>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub original_title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub series: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading_progress: Option<ReadingProgress>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// 图书文件 MD5（小写 hex），用于 KOReader document 关联
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_md5: Option<String>,
    /// 图书文件 SHA-256（小写 hex）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_sha256: Option<String>,
}

impl BookMetadata {
    pub fn from_json(s: &str) -> crate::error::AppResult<Self> {
        Ok(serde_json::from_str(s)?)
    }

    pub fn to_json(&self) -> crate::error::AppResult<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 规范化出版日期；空字符串合法；否则须为 `YYYY-MM`
    pub fn normalize_publish_date(s: &str) -> crate::error::AppResult<String> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(String::new());
        }
        let valid = s.len() == 7
            && s.as_bytes().get(4) == Some(&b'-')
            && s[..4].chars().all(|c| c.is_ascii_digit())
            && s[5..].chars().all(|c| c.is_ascii_digit())
            && matches!(s[5..].parse::<u32>(), Ok(m) if (1..=12).contains(&m));
        if valid {
            Ok(s.to_string())
        } else {
            Err(crate::error::AppError::BadRequest(
                "publish_date must be YYYY-MM (e.g. 2024-02)".into(),
            ))
        }
    }

    pub fn normalize_rating(rating: Option<u8>) -> crate::error::AppResult<Option<u8>> {
        match rating {
            None => Ok(None),
            Some(0) => Ok(None),
            Some(n @ 1..=5) => Ok(Some(n)),
            Some(_) => Err(crate::error::AppError::BadRequest(
                "rating must be an integer from 1 to 5".into(),
            )),
        }
    }

    pub fn normalize_fields(&mut self) -> crate::error::AppResult<()> {
        self.publish_date = Self::normalize_publish_date(&self.publish_date)?;
        self.rating = Self::normalize_rating(self.rating)?;
        Ok(())
    }

    /// 根据内存中的图书文件内容设置 MD5 / SHA-256
    pub fn set_book_bytes_hashes(&mut self, bytes: &[u8]) {
        self.file_md5 = Some(hash::md5_hex(bytes));
        self.file_sha256 = Some(hash::sha256_hex(bytes));
    }

    /// 根据磁盘上的电子书文件刷新 MD5 / SHA-256
    pub async fn refresh_book_file_hashes(
        &mut self,
        book_path: &Path,
    ) -> crate::error::AppResult<()> {
        if book_path.is_file() {
            self.file_md5 = Some(hash::md5_hex_file(book_path).await?);
            self.file_sha256 = Some(hash::sha256_hex_file(book_path).await?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{BookMetadata, ReadingProgress};

    #[test]
    fn metadata_roundtrip() {
        let meta = BookMetadata {
            title: "测试".into(),
            author: "作者".into(),
            reading_progress: Some(ReadingProgress {
                current_page: Some(10),
                percent: Some(25.5),
                last_position: None,
            }),
            ..Default::default()
        };
        let json = meta.to_json().unwrap();
        let parsed = BookMetadata::from_json(&json).unwrap();
        assert_eq!(parsed.title, "测试");
        assert_eq!(parsed.reading_progress.unwrap().current_page, Some(10));
    }

    #[test]
    fn file_sha256_serializes() {
        let meta = BookMetadata {
            title: "T".into(),
            file_sha256: Some("abc".into()),
            ..Default::default()
        };
        let json = meta.to_json().unwrap();
        assert!(json.contains("file_sha256"));
        assert!(!json.contains("cover_sha256"));
        assert!(!json.contains("metadata_sha256"));
    }

    #[test]
    fn rating_roundtrip_and_validation() {
        let mut meta = BookMetadata {
            title: "T".into(),
            rating: Some(4),
            ..Default::default()
        };
        meta.normalize_fields().unwrap();
        let json = meta.to_json().unwrap();
        assert!(json.contains("\"rating\": 4"));
        let mut bad = BookMetadata {
            title: "T".into(),
            rating: Some(6),
            ..Default::default()
        };
        assert!(bad.normalize_fields().is_err());
        let mut zero = BookMetadata {
            title: "T".into(),
            rating: Some(0),
            ..Default::default()
        };
        zero.normalize_fields().unwrap();
        assert_eq!(zero.rating, None);
    }
}
