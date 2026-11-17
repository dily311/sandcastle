//! 둘 다 남긴 것 고르기와 합치기 되돌리기 (CLAUDE.md 2-E, ADR-011).
//!
//! 충돌(`sync_conflicts`)마다:
//! - edit(양쪽이 고침): **이 기기 것 두기** — 이 행을 지금 고친 것으로 표시해(updated_at) 다음 동기화 때 다른 기기도 이것을
//!   받게 한다. **받은 것으로 바꾸기** — 받은 행으로 바꾸고(지금 고친 것으로) 다른 기기도 같은 내용이 된다.
//! - unique / orphan(넣지 못한 받은 행): 내용을 보고 **버리기**만 (넣을 자리가 없거나 가리키는 항목이 없음).
//! - branch(같은 대화에 이어 씀): 분기 대화를 열어 보고 **확인함**. 필요 없으면 그 대화를 지우면 된다(휴지통).
//!
//! 동기화로 양쪽 내용이 같아지면 열린 충돌은 저절로 닫힌다(sync_merge).
//!
//! 되돌리기: 합치기 전 사본(`sync-backups/app-….db` + `settings-….json`)을 고르면 다음 시작 때 지금 DB·설정을
//! `restore-backups/`로 치우고 사본으로 바꾼다. 자동 동기화는 끈다 — 켜 둔 채면 다음 동기화가 되돌린 내용을 다시 합친다.
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Map, Value};
use sqlx::SqlitePool;

use crate::db::schema::{bind_query, quote, Schema};
use crate::error::{AppError, Result};

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FieldDiff {
    pub column: String,
    pub local: Value,
    pub remote: Value,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncConflict {
    pub id: String,
    /// edit / unique / orphan / branch
    pub kind: String,
    pub tbl: String,
    pub key: String,
    /// 이름·제목·내용 앞부분 (화면용)
    pub label: String,
    pub device_id: String,
    pub created_at: i64,
    /// edit: 다른 칸만 (시각 칸 빼고)
    pub diff: Vec<FieldDiff>,
    /// 받은 행 (unique·orphan은 넣지 못한 행 전체, branch는 {branchId, fromSeq})
    pub remote: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ConflictChoice {
    KeepLocal,
    UseRemote,
    Dismiss,
}

fn label_of(row: &Map<String, Value>) -> String {
    for k in ["name", "title", "content", "label", "key"] {
        if let Some(s) = row.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
            let s: String = s.chars().take(60).collect();
            return s;
        }
    }
    String::new()
}

fn diff(local: &Map<String, Value>, remote: &Map<String, Value>) -> Vec<FieldDiff> {
    remote
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "updated_at" | "created_at"))
        .filter_map(|(k, r)| {
            let l = local.get(k).cloned().unwrap_or(Value::Null);
            (l != *r).then(|| FieldDiff {
                column: k.clone(),
                local: l,
                remote: r.clone(),
            })
        })
        .collect()
}

fn parse_obj(s: Option<String>) -> Option<Map<String, Value>> {
    match serde_json::from_str::<Value>(&s?).ok()? {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

type ConflictRow = (String, String, String, String, Option<String>, Option<String>, String, i64);

pub async fn list(db: &SqlitePool) -> Result<Vec<SyncConflict>> {
    let rows: Vec<ConflictRow> = sqlx::query_as(
        "SELECT id, kind, tbl, key, local_row, remote_row, device_id, created_at FROM sync_conflicts WHERE resolved_at IS NULL ORDER BY created_at DESC LIMIT 500",
    )
    .fetch_all(db)
    .await?;
    let mut out = vec![];
    for (id, kind, tbl, key, local, remote, device_id, created_at) in rows {
        let (local, remote_obj, remote_raw) = (parse_obj(local), parse_obj(remote.clone()), remote.and_then(|r| serde_json::from_str(&r).ok()));
        let label = match kind.as_str() {
            // 분기는 원래 대화의 제목
            "branch" => {
                let k: Vec<Value> = serde_json::from_str(&key).unwrap_or_default();
                sqlx::query_scalar::<_, String>("SELECT title FROM conversations WHERE id = ?")
                    .bind(k.first().and_then(Value::as_str).unwrap_or(""))
                    .fetch_optional(db)
                    .await?
                    .unwrap_or_default()
            }
            _ => label_of(remote_obj.as_ref().or(local.as_ref()).unwrap_or(&Map::new())),
        };
        let diff = match (&local, &remote_obj) {
            (Some(l), Some(r)) if kind == "edit" => diff(l, r),
            _ => vec![],
        };
        out.push(SyncConflict {
            id,
            kind,
            tbl,
            key,
            label,
            device_id,
            created_at,
            diff,
            remote: remote_raw,
        });
    }
    Ok(out)
}

pub async fn open_count(db: &SqlitePool) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM sync_conflicts WHERE resolved_at IS NULL")
        .fetch_one(db)
        .await?)
}

/// 고르기. edit만 이 기기 것 두기 / 받은 것으로 바꾸기가 되고, 나머지는 버리기(확인함)만
pub async fn resolve(db: &SqlitePool, id: &str, choice: ConflictChoice) -> Result<()> {
    let row: Option<(String, String, String, Option<String>)> =
        sqlx::query_as("SELECT kind, tbl, key, remote_row FROM sync_conflicts WHERE id = ? AND resolved_at IS NULL")
            .bind(id)
            .fetch_optional(db)
            .await?;
    let (kind, tbl, key, remote) = row.ok_or_else(|| AppError::not_found("이미 고른 충돌이거나 없는 충돌"))?;
    let now = chrono::Utc::now().timestamp_millis();
    let mut tx = db.begin().await?;
    if choice != ConflictChoice::Dismiss {
        if kind != "edit" {
            return Err(AppError::validation("이 항목은 버리기(확인함)만 할 수 있음", None));
        }
        let schema = Schema::load(&mut tx).await?;
        let key_cols = schema
            .keys
            .get(&tbl)
            .filter(|k| !k.is_empty())
            .cloned()
            .ok_or_else(|| AppError::validation(format!("모르는 테이블: {tbl}"), None))?;
        let key_vals: Vec<Value> = serde_json::from_str(&key).unwrap_or_default();
        if key_vals.len() != key_cols.len() {
            return Err(AppError::internal("충돌의 키 모양이 이상함"));
        }
        let cond = key_cols.iter().map(|c| format!("{} = ?", quote(c))).collect::<Vec<_>>().join(" AND ");
        let cols = schema.columns.get(&tbl).cloned().unwrap_or_default();
        match choice {
            ConflictChoice::KeepLocal => {
                // 지금 고친 것으로 표시 → 다음 동기화 때 다른 기기가 이것을 받는다
                let mut q = sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE {} SET updated_at = ? WHERE {cond}", quote(&tbl)))).bind(now);
                for v in &key_vals {
                    q = bind_query(q, v);
                }
                if q.execute(&mut *tx).await?.rows_affected() == 0 {
                    return Err(AppError::not_found("이 기기에 그 항목이 더는 없음 — 버리기를 고르세요"));
                }
            }
            ConflictChoice::UseRemote => {
                let r = parse_obj(remote).ok_or_else(|| AppError::internal("받은 행을 읽지 못함"))?;
                let sets: Vec<&String> = cols.iter().filter(|c| r.contains_key(c.as_str()) && !key_cols.contains(c)).collect();
                let sql = format!(
                    "UPDATE {} SET {} WHERE {cond}",
                    quote(&tbl),
                    sets.iter().map(|c| format!("{} = ?", quote(c))).collect::<Vec<_>>().join(", ")
                );
                let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
                for c in &sets {
                    let v = if c.as_str() == "updated_at" {
                        Value::from(now)
                    } else {
                        r[c.as_str()].clone()
                    };
                    q = bind_query(q, &v);
                }
                for v in &key_vals {
                    q = bind_query(q, v);
                }
                if q.execute(&mut *tx).await?.rows_affected() == 0 {
                    return Err(AppError::not_found("이 기기에 그 항목이 더는 없음 — 버리기를 고르세요"));
                }
            }
            ConflictChoice::Dismiss => {}
        }
    }
    // 같은 행의 다른 열린 edit 충돌도 같이 닫는다 (같은 기기에서 여러 번 받은 경우)
    sqlx::query("UPDATE sync_conflicts SET resolved_at = ? WHERE (id = ? OR (tbl = ? AND key = ? AND kind = 'edit' AND ? = 'edit')) AND resolved_at IS NULL")
        .bind(now)
        .bind(id)
        .bind(&tbl)
        .bind(&key)
        .bind(&kind)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 합치기 되돌리기
// ---------------------------------------------------------------------------

const ROLLBACK_FILE: &str = "sync-rollback.json";
const KEEP_DIR: &str = "restore-backups";

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncBackup {
    /// `app-….db` 파일 이름
    pub name: String,
    pub created_at: i64,
    pub bytes: u64,
}

fn backups_dir(base: &Path) -> PathBuf {
    base.join(super::sync_exchange::BACKUP_DIR)
}

fn settings_for(db_name: &str) -> String {
    db_name.replacen("app-", "settings-", 1).replace(".db", ".json")
}

/// 합치기 전 사본 (최근부터)
pub fn list_backups(base: &Path) -> Vec<SyncBackup> {
    let Ok(read) = std::fs::read_dir(backups_dir(base)) else { return vec![] };
    let mut out: Vec<SyncBackup> = read
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if !(name.starts_with("app-") && name.ends_with(".db")) {
                return None;
            }
            let meta = e.metadata().ok()?;
            let created_at = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis() as i64;
            Some(SyncBackup {
                name,
                created_at,
                bytes: meta.len(),
            })
        })
        .collect();
    out.sort_by(|a, b| b.name.cmp(&a.name));
    out
}

/// 되돌리기 준비: 다음 시작 때 적용한다
pub fn request_rollback(base: &Path, name: &str) -> Result<()> {
    if !list_backups(base).iter().any(|b| b.name == name) {
        return Err(AppError::not_found(format!("없는 사본: {name}")));
    }
    std::fs::write(base.join(ROLLBACK_FILE), serde_json::to_vec(&serde_json::json!({ "db": name }))?)?;
    Ok(())
}

pub fn pending_rollback(base: &Path) -> Option<String> {
    let raw = std::fs::read(base.join(ROLLBACK_FILE)).ok()?;
    serde_json::from_slice::<Value>(&raw).ok()?["db"].as_str().map(String::from)
}

pub fn cancel_rollback(base: &Path) {
    let _ = std::fs::remove_file(base.join(ROLLBACK_FILE));
}

/// 앱 시작 시 DB를 열기 전에. 준비된 되돌리기가 있으면 지금 DB·설정을 `restore-backups/`로 치우고 사본으로 바꾼다.
/// 중간에 실패하면 원래대로 둔다. 되돌린 설정의 자동 동기화는 끈다
pub fn apply_pending_rollback(base: &Path) -> Result<bool> {
    let Some(name) = pending_rollback(base) else { return Ok(false) };
    cancel_rollback(base);
    let dir = backups_dir(base);
    let (db_src, settings_src) = (dir.join(&name), dir.join(settings_for(&name)));
    if !db_src.exists() {
        return Err(AppError::not_found(format!("되돌릴 사본이 없음: {name}")));
    }
    let keep = base
        .join(KEEP_DIR)
        .join(format!("{}-sync-rollback", chrono::Local::now().format("%Y%m%d-%H%M%S")));
    std::fs::create_dir_all(&keep)?;
    let mut moved: Vec<(PathBuf, PathBuf)> = vec![];
    let result = (|| -> Result<()> {
        for f in ["app.db", "app.db-wal", "app.db-shm", "settings.json"] {
            let from = base.join(f);
            if from.exists() {
                std::fs::rename(&from, keep.join(f))?;
                moved.push((from, keep.join(f)));
            }
        }
        std::fs::copy(&db_src, base.join("app.db"))?;
        if let Ok(text) = std::fs::read_to_string(&settings_src) {
            let mut v: Value = serde_json::from_str(&text).unwrap_or(Value::Object(Map::new()));
            if let Some(sync) = v.pointer_mut("/plugin-setting/sync").and_then(Value::as_object_mut) {
                sync.insert("auto".into(), Value::Bool(false));
            }
            std::fs::write(base.join("settings.json"), serde_json::to_vec_pretty(&v)?)?;
        } else if let Some((_, kept)) = moved.iter().find(|(f, _)| f.ends_with("settings.json")) {
            // 설정 사본이 없으면 지금 설정을 그대로
            std::fs::copy(kept, base.join("settings.json"))?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(base.join("app.db"));
        for (from, to) in moved.into_iter().rev() {
            let _ = std::fs::rename(&to, &from);
        }
        return Err(AppError::io(format!("되돌리기 실패 (원래 데이터로 둠): {e}")));
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn conflict(db: &SqlitePool, kind: &str, tbl: &str, key: &str, local: Option<Value>, remote: Option<Value>) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO sync_conflicts (id, kind, tbl, key, local_row, remote_row, device_id, created_at) VALUES (?, ?, ?, ?, ?, ?, 'dev-b', 1)")
            .bind(&id)
            .bind(kind)
            .bind(tbl)
            .bind(key)
            .bind(local.map(|v| v.to_string()))
            .bind(remote.map(|v| v.to_string()))
            .execute(db)
            .await
            .unwrap();
        id
    }

    #[tokio::test]
    async fn choose_keep_use_and_dismiss() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::conn::init_pool(dir.path().join("a")).await.unwrap();
        sqlx::query("INSERT INTO characters (id, name, description, created_at, updated_at) VALUES ('c1', '내 이름', '같음', 1, 5)")
            .execute(&db)
            .await
            .unwrap();
        let local = serde_json::json!({ "id": "c1", "name": "내 이름", "description": "같음", "updated_at": 5 });
        let remote = serde_json::json!({ "id": "c1", "name": "받은 이름", "description": "같음", "updated_at": 9 });

        let a = conflict(&db, "edit", "characters", r#"["c1"]"#, Some(local.clone()), Some(remote.clone())).await;
        let list = list(&db).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].label, "받은 이름");
        assert_eq!(list[0].diff.len(), 1, "다른 칸만: {:?}", list[0].diff);
        assert_eq!(list[0].diff[0].column, "name");

        // 이 기기 것 두기: 내용은 그대로, 지금 고친 것으로
        resolve(&db, &a, ConflictChoice::KeepLocal).await.unwrap();
        let (name, at): (String, i64) = sqlx::query_as("SELECT name, updated_at FROM characters WHERE id = 'c1'")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(name, "내 이름");
        assert!(at > 5);
        assert_eq!(open_count(&db).await.unwrap(), 0);
        assert!(resolve(&db, &a, ConflictChoice::KeepLocal).await.is_err(), "이미 고름");

        // 받은 것으로 바꾸기
        let b = conflict(&db, "edit", "characters", r#"["c1"]"#, Some(local), Some(remote.clone())).await;
        resolve(&db, &b, ConflictChoice::UseRemote).await.unwrap();
        let name: String = sqlx::query_scalar("SELECT name FROM characters WHERE id = 'c1'").fetch_one(&db).await.unwrap();
        assert_eq!(name, "받은 이름");

        // 넣지 못한 행은 버리기만
        let c = conflict(&db, "unique", "characters", r#"["c9"]"#, None, Some(remote)).await;
        assert!(resolve(&db, &c, ConflictChoice::UseRemote).await.is_err());
        resolve(&db, &c, ConflictChoice::Dismiss).await.unwrap();
        assert_eq!(open_count(&db).await.unwrap(), 0);
    }

    #[test]
    fn rollback_swaps_db_and_settings_and_turns_auto_off() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let backups = base.join("sync-backups");
        std::fs::create_dir_all(&backups).unwrap();
        std::fs::write(base.join("app.db"), b"now").unwrap();
        std::fs::write(base.join("settings.json"), b"{}").unwrap();
        std::fs::write(backups.join("app-20261117-100000-000.db"), b"before").unwrap();
        std::fs::write(
            backups.join("settings-20261117-100000-000.json"),
            serde_json::json!({ "plugin-setting": { "sync": { "folder": "x", "auto": true } } }).to_string(),
        )
        .unwrap();

        assert!(request_rollback(base, "app-없음.db").is_err());
        assert!(!apply_pending_rollback(base).unwrap(), "준비한 것이 없으면 그대로");
        request_rollback(base, "app-20261117-100000-000.db").unwrap();
        assert_eq!(pending_rollback(base).as_deref(), Some("app-20261117-100000-000.db"));
        assert!(apply_pending_rollback(base).unwrap());
        assert_eq!(std::fs::read(base.join("app.db")).unwrap(), b"before");
        let s: Value = serde_json::from_slice(&std::fs::read(base.join("settings.json")).unwrap()).unwrap();
        assert_eq!(s["plugin-setting"]["sync"]["auto"], false, "되돌리면 자동 동기화는 꺼짐");
        assert_eq!(s["plugin-setting"]["sync"]["folder"], "x");
        // 지금 것은 지우지 않고 치워 둔다
        let kept: Vec<_> = std::fs::read_dir(base.join("restore-backups")).unwrap().flatten().collect();
        assert_eq!(std::fs::read(kept[0].path().join("app.db")).unwrap(), b"now");
        assert!(pending_rollback(base).is_none());
    }
}
