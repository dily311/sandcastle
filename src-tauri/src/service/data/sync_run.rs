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

use futures::stream::{self, StreamExt};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::SqlitePool;

use super::sync_bundle::{self, BundleOptions};
use super::sync_merge::{self, BundleData, MergeReport};
use super::sync_target::SyncTarget;
use crate::error::{AppError, Result};

/// 파일 내용을 동시에 몇 개씩 주고받나
const PARALLEL: usize = 4;

/// 지운 기록을 남겨 두는 최소 기간 (모든 기기가 봤어도)
const TOMBSTONE_KEEP_MS: i64 = 30 * 86_400_000;

/// 이 기간 넘게 새 묶음이 안 온 기기는 "오래 못 봄"으로 보여 준다 (1-F-3-3). 자동으로 빼지는 않고 사용자가 제외해야 정리가 풀린다
pub const STALE_PEER_MS: i64 = 90 * 86_400_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MergedDevice {
    pub device_id: String,
    /// 그 묶음을 만든 시각 (그 기기 시계)
    pub created_at: i64,
    pub report: MergeReport,
    /// 사용자가 제외했던 기기가 돌아와 전체를 다시 합쳤다. 그동안 지운 기록이 정리됐을 수 있어, 그 기기에 남아 있던 항목이 되살아났을 수 있다
    #[serde(default)]
    pub returned_after_exclusion: bool,
}

/// 열지 못해 건너뛴 기기의 묶음 (더 새 앱에서 만든 것, 깨진 것 등). 다른 기기와의 동기화는 막지 않는다 — 이유를 보여 주고 다음 동기화 때 다시 본다
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SkippedDevice {
    pub device_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncRunReport {
    /// 열지 못해 건너뛴 기기
    #[serde(default)]
    pub skipped_devices: Vec<SkippedDevice>,
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
        let mut b = match sync_bundle::open(&local, local_schema, base) {
            Ok(b) => b,
            Err(e) => {
                rep.skipped_devices.push(SkippedDevice {
                    device_id: id,
                    reason: e.to_string(),
                });
                continue;
            }
        };
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
        let wanted: Vec<_> = b.files()?.into_iter().filter(|f| !base.join(&f.path).exists()).collect();
        // 동시에 몇 개씩 (드라이브는 파일마다 왕복이 있어 하나씩이면 느리다)
        let results: Vec<Result<bool>> = stream::iter(wanted)
            .map(|f| async move {
                let dest = base.join(&f.path);
                let part = dest.with_extension("sync-partial");
                match target.get(&format!("blobs/{}", f.hash), &part).await {
                    Ok(()) if sync_bundle::sha256_file(&part)? == f.hash => {
                        std::fs::rename(&part, &dest)?;
                        Ok(true)
                    }
                    Ok(()) => {
                        let _ = std::fs::remove_file(&part);
                        Err(AppError::validation(format!("동기화 저장소의 파일 내용이 해시와 다름: {}", f.path), None))
                    }
                    Err(_) => Ok(false),
                }
            })
            .buffer_unordered(PARALLEL)
            .collect()
            .await;
        for r in results {
            if r? {
                rep.files_down += 1;
            } else {
                rep.files_missing += 1;
            }
        }
        let was_excluded: bool = sqlx::query_scalar("SELECT COALESCE((SELECT excluded FROM sync_peers WHERE device_id = ?), 0)")
            .bind(&id)
            .fetch_one(db)
            .await?;
        let data = BundleData::read_meta(&mut b, db).await?;
        let report = sync_merge::merge_bundle(db, &mut b, &data, me, settings, false).await?;
        if let Some(ack) = data.manifest.acks.get(me) {
            sqlx::query("UPDATE sync_peers SET their_ack = ? WHERE device_id = ?")
                .bind(ack)
                .bind(&id)
                .execute(db)
                .await?;
        }
        if was_excluded {
            // 돌아온 기기: 제외를 풀고 (이미 전체 묶음을 합쳤으므로 전체 재동기화가 된 것) 보고에 표시한다
            sqlx::query("UPDATE sync_peers SET excluded = 0 WHERE device_id = ?")
                .bind(&id)
                .execute(db)
                .await?;
        }
        rep.merged.push(MergedDevice {
            device_id: id,
            created_at: data.manifest.created_at,
            report,
            returned_after_exclusion: was_excluded,
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
    let todo: Vec<_> = files.into_iter().filter(|f| !have.contains(&f.hash) && sent.insert(f.hash.clone())).collect();
    let results: Vec<Result<()>> = stream::iter(todo)
        .map(|f| async move { target.put(&format!("blobs/{}", f.hash), &base.join(&f.path)).await })
        .buffer_unordered(PARALLEL)
        .collect()
        .await;
    for r in results {
        r?;
        rep.files_up += 1;
    }

    // ---- 4. 지운 기록 정리 ----
    rep.tombstones_cleaned = cleanup_tombstones(db).await?;
    Ok(rep)
}

/// 아는 모든 기기가 본(그 기기 묶음의 acks가 이 기록보다 뒤) 30일 넘은 지운 기록만 지운다.
/// 아직 이 기기 묶음을 합친 적이 없는 기기가 하나라도 있으면 지우지 않는다
pub async fn cleanup_tombstones(db: &SqlitePool) -> Result<u32> {
    cleanup_tombstones_at(db, chrono::Utc::now().timestamp_millis()).await
}

/// 시계를 받는 판 (시험에서 몇십 일 뒤를 흉내 낸다). 사용자가 제외한 기기는 기다리지 않는다
pub async fn cleanup_tombstones_at(db: &SqlitePool, now: i64) -> Result<u32> {
    let (peers, acked, min_ack): (i64, i64, Option<i64>) =
        sqlx::query_as("SELECT COUNT(*), COUNT(their_ack), MIN(their_ack) FROM sync_peers WHERE excluded = 0")
            .fetch_one(db)
            .await?;
    let Some(min_ack) = min_ack.filter(|_| peers > 0 && peers == acked) else {
        return Ok(0);
    };
    let before = min_ack.min(now - TOMBSTONE_KEEP_MS);
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
        dev_with(root, name, &uuid::Uuid::new_v4().to_string()).await
    }

    async fn dev_with(root: &Path, name: &str, id: &str) -> Dev {
        let base = root.join(name);
        let db = crate::db::conn::init_pool(base.clone()).await.unwrap();
        Dev { base, db, id: id.to_string() }
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
        // 기기 id 순서 = 저장소 목록 순서 = 합치는 순서. 두 순서 모두 (새 판을 먼저 받고 옛 판이 나중에 와도 충돌이 아니어야)
        let low = "00000000-0000-4000-8000-000000000000";
        let high = "ffffffff-ffff-4fff-bfff-ffffffffffff";
        three_devices(low, high).await;
        three_devices(high, low).await;
    }

    async fn three_devices(a_id: &str, b_id: &str) {
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("OneDrive");
        std::fs::create_dir_all(&shared).unwrap();
        let folder = FolderTarget::new(&shared).unwrap();
        let (a, b, c) = (
            dev_with(root.path(), "a", a_id).await,
            dev_with(root.path(), "b", b_id).await,
            dev(root.path(), "c").await,
        );

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

    // ---- 1-F-3-3 장기 오프라인 ----
    const DAY: i64 = 86_400_000;

    async fn tombstones(d: &Dev) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM sync_tombstones").fetch_one(&d.db).await.unwrap()
    }

    async fn add_char(d: &Dev, id: &str, name: &str) {
        sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES (?, ?, 1, 0)")
            .bind(id)
            .bind(name)
            .execute(&d.db)
            .await
            .unwrap();
    }

    /// 셋이 모두 서로의 내용을 한 번씩 본 상태 (acks가 퍼짐)
    async fn settle(ds: &[&Dev], t: &FolderTarget) {
        for _ in 0..2 {
            for d in ds {
                sync(d, t).await;
            }
        }
    }

    #[tokio::test]
    async fn a_device_offline_for_weeks_keeps_its_edits_and_deleted_rows_stay_deleted() {
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("Drive");
        std::fs::create_dir_all(&shared).unwrap();
        let folder = FolderTarget::new(&shared).unwrap();
        let (a, b, c) = (dev(root.path(), "a").await, dev(root.path(), "b").await, dev(root.path(), "c").await);
        add_char(&a, "gone", "곧 지움").await;
        add_char(&a, "kept", "원래 이름").await;
        settle(&[&a, &b, &c], &folder).await;
        assert_eq!(names(&c).await.len(), 2);

        // C가 오프라인: 그동안 A가 지우고 고치고, C도 같은 것을 고친다
        sqlx::query("DELETE FROM characters WHERE id = 'gone'").execute(&a.db).await.unwrap();
        sqlx::query("UPDATE characters SET name = 'A가 고침' WHERE id = 'kept'")
            .execute(&a.db)
            .await
            .unwrap();
        sqlx::query("UPDATE characters SET name = 'C가 고침' WHERE id = 'kept'")
            .execute(&c.db)
            .await
            .unwrap();
        for _ in 0..3 {
            sync(&a, &folder).await;
            sync(&b, &folder).await;
        }
        assert_eq!(tombstones(&a).await, 1);
        // ① 45일이 지나도 C가 아직 못 봤으므로 지운 기록은 남는다
        let later = chrono::Utc::now().timestamp_millis() + 45 * DAY;
        assert_eq!(cleanup_tombstones_at(&a.db, later).await.unwrap(), 0);
        assert_eq!(cleanup_tombstones_at(&b.db, later).await.unwrap(), 0);
        assert_eq!(tombstones(&a).await, 1);

        // ② C가 돌아온다: 지운 것은 되살아나지 않고, C의 오프라인 수정은 충돌로 남는다
        let r = sync(&c, &folder).await;
        assert!(
            r.merged.iter().map(|m| m.report.conflicts).sum::<u32>() >= 1,
            "C의 수정과 A의 수정이 둘 다 남아야 함: {r:?}"
        );
        assert!(r.merged.iter().all(|m| !m.returned_after_exclusion));
        let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM characters ORDER BY id").fetch_all(&c.db).await.unwrap();
        assert_eq!(ids, vec!["kept"], "지운 항목이 C에서도 지워짐");
        settle(&[&a, &b, &c], &folder).await;
        assert_eq!(names(&a).await.len(), 1, "A에도 되살아나지 않음");
        // C가 봤으니 이제 정리된다
        assert_eq!(cleanup_tombstones_at(&a.db, later).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn excluding_a_stale_device_lets_cleanup_proceed_and_its_return_is_flagged() {
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("Drive");
        std::fs::create_dir_all(&shared).unwrap();
        let folder = FolderTarget::new(&shared).unwrap();
        let (a, b, d) = (dev(root.path(), "a").await, dev(root.path(), "b").await, dev(root.path(), "d").await);
        add_char(&a, "x", "남은 것").await;
        add_char(&a, "y", "지울 것").await;
        settle(&[&a, &b, &d], &folder).await;

        // D가 사라진 채 A가 지운다
        sqlx::query("DELETE FROM characters WHERE id = 'y'").execute(&a.db).await.unwrap();
        for _ in 0..3 {
            sync(&a, &folder).await;
            sync(&b, &folder).await;
        }
        let much_later = chrono::Utc::now().timestamp_millis() + 100 * DAY;
        assert_eq!(cleanup_tombstones_at(&a.db, much_later).await.unwrap(), 0, "D를 기다리는 중");

        // 사용자가 D를 제외 → 정리됨
        sqlx::query("UPDATE sync_peers SET excluded = 1 WHERE device_id = ?")
            .bind(&d.id)
            .execute(&a.db)
            .await
            .unwrap();
        assert_eq!(cleanup_tombstones_at(&a.db, much_later).await.unwrap(), 1);
        assert_eq!(tombstones(&a).await, 0);

        // D가 돌아오면 전체를 다시 합치고 표시된다. 지운 기록이 없으므로 D에 남은 항목이 되살아날 수 있다 (알려진 한계)
        let r = sync(&d, &folder).await;
        assert!(r.merged.len() >= 1);
        let r = sync(&a, &folder).await;
        let back: Vec<_> = r.merged.iter().filter(|m| m.device_id == d.id).collect();
        assert_eq!(back.len(), 1);
        assert!(back[0].returned_after_exclusion, "{r:?}");
        let excluded: bool = sqlx::query_scalar("SELECT excluded FROM sync_peers WHERE device_id = ?")
            .bind(&d.id)
            .fetch_one(&a.db)
            .await
            .unwrap();
        assert!(!excluded, "돌아오면 제외가 풀림");
    }

    // ---- 1-F-3-4 중단·깨진 묶음 ----
    #[tokio::test]
    async fn a_bundle_that_cannot_be_opened_is_reported_and_does_not_block_the_others() {
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("Drive");
        std::fs::create_dir_all(&shared).unwrap();
        let folder = FolderTarget::new(&shared).unwrap();
        let (a, b) = (dev(root.path(), "a").await, dev(root.path(), "b").await);
        add_char(&a, "c1", "Aria").await;
        sync(&a, &folder).await;
        // 업로드가 도중에 끊겨 내용이 깨진 다른 기기의 묶음 (이름은 정상)
        let broken = uuid::Uuid::new_v4().to_string();
        std::fs::write(folder.root().join(format!("devices/{broken}.scsync")), b"PK not a zip").unwrap();
        let r = sync(&b, &folder).await;
        assert_eq!(r.skipped_devices.len(), 1, "{r:?}");
        assert_eq!(r.skipped_devices[0].device_id, broken);
        assert!(!r.skipped_devices[0].reason.is_empty());
        assert_eq!(names(&b).await, vec!["Aria"], "깨진 묶음 때문에 정상 기기와의 동기화가 막히지 않음");
        // 쓰는 중인 파일(.partial)은 보이지도 않는다
        std::fs::write(folder.root().join(format!("devices/{}.scsync.partial", uuid::Uuid::new_v4())), b"half").unwrap();
        let r = sync(&b, &folder).await;
        assert_eq!(r.skipped_devices.len(), 1);
    }

    #[tokio::test]
    async fn a_corrupted_file_in_the_folder_stops_before_the_database_changes_and_resumes_after_repair() {
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("Drive");
        std::fs::create_dir_all(&shared).unwrap();
        let folder = FolderTarget::new(&shared).unwrap();
        let (a, b) = (dev(root.path(), "a").await, dev(root.path(), "b").await);
        let avatar = a.base.join("characters/avatars/x.png");
        std::fs::create_dir_all(avatar.parent().unwrap()).unwrap();
        std::fs::write(&avatar, b"png-bytes").unwrap();
        sqlx::query("INSERT INTO characters (id, name, avatar, created_at, updated_at) VALUES ('c1', 'Aria', ?, 1, 0)")
            .bind(avatar.to_string_lossy().to_string())
            .execute(&a.db)
            .await
            .unwrap();
        sync(&a, &folder).await;
        let blob = std::fs::read_dir(folder.root().join("blobs")).unwrap().next().unwrap().unwrap().path();
        let good = std::fs::read(&blob).unwrap();
        std::fs::write(&blob, b"half-written").unwrap();

        // 내용이 해시와 다른 파일: 멈추고, 행은 하나도 합치지 않음 (파일보다 DB를 먼저 바꾸지 않으므로)
        tokio::time::sleep(std::time::Duration::from_millis(4)).await;
        let err = run(&b.db, &b.base, &mut Map::new(), &b.id, &folder, &|_| {}).await.err().expect("멈춰야 함");
        assert!(err.to_string().contains("해시와 다름"), "{err}");
        assert!(names(&b).await.is_empty(), "DB는 그대로");
        assert!(!b.base.join("characters/avatars/x.png").exists() || std::fs::read(b.base.join("characters/avatars/x.png")).unwrap() != b"half-written");

        // 고쳐지면 다음 동기화에서 이어진다
        std::fs::write(&blob, good).unwrap();
        let r = sync(&b, &folder).await;
        assert_eq!((r.merged.len(), r.files_down), (1, 1), "{r:?}");
        assert_eq!(names(&b).await, vec!["Aria"]);
    }
}
