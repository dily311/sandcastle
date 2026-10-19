use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteConnectOptions};
use std::path::PathBuf;
use std::str::FromStr;

pub type DbPool = SqlitePool;

pub async fn init_pool(app_data_dir: PathBuf) -> Result<DbPool, sqlx::Error> {
    std::fs::create_dir_all(&app_data_dir).ok();
    let db_path = app_data_dir.join("app.db");

    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    sqlx::migrate!("src/db/migrations").run(&pool).await?;
    Ok(pool)
}