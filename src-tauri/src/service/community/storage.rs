//! 플러그인 저장소 (`storage` 권한, CLAUDE.md 6-D-2). 플러그인마다 키-값, 값은 JSON.
//! 상한을 둬 플러그인 하나가 DB를 키우지 못하게 한다.
use serde_json::Value;
use sqlx::SqlitePool;

use crate::error::{AppError, Result};

pub const MAX_KEY: usize = 200;
pub const MAX_VALUE_BYTES: usize = 64 * 1024;
pub const MAX_TOTAL_BYTES: i64 = 1024 * 1024;
pub const MAX_KEYS: i64 = 1000;
/// 앱이 쓰는 키의 머리 (플러그인의 `storage` API는 이 키를 읽거나 쓰지 못한다)
pub const RESERVED_PREFIX: &str = "__";

fn check_plugin_key(key: &str) -> Result<()> {
    if key.starts_with(RESERVED_PREFIX) {
        return Err(AppError::validation(
            format!("'{RESERVED_PREFIX}'로 시작하는 저장소 키는 앱이 쓰므로 쓸 수 없음"),
            None,
        ));
    }
    Ok(())
}

/// 플러그인의 `storage.get/set/delete`가 부르는 것 (예약 키 거절) — 앱 안에서는 `*_raw`
pub async fn get(db: &SqlitePool, plugin: &str, key: &str) -> Result<Value> {
    check_plugin_key(key)?;
    get_raw(db, plugin, key).await
}
pub async fn set(db: &SqlitePool, plugin: &str, key: &str, value: &Value) -> Result<()> {
    check_plugin_key(key)?;
    set_raw(db, plugin, key, value).await
}
pub async fn delete(db: &SqlitePool, plugin: &str, key: &str) -> Result<()> {
    check_plugin_key(key)?;
    delete_raw(db, plugin, key).await
}

fn check_key(key: &str) -> Result<()> {
    if key.is_empty() || key.chars().count() > MAX_KEY {
        return Err(AppError::validation(format!("저장소 키는 1~{MAX_KEY}자"), None));
    }
    Ok(())
}

pub async fn get_raw(db: &SqlitePool, plugin: &str, key: &str) -> Result<Value> {
    check_key(key)?;
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM user_plugin_storage WHERE plugin_id = ? AND key = ?")
        .bind(plugin)
        .bind(key)
        .fetch_optional(db)
        .await?;
    Ok(raw.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or(Value::Null))
}

/// 값이 null이면 지운다
pub async fn set_raw(db: &SqlitePool, plugin: &str, key: &str, value: &Value) -> Result<()> {
    check_key(key)?;
    if value.is_null() {
        return delete_raw(db, plugin, key).await;
    }
    let raw = serde_json::to_string(value).map_err(|e| AppError::internal(e.to_string()))?;
    if raw.len() > MAX_VALUE_BYTES {
        return Err(AppError::validation(format!("저장소 값이 너무 큼 (최대 {}KB)", MAX_VALUE_BYTES / 1024), None));
    }
    let mut tx = db.begin().await?;
    // 앱이 쓰는 예약 키(설정 등)는 플러그인의 몫에 세지 않는다 — 플러그인이 저장소를 채워 사용자의 설정 저장을 막지 못하게
    if !key.starts_with(RESERVED_PREFIX) {
        let (count, total): (i64, i64) = sqlx::query_as(
            "SELECT COUNT(*), COALESCE(SUM(LENGTH(key) + LENGTH(value)), 0) FROM user_plugin_storage WHERE plugin_id = ? AND key <> ? AND substr(key, 1, 2) <> '__'",
        )
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

pub async fn delete_raw(db: &SqlitePool, plugin: &str, key: &str) -> Result<()> {
    check_key(key)?;
    sqlx::query("DELETE FROM user_plugin_storage WHERE plugin_id = ? AND key = ?")
        .bind(plugin)
        .bind(key)
        .execute(db)
        .await?;
    Ok(())
}

/// 이 플러그인의 저장소 전체를 {키: 값} 객체로 (예약 키 포함 — 설정도 판과 함께 돌아가야 하므로). 판 폴더에 보관하는 데 쓴다
pub async fn snapshot(db: &SqlitePool, plugin: &str) -> Result<Value> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM user_plugin_storage WHERE plugin_id = ? ORDER BY key")
        .bind(plugin)
        .fetch_all(db)
        .await?;
    let mut m = serde_json::Map::new();
    for (k, v) in rows {
        m.insert(k, serde_json::from_str(&v).unwrap_or(Value::Null));
    }
    Ok(Value::Object(m))
}

/// 보관한 저장소로 통째로 바꾼다 (한 번에: 중간에 실패하면 원래대로). 상한은 보관할 때의 것이라 다시 검사하지 않는다
pub async fn restore(db: &SqlitePool, plugin: &str, snap: &Value) -> Result<()> {
    let Some(map) = snap.as_object() else {
        return Err(AppError::validation("저장소 보관본의 모양이 올바르지 않음", None));
    };
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM user_plugin_storage WHERE plugin_id = ?")
        .bind(plugin)
        .execute(&mut *tx)
        .await?;
    for (k, v) in map {
        if v.is_null() || k.is_empty() || k.chars().count() > MAX_KEY {
            continue;
        }
        let raw = serde_json::to_string(v).map_err(|e| AppError::internal(e.to_string()))?;
        sqlx::query("INSERT INTO user_plugin_storage (plugin_id, key, value, updated_at) VALUES (?, ?, ?, ?)")
            .bind(plugin)
            .bind(k)
            .bind(raw)
            .bind(chrono::Utc::now().timestamp_millis())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn keys(db: &SqlitePool, plugin: &str) -> Result<Vec<String>> {
    // 앱이 쓰는 예약 키(`__…`)는 플러그인에 보이지 않는다
    Ok(
        sqlx::query_scalar("SELECT key FROM user_plugin_storage WHERE plugin_id = ? AND substr(key, 1, 2) <> '__' ORDER BY key")
            .bind(plugin)
            .fetch_all(db)
            .await?,
    )
}
