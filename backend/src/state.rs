use crate::config::Config;
use crate::domain::tasks::TaskManager;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub config: Arc<Config>,
    pub tasks: TaskManager,
}

impl AppState {
    pub fn new(db: SqlitePool, config: Config) -> Self {
        Self {
            db,
            config: Arc::new(config),
            tasks: TaskManager::default(),
        }
    }
}
