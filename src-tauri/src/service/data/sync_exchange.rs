//! 파일로 주고받기 (CLAUDE.md 2-B-3, ADR-011): 묶음 만들기 / 미리보기 / 적용.
//!
//! - 만들기: 전부 담은 묶음(since 없음). 받는 쪽이 `sync_peers`로 이미 본 것을 건너뛰므로 같은 묶음을 여러 기기에 줘도 된다.
//!   파일 내용은 "파일 포함"을 고를 때만 (아니면 목록과 해시만 — 받는 쪽에 없는 파일 수를 알려줌).
//! - 미리보기: 같은 병합을 되돌리는 트랜잭션에서 돌려 숫자만 보여준다. DB·파일·설정을 바꾸지 않는다.
//! - 적용: 먼저 이 기기 DB와 settings.json 사본을 `sync-backups/`에 둔다(최근 3개). 받은 파일 중 **이 기기에 없는 것만**
//!   넣고(같은 경로에 내용이 다른 파일이 있으면 이 기기 것을 둔다), 그다음 DB를 합친다. 파일을 먼저 넣는 이유: 합친 행이
//!   가리키는 그림이 없는 순간이 생기지 않게. 실패하면 파일은 남지만 DB는 그대로다.
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Map, Value};
use sqlx::SqlitePool;

use super::sync_bundle::{self, BundleOptions, FileEntry};
use super::sync_merge::{self, BundleData, MergeReport};
use crate::error::Result;

pub const BACKUP_DIR: &str = "sync-backups";
const BACKUP_KEEP: usize = 3;

/// 묶음 하나의 요약 (화면용)
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncBundleInfo {
    pub device_id: String,
    pub created_at: i64,
    pub app_version: String,
    pub rows: u32,
    pub tombstones: u32,
    pub files: u32,
    pub blobs: u32,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FileCounts {
    /// 묶음에 내용이 있고 이 기기에 없어 넣을 (넣은) 파일
    pub added: u32,
    /// 이 기기에 없는데 묶음에 내용이 없는 파일 ("파일 포함"으로 다시 만들면 들어옴)
    pub missing: u32,
    /// 같은 경로에 내용이 다른 파일이 있어 이 기기 것을 둔 파일
    pub kept_local: u32,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncPreview {
    pub bundle: SyncBundleInfo,
    pub report: MergeReport,
    pub files: FileCounts,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncApplied {
    pub report: MergeReport,
    pub files: FileCounts,
    /// 적용 전 사본 (앱 데이터 폴더 기준)
    pub backup: String,
}

fn info(m: &sync_bundle::SyncManifest, bytes: u64) -> SyncBundleInfo {
    SyncBundleInfo {
        device_id: m.device_id.clone(),
        created_at: m.created_at,
        app_version: m.app_version.clone(),
        rows: m.tables.values().sum::<u64>() as u32,
        tombstones: m.tombstones as u32,
        files: m.files as u32,
        blobs: m.blobs as u32,
        bytes,
    }
}

pub async fn export(db: &SqlitePool, base: &Path, settings: &Map<String, Value>, device_id: &str, include_files: bool, out: &Path) -> Result<SyncBundleInfo> {
    let opts = BundleOptions { since: None, include_files };
    let m = sync_bundle::write_bundle(db, base, settings, device_id, &opts, out).await?;
    Ok(info(&m, std::fs::metadata(out).map(|x| x.len()).unwrap_or(0)))
}

/// 파일마다 할 일 (넣기 / 없음 / 이 기기 것 둠)
fn plan_files(base: &Path, files: &[FileEntry]) -> Result<(FileCounts, Vec<FileEntry>)> {
    let mut c = FileCounts::default();
    let mut to_add = vec![];
    for f in files {
        let dest = base.join(&f.path);
        if dest.exists() {
            if sync_bundle::sha256_file(&dest)? != f.hash {
                c.kept_local += 1;
            }
        } else if f.included {
            c.added += 1;
            to_add.push(f.clone());
        } else {
            c.missing += 1;
        }
    }
    Ok((c, to_add))
}

async fn open(db: &SqlitePool, base: &Path, path: &Path) -> Result<(sync_bundle::Bundle, BundleData)> {
    let local = sync_bundle::schema_version(db).await?;
    let mut b = sync_bundle::open(path, local, base)?;
    let data = BundleData::read(&mut b, db).await?;
    Ok((b, data))
}

pub async fn preview(db: &SqlitePool, base: &Path, settings: &Map<String, Value>, device_id: &str, path: &Path) -> Result<SyncPreview> {
    let (mut b, data) = open(db, base, path).await?;
    let mut scratch = settings.clone();
    let report = sync_merge::merge(db, &data, device_id, &mut scratch, true).await?;
    let (files, _) = plan_files(base, &b.files()?)?;
    Ok(SyncPreview {
        bundle: info(&data.manifest, std::fs::metadata(path).map(|x| x.len()).unwrap_or(0)),
        report,
        files,
    })
}

/// 적용. `settings`의 같이 쓰는 칸이 바뀐다 — 호출하는 쪽이 저장하고 화면에 알린다
pub async fn apply(db: &SqlitePool, base: &Path, settings: &mut Map<String, Value>, device_id: &str, path: &Path) -> Result<SyncApplied> {
    let (mut b, data) = open(db, base, path).await?;
    let backup = backup_before(db, base, settings).await?;

    let (files, to_add) = plan_files(base, &b.files()?)?;
    for f in &to_add {
        let dest = base.join(&f.path);
        let tmp = dest.with_extension("sync-partial");
        b.extract_blob(&f.hash, &tmp)?;
        std::fs::rename(&tmp, &dest)?;
    }
    let report = sync_merge::merge(db, &data, device_id, settings, false).await?;
    Ok(SyncApplied {
        report,
        files,
        backup: backup.strip_prefix(base).unwrap_or(&backup).to_string_lossy().replace('\\', "/"),
    })
}

/// 적용 전 사본: DB(VACUUM INTO)와 settings.json. 최근 BACKUP_KEEP개만
async fn backup_before(db: &SqlitePool, base: &Path, settings: &Map<String, Value>) -> Result<PathBuf> {
    let dir = base.join(BACKUP_DIR);
    std::fs::create_dir_all(&dir)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%3f");
    let out = dir.join(format!("app-{stamp}.db"));
    sqlx::query("VACUUM INTO ?").bind(out.to_string_lossy().to_string()).execute(db).await?;
    std::fs::write(
        dir.join(format!("settings-{stamp}.json")),
        serde_json::to_vec_pretty(settings).unwrap_or_default(),
    )?;
    prune(&dir);
    Ok(out)
}

fn prune(dir: &Path) {
    for prefix in ["app-", "settings-"] {
        let Ok(read) = std::fs::read_dir(dir) else { return };
        let mut files: Vec<PathBuf> = read
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(prefix)))
            .collect();
        files.sort();
        let extra = files.len().saturating_sub(BACKUP_KEEP);
        for p in files.into_iter().take(extra) {
            let _ = std::fs::remove_file(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn preview_changes_nothing_and_apply_backs_up_and_adds_files() {
        let dir = tempfile::tempdir().unwrap();
        let (base_a, base_b) = (dir.path().join("a"), dir.path().join("b"));
        let a = crate::db::conn::init_pool(base_a.clone()).await.unwrap();
        let b = crate::db::conn::init_pool(base_b.clone()).await.unwrap();
        let img = base_a.join("characters").join("c1").join("joy.png");
        std::fs::create_dir_all(img.parent().unwrap()).unwrap();
        std::fs::write(&img, b"joy").unwrap();
        sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c1', 'Aria', 1, 0)")
            .execute(&a)
            .await
            .unwrap();
        // B에 같은 경로, 다른 내용의 파일
        let clash = base_b.join("characters").join("c1").join("joy.png");
        std::fs::create_dir_all(clash.parent().unwrap()).unwrap();
        std::fs::write(&clash, b"b's own").unwrap();
        let img2 = base_a.join("generated").join("1.png");
        std::fs::create_dir_all(img2.parent().unwrap()).unwrap();
        std::fs::write(&img2, b"gen").unwrap();

        // 파일 없이 만든 묶음: 없는 파일 수만 알려줌
        let lite = dir.path().join("lite.scsync");
        let i = export(&a, &base_a, &Map::new(), "dev-a", false, &lite).await.unwrap();
        assert_eq!((i.files, i.blobs), (2, 0));
        let p = preview(&b, &base_b, &Map::new(), "dev-b", &lite).await.unwrap();
        assert_eq!((p.files.added, p.files.missing, p.files.kept_local), (0, 1, 1));
        assert!(p.report.dry_run && p.report.added >= 1);
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM characters").fetch_one(&b).await.unwrap();
        assert_eq!(n, 0, "미리보기는 바꾸지 않음");

        let full = dir.path().join("full.scsync");
        export(&a, &base_a, &Map::new(), "dev-a", true, &full).await.unwrap();
        let mut settings = Map::new();
        let r = apply(&b, &base_b, &mut settings, "dev-b", &full).await.unwrap();
        assert!(r.report.added >= 1 && !r.report.dry_run);
        assert_eq!((r.files.added, r.files.kept_local), (1, 1));
        assert_eq!(std::fs::read(base_b.join("generated/1.png")).unwrap(), b"gen");
        assert_eq!(std::fs::read(&clash).unwrap(), b"b's own", "이 기기 파일은 덮어쓰지 않음");
        assert!(r.backup.starts_with("sync-backups/app-"));
        assert!(base_b.join(&r.backup).exists());
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM characters WHERE id = 'c1'")
            .fetch_one(&b)
            .await
            .unwrap();
        assert_eq!(n, 1);

        // 사본은 최근 3개만
        for _ in 0..4 {
            backup_before(&b, &base_b, &settings).await.unwrap();
        }
        let count = std::fs::read_dir(base_b.join(BACKUP_DIR))
            .unwrap()
            .filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().starts_with("app-"))
            .count();
        assert_eq!(count, BACKUP_KEEP);
    }
}
