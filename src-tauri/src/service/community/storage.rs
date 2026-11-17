//! 플러그인 저장소 (`storage` 권한, CLAUDE.md 6-D-2). 플러그인마다 키-값, 값은 JSON.
//! 상한을 둬 플러그인 하나가 DB를 키우지 못하게 한다.
use serde_json::Value;
use sqlx::SqlitePool;

use crate::error::{AppError, Result};

pub const MAX_KEY: usize = 200;
pub const MAX_VALUE_BYTES: usize = 64 * 1024;
pub const MAX_TOTAL_BYTES: i64 = 1024 * 1024;
pub const MAX_KEYS: i64 = 1000;

fn check_key(key: &str) -> Result<()> {
    if key.is_empty() || key.chars().count() > MAX_KEY {
        return Err(AppError::validation(format!("저장소 키는 1~{MAX_KEY}자"), None));
    }
    Ok(())
}

pub async fn get(db: &SqlitePool, plugin: &str, key: &str) -> Result<Value> {
    check_key(key)?;
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM user_plugin_storage WHERE plugin_id = ? AND key = ?")
        .bind(plugin)
        .bind(key)
        .fetch_optional(db)
        .await?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or(Value::Null))
}

/// 값이 null이면 지운다
pub async fn set(db: &SqlitePool, plugin: &str, key: &str, value: &Value) -> Result<()> {
    check_key(key)?;
    if value.is_null() {
        return delete(db, plugin, key).await;
    }
    let raw = serde_json::to_string(value).map_err(|e| AppError::internal(e.to_string()))?;
    if raw.len() > MAX_VALUE_BYTES {
        return Err(AppError::validation(format!("저장소 값이 너무 큼 (최대 {}KB)", MAX_VALUE_BYTES / 1024), None));
    }
    let mut tx = db.begin().await?;
    let (count, total): (i64, i64) =
        sqlx::query_as("SELECT COUNT(*), COALESCE(SUM(LENGTH(key) + LENGTH(value)), 0) FROM user_plugin_storage WHERE plugin_id = ? AND key <> ?")
            .bind(plugin)
            .bind(key)
            .fetch_one(&mut *tx)
            .await?;
    if count + 1 > MAX_KEYS || total + (key.len() + raw.len()) as i64 > MAX_TOTAL_BYTES {
        return Err(AppError::validation(
            format!("플러그인 저장소가 가득 참 (키 {MAX_KEYS}개, {}MB까지)", MAX_TOTAL_BYTES / 1024 / 1024),
            None,
        ));
    }
    sqlx::query(
        "INSERT INTO user_plugin_storage (plugin_id, key, value, updated_at) VALUES (?, ?, ?, ?)
         ON CONFLICT(plugin_id, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(plugin)
    .bind(key)
    .bind(&raw)
    .bind(chrono::Utc::now().timestamp_millis())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn delete(db: &SqlitePool, plugin: &str, key: &str) -> Result<()> {
    check_key(key)?;
    sqlx::query("DELETE FROM user_plugin_storage WHERE plugin_id = ? AND key = ?")
        .bind(plugin)
        .bind(key)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn keys(db: &SqlitePool, plugin: &str) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar("SELECT key FROM user_plugin_storage WHERE plugin_id = ? ORDER BY key")
        .bind(plugin)
        .fetch_all(db)
        .await?)
}
