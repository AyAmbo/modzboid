use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous};
use std::path::Path;
use std::str::FromStr;
use crate::app_core::error::AppError;

pub async fn init_db(app_data_dir: &Path) -> Result<SqlitePool, AppError> {
    let db_path = app_data_dir.join("cache.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| AppError::Database(e.to_string()))?
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(std::time::Duration::from_secs(30));

    let pool = SqlitePoolOptions::new()
        .max_connections(1) // serialize all writes through one connection
        .connect_with(options)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Path is relative to CARGO_MANIFEST_DIR (src-tauri/)
    sqlx::migrate!("resources/migrations")
        .run(&pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(pool)
}
