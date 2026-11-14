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

/// 이 빌드에 포함된 마이그레이션 (버전, 체크섬). 백업 복원 전 호환성 검사에 쓴다.
pub fn schema_migrations() -> Vec<(i64, Vec<u8>)> {
    sqlx::migrate!("src/db/migrations")
        .migrations
        .iter()
        .map(|m| (m.version, m.checksum.to_vec()))
        .collect()
}

/// 이 빌드의 최신 스키마 버전
pub fn latest_schema_version() -> i64 {
    schema_migrations().iter().map(|(v, _)| *v).max().unwrap_or(0)
}
