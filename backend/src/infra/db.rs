use crate::config::Config;
use crate::error::AppResult;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use std::path::Path;

pub async fn connect(config: &Config) -> AppResult<SqlitePool> {
    if let Some(parent) = db_parent_dir(&config.database_url) {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::create_dir_all(&config.data_root).await.ok();

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

fn db_parent_dir(url: &str) -> Option<&Path> {
    let path = url.strip_prefix("sqlite:")?.split('?').next()?;
    Path::new(path).parent()
}
