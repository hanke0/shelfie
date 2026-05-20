#[cfg(feature = "embed-frontend")]
pub mod embed_frontend;

pub mod content_disposition;
pub mod db;
pub mod fs;
pub mod hash;
pub mod metadata;
pub mod safe_name;
pub mod thumbnail;

pub use metadata::{BookMetadata, ReadingProgress};
