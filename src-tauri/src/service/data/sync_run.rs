//! 저장소 동기화 한 번 (CLAUDE.md 2-C-2, ADR-011). 저장소는 `SyncTarget`(폴더, 나중에 드라이브).
//!
//! 1. 다른 기기의 묶음(`devices/<id>.scsync`) 중 마지막으로 합친 뒤 새로 쓰인 것만 내려받아
//!    - 처음 합치기 전에 이 기기 DB·설정 사본(`sync-backups/`)
//!    - 그 묶음이 가리키는 파일 중 이 기기에 없는 것을 `blobs/<해시>`에서 받아 넣고(해시 확인)
//!    - DB를 합친다(2-B-2). 그 기기가 이 기기 묶음을 어디까지 봤는지(`acks`)도 적는다
//! 2. 이 기기 묶음을 만들어 `devices/<이 기기>.scsync`에 올린다 (파일 내용은 넣지 않고 목록만)
//! 3. 저장소에 없는 파일 내용만 `blobs/<해시>`로 올린다 (같은 내용은 한 번)
//! 4. 모든 기기가 본, 30일 넘은 지운 기록을 정리한다
//!
//! 순서가 "받기 → 올리기"인 이유: 올리는 묶음의 acks에 방금 합친 것이 들어가야 상대가 이 기기 행을 덮어쓰지 않는다.
use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::SqlitePool;

use super::sync_bundle::{self, BundleOptions};
use super::sync_merge::{self, BundleData, MergeReport};
use super::sync_target::SyncTarget;
use crate::error::{AppError, Result};

/// 지운 기록을 남겨 두는 최소 기간 (모든 기기가 봤어도)
const TOMBSTONE_KEEP_MS: i64 = 30 * 86_400_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MergedDevice {
    pub device_id: String,
    /// 그 묶음을 만든 시각 (그 기기 시계)
    pub created_at: i64,
    pub report: MergeReport,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncRunReport {
    pub merged: Vec<MergedDevice>,
    /// 새로 쓰인 것이 없어 건너뛴 기기
    pub unchanged_devices: u32,
    pub files_down: u32,
    /// 저장소에 내용이 아직 없는 파일 (그 기기가 다음 동기화 때 올림)
    pub files_missing: u32,
    pub files_up: u32,
    pub tombstones_cleaned: u32,
    /// 합치기 전 사본 (앱 데이터 폴더 기준). 합친 것이 없으면 없음
    pub backup: Option<String>,
}

fn is_device_bundle(name: &str) -> Option<String> {
    let file = name.strip_prefix("devices/")?;
    let id = file.strip_suffix(".scsync")?;
    // 동기화 앱이 만든 "충돌 사본"(이름 (1).scsync 등)은 빼고 기기 id 모양만
    uuid::Uuid::parse_str(id).ok().map(|_| id.to_string())
}

pub async fn run<T: SyncTarget>(
    db: &SqlitePool,
    base: &Path,
    settings: &mut Map<String, Value>,
    me: &str,
    target: &T,
    progress: &(dyn Fn(&str) + Sync),
) -> Result<SyncRunReport> {
    let tmp = base.join("tmp").join("sync").join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&tmp)?;
    let result = run_in(db, base, settings, me, target, progress, &tmp).await;
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

async fn run_in<T: SyncTarget>(
    db: &SqlitePool,
    base: &Path,
    settings: &mut Map<String, Value>,
    me: &str,
    target: &T,
    progress: &(dyn Fn(&str) + Sync),
    tmp: &Path,
) -> Result<SyncRunReport> {
    let mut rep = SyncRunReport::default();
    let local_schema = sync_bundle::schema_version(db).await?;

    // ---- 1. 받기 ----
    progress("다른 기기 확인");
    for e in target.list("devices/").await? {
        let Some(id) = is_device_bundle(&e.name) else { continue };
        if id == me {
            continue;
        }
        let local = tmp.join(format!("{id}.scsync"));
        target.get(&e.name, &local).await?;
        let mut b = sync_bundle::open(&local, local_schema, base)?;
        if b.manifest.device_id != id {
            continue; // 이름과 안의 기기가 다른 파일은 믿지 않는다
        }
        let seen: Option<i64> = sqlx::query_scalar("SELECT last_bundle_created_at FROM sync_peers WHERE device_id = ?")
            .bind(&id)
            .fetch_optional(db)
            .await?;
        if seen.is_some_and(|s| s >= b.manifest.created_at) {
            rep.unchanged_devices += 1;
            continue;
        }
        progress(&format!("기기 {} 합치는 중", &id[..8]));
        if rep.backup.is_none() {
            let p = super::sync_exchange::backup_before(db, base, settings).await?;
            rep.backup = Some(p.strip_prefix(base).unwrap_or(&p).to_string_lossy().replace('\\', "/"));
        }
        // 이 기기에 없는 파일을 저장소에서 (DB보다 먼저 — 합친 행이 가리키는 그림이 없는 순간이 없게)
        for f in b.files()? {
            let dest = base.join(&f.path);
            if dest.exists() {
                continue;
            }
            let part = dest.with_extension("sync-partial");
            match target.get(&format!("blobs/{}", f.hash), &part).await {
                Ok(()) if sync_bundle::sha256_file(&part)? == f.hash => {
                    std::fs::rename(&part, &dest)?;
                    rep.files_down += 1;
                }
                Ok(()) => {
                    let _ = std::fs::remove_file(&part);
                    return Err(AppError::validation(format!("동기화 폴더의 파일 내용이 해시와 다름: {}", f.path), None));
                }
                Err(_) => rep.files_missing += 1,
            }
        }
        let data = BundleData::read(&mut b, db).await?;
        let report = sync_merge::merge(db, &data, me, settings, false).await?;
        if let Some(ack) = data.manifest.acks.get(me) {
            sqlx::query("UPDATE sync_peers SET their_ack = ? WHERE device_id = ?")
                .bind(ack)
                .bind(&id)
                .execute(db)
                .await?;
        }
        rep.merged.push(MergedDevice {
            device_id: id,
            created_at: data.manifest.created_at,
            report,
        });
    }

    // ---- 2. 이 기기 묶음 올리기 ----
    progress("이 기기 내용 올리는 중");
    let mine = tmp.join("mine.scsync");
    sync_bundle::write_bundle(
        db,
        base,
        settings,
        me,
        &BundleOptions {
            since: None,
            include_files: false,
        },
        &mine,
    )
    .await?;
    target.put(&format!("devices/{me}.scsync"), &mine).await?;

    // ---- 3. 파일 내용 ----
    progress("파일 올리는 중");
    let have: HashSet<String> = target
        .list("blobs/")
        .await?
        .into_iter()
        .filter_map(|e| e.name.strip_prefix("blobs/").map(String::from))
        .collect();
    let files = sync_bundle::open(&mine, i64::MAX, base)?.files()?;
    let mut sent: HashSet<String> = HashSet::new();
    for f in files {
        if have.contains(&f.hash) || !sent.insert(f.hash.clone()) {
            continue;
        }
        target.put(&format!("blobs/{}", f.hash), &base.join(&f.path)).await?;
        rep.files_up += 1;
    }

    // ---- 4. 지운 기록 정리 ----
    rep.tombstones_cleaned = cleanup_tombstones(db).await?;
    Ok(rep)
}

/// 아는 모든 기기가 본(그 기기 묶음의 acks가 이 기록보다 뒤) 30일 넘은 지운 기록만 지운다.
/// 아직 이 기기 묶음을 합친 적이 없는 기기가 하나라도 있으면 지우지 않는다
pub async fn cleanup_tombstones(db: &SqlitePool) -> Result<u32> {
    let (peers, acked, min_ack): (i64, i64, Option<i64>) = sqlx::query_as("SELECT COUNT(*), COUNT(their_ack), MIN(their_ack) FROM sync_peers")
        .fetch_one(db)
        .await?;
    let Some(min_ack) = min_ack.filter(|_| peers > 0 && peers == acked) else {
        return Ok(0);
    };
    let before = min_ack.min(chrono::Utc::now().timestamp_millis() - TOMBSTONE_KEEP_MS);
    Ok(sqlx::query("DELETE FROM sync_tombstones WHERE deleted_at < ?")
        .bind(before)
        .execute(db)
        .await?
        .rows_affected() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::data::sync_target::FolderTarget;
    use std::path::PathBuf;

    struct Dev {
        base: PathBuf,
        db: SqlitePool,
        id: String,
    }

    async fn dev(root: &Path, name: &str) -> Dev {
        let base = root.join(name);
        let db = crate::db::conn::init_pool(base.clone()).await.unwrap();
        Dev {
            base,
            db,
            id: uuid::Uuid::new_v4().to_string(),
        }
    }

    async fn sync(d: &Dev, t: &FolderTarget) -> SyncRunReport {
        tokio::time::sleep(std::time::Duration::from_millis(4)).await;
        let r = run(&d.db, &d.base, &mut Map::new(), &d.id, t, &|_| {}).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(4)).await;
        r
    }

    async fn names(d: &Dev) -> Vec<String> {
        sqlx::query_scalar("SELECT name FROM characters ORDER BY name").fetch_all(&d.db).await.unwrap()
    }

    #[tokio::test]
    async fn three_devices_meet_in_one_folder() {
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("OneDrive");
        std::fs::create_dir_all(&shared).unwrap();
        let folder = FolderTarget::new(&shared).unwrap();
        let (a, b, c) = (dev(root.path(), "a").await, dev(root.path(), "b").await, dev(root.path(), "c").await);

        // A: 캐릭터 + 아바타 파일
        let avatar = a.base.join("characters/avatars/x.png");
        std::fs::create_dir_all(avatar.parent().unwrap()).unwrap();
        std::fs::write(&avatar, b"png").unwrap();
        sqlx::query("INSERT INTO characters (id, name, avatar, created_at, updated_at) VALUES ('c1', 'Aria', ?, 1, 0)")
            .bind(avatar.to_string_lossy().to_string())
            .execute(&a.db)
            .await
            .unwrap();

        let r = sync(&a, &folder).await;
        assert!(r.merged.is_empty() && r.files_up == 1 && r.backup.is_none(), "{r:?}");
        let r = sync(&b, &folder).await;
        assert_eq!(r.merged.len(), 1);
        assert_eq!((r.files_down, r.files_missing), (1, 0));
        assert!(r.backup.is_some());
        assert_eq!(names(&b).await, vec!["Aria"]);
        let got: String = sqlx::query_scalar("SELECT avatar FROM characters WHERE id = 'c1'")
            .fetch_one(&b.db)
            .await
            .unwrap();
        assert!(std::fs::read(&got).unwrap() == b"png", "받은 경로에 파일이 있음: {got}");

        // B가 고치고, C가 처음 들어온다
        sqlx::query("UPDATE characters SET name = 'Aria (B)' WHERE id = 'c1'")
            .execute(&b.db)
            .await
            .unwrap();
        sync(&b, &folder).await;
        let r = sync(&c, &folder).await;
        assert_eq!(r.merged.len(), 2, "A와 B 둘 다");
        assert_eq!(names(&c).await, vec!["Aria (B)"], "{r:?}");
        let r = sync(&a, &folder).await;
        assert!(r.merged.iter().all(|m| m.report.conflicts == 0), "{r:?}");
        assert_eq!(names(&a).await, vec!["Aria (B)"]);

        // 새로 쓰인 것이 없으면 건너뜀, 파일은 다시 올리지 않음
        let r = sync(&a, &folder).await;
        assert!(r.merged.is_empty() && r.unchanged_devices == 2 && r.files_up == 0, "{r:?}");

        // 동기화 앱의 충돌 사본·다른 이름 파일은 무시
        std::fs::copy(
            folder.root().join(format!("devices/{}.scsync", a.id)),
            folder.root().join("devices/copy (1).scsync"),
        )
        .unwrap();
        let r = sync(&b, &folder).await;
        let mut ids: Vec<&str> = r.merged.iter().map(|m| m.device_id.as_str()).collect();
        ids.sort();
        let mut want = vec![a.id.as_str(), c.id.as_str()];
        want.sort();
        assert_eq!(ids, want, "A와 C만 (B 자기 것·사본 제외)");
    }

    #[tokio::test]
    async fn tombstones_are_cleaned_only_when_every_peer_has_seen_them() {
        let root = tempfile::tempdir().unwrap();
        let db = crate::db::conn::init_pool(root.path().join("x")).await.unwrap();
        let old = chrono::Utc::now().timestamp_millis() - 40 * 86_400_000;
        sqlx::query("INSERT INTO sync_tombstones (tbl, key, deleted_at) VALUES ('characters', '[\"a\"]', ?), ('characters', '[\"b\"]', ?)")
            .bind(old)
            .bind(chrono::Utc::now().timestamp_millis())
            .execute(&db)
            .await
            .unwrap();
        assert_eq!(cleanup_tombstones(&db).await.unwrap(), 0, "아는 기기가 없으면 지우지 않음");
        sqlx::query("INSERT INTO sync_peers (device_id, last_bundle_created_at, last_merge_at, their_ack) VALUES ('p1', 0, 0, ?), ('p2', 0, 0, NULL)")
            .bind(chrono::Utc::now().timestamp_millis())
            .execute(&db)
            .await
            .unwrap();
        assert_eq!(cleanup_tombstones(&db).await.unwrap(), 0, "아직 안 본 기기가 있음");
        sqlx::query("UPDATE sync_peers SET their_ack = ? WHERE device_id = 'p2'")
            .bind(chrono::Utc::now().timestamp_millis())
            .execute(&db)
            .await
            .unwrap();
        assert_eq!(cleanup_tombstones(&db).await.unwrap(), 1, "30일 넘은 것만");
    }
}
