use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub type DbPool = SqlitePool;

/// 마이그레이션 전 DB 사본을 두는 폴더 (앱 데이터 폴더 안, 백업에는 넣지 않음 — ADR-008)
pub const MIGRATION_BACKUP_DIR: &str = "migration-backups";
/// 남겨 둘 사본 수 (오래된 것부터 지움)
const MIGRATION_BACKUP_KEEP: usize = 3;

pub async fn init_pool(app_data_dir: PathBuf) -> Result<DbPool, sqlx::Error> {
    std::fs::create_dir_all(&app_data_dir).ok();
    let db_path = app_data_dir.join("app.db");

    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new().max_connections(5).connect_with(opts).await?;

    // 적용할 마이그레이션이 있으면 먼저 사본을 만든다 (1-B-2). 실패해도 앱은 연다(로그만)
    match backup_before_migrations(&pool, &app_data_dir).await {
        Ok(Some(p)) => eprintln!("[db] 마이그레이션 전 사본: {}", p.display()),
        Ok(None) => {}
        Err(e) => eprintln!("[db] 마이그레이션 전 사본을 만들지 못함 (계속 진행): {e}"),
    }

    sqlx::migrate!("src/db/migrations").run(&pool).await?;
    Ok(pool)
}

/// 이미 쓰던 DB에 이 빌드의 새 마이그레이션이 있으면 `migration-backups/app-<시각>-v<지금 버전>.db`로 사본을 만든다.
/// 새로 만든 DB(적용된 마이그레이션 없음)는 건너뛴다. 사본은 최근 몇 개만 남긴다
pub async fn backup_before_migrations(pool: &DbPool, app_data_dir: &Path) -> Result<Option<PathBuf>, sqlx::Error> {
    let has_table: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations')")
        .fetch_one(pool)
        .await?;
    if !has_table {
        return Ok(None);
    }
    let applied: Vec<i64> = sqlx::query_scalar("SELECT version FROM _sqlx_migrations WHERE success = 1")
        .fetch_all(pool)
        .await?;
    let Some(current) = applied.iter().max().copied() else { return Ok(None) };
    let pending = schema_migrations().iter().any(|(v, _)| !applied.contains(v));
    if !pending {
        return Ok(None);
    }

    let dir = app_data_dir.join(MIGRATION_BACKUP_DIR);
    std::fs::create_dir_all(&dir).map_err(sqlx::Error::Io)?;
    let out = dir.join(format!("app-{}-v{current}.db", chrono::Local::now().format("%Y%m%d-%H%M%S")));
    // VACUUM INTO: 열린 DB에서도 일관된 사본 (WAL 내용 포함)
    sqlx::query("VACUUM INTO ?").bind(out.to_string_lossy().to_string()).execute(pool).await?;
    prune_migration_backups(&dir);
    Ok(Some(out))
}

/// 오래된 사본 지우기 (이름에 시각이 들어 있어 이름순 = 시간순)
fn prune_migration_backups(dir: &Path) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<PathBuf> = read
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "db") && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("app-")))
        .collect();
    files.sort();
    let extra = files.len().saturating_sub(MIGRATION_BACKUP_KEEP);
    for p in files.into_iter().take(extra) {
        let _ = std::fs::remove_file(p);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn backups(base: &Path) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(base.join(MIGRATION_BACKUP_DIR))
            .map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        v.sort();
        v
    }

    #[tokio::test]
    async fn copies_db_only_when_migrations_are_pending() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("appdata");

        // 새 DB: 사본 없음
        let pool = init_pool(base.clone()).await.unwrap();
        assert!(backups(&base).is_empty());
        sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c1', 'Keep', 0, 0)")
            .execute(&pool)
            .await
            .unwrap();
        // 최신이면 사본 없음
        assert!(backup_before_migrations(&pool, &base).await.unwrap().is_none());

        // 마지막 마이그레이션이 안 된 상태로 되돌려 "업데이트한 앱"을 흉내
        let latest = latest_schema_version();
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = ?")
            .bind(latest)
            .execute(&pool)
            .await
            .unwrap();
        let copy = backup_before_migrations(&pool, &base).await.unwrap().expect("사본");
        assert!(copy.file_name().unwrap().to_string_lossy().ends_with(&format!(
            "-v{}.db",
            schema_migrations().iter().map(|(v, _)| *v).filter(|v| *v < latest).max().unwrap()
        )));
        let snap = SqlitePoolOptions::new()
            .connect_with(SqliteConnectOptions::new().filename(&copy).read_only(true))
            .await
            .unwrap();
        let name: String = sqlx::query_scalar("SELECT name FROM characters WHERE id = 'c1'")
            .fetch_one(&snap)
            .await
            .unwrap();
        assert_eq!(name, "Keep");
        snap.close().await;

        // 최근 3개만 남김
        for i in 0..5 {
            std::fs::write(base.join(MIGRATION_BACKUP_DIR).join(format!("app-2000010{i}-000000-v1.db")), "x").unwrap();
        }
        prune_migration_backups(&base.join(MIGRATION_BACKUP_DIR));
        let left = backups(&base);
        assert_eq!(left.len(), MIGRATION_BACKUP_KEEP);
        assert!(left.contains(&copy), "가장 최근 사본은 남음");
    }
}
