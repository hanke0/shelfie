use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub data_root: PathBuf,
    pub jwt_secret: String,
    pub host: String,
    pub port: u16,
    /// 用于 OPDS 等需要绝对 URL 的场景；未设置时从请求头推断
    pub public_base_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/shelfie.db?mode=rwc".into()),
            data_root: PathBuf::from(
                std::env::var("DATA_ROOT").unwrap_or_else(|_| "data/libraries".into()),
            ),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-production".into()),
            host: std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            public_base_url: std::env::var("PUBLIC_BASE_URL")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim_end_matches('/').to_string()),
        }
    }
}
