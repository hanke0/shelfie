use crate::config::Config;
use crate::error::{AppError, AppResult};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

pub async fn connect(config: &Config) -> AppResult<SqlitePool> {
    if let Some(parent) = db_parent_dir(&config.database_url) {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::create_dir_all(&config.data_root).await.ok();

    let connect_options = SqliteConnectOptions::from_str(&config.database_url)
        .map_err(|e| AppError::Internal(format!("Invalid DATABASE_URL: {e}")))?
        .pragma("foreign_keys", "ON");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

fn db_parent_dir(url: &str) -> Option<&Path> {
    let path = url.strip_prefix("sqlite:")?.split('?').next()?;
    Path::new(path).parent()
}
