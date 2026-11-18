//! 전체 백업 (.scbackup) / 복원
//!
//! ```text
//! manifest.json            형식/버전/만든 시각/스키마(마이그레이션 버전·체크섬)/개수
//! app.db                   DB 스냅샷 (VACUUM INTO — 실행 중에도 일관된 사본)
//! settings.json            앱 설정 (API 키는 OS keyring에 있으므로 포함되지 않음)
//! files/characters/**      아바타, 채팅 이미지
//! files/games/**           게임 패키지
//! files/themes/**          테마
//! files/generated/**       갤러리 그림 (만든·올린 그림과 정보 .json)  — v2부터
//! files/backgrounds/**     올린 배경 (테마·폰 배경)                   — v2부터
//! ```
//!
//! 무엇이 사용자 데이터인지는 ADR-008의 표. 캐시(음성·앨범 아트 = app_cache_dir), tmp, exports,
//! restore-* 폴더는 넣지 않는다. API 키는 OS keyring에만 있어 어떤 경우에도 들어가지 않는다.
//!
//! ## 복원 순서
//! 1. `prepare_restore`: 백업을 `restore-staging/`에 풀고 검사한다
//!    (zip 경로, DB 무결성, 스키마 호환성). 통과하면 `READY.json`을 마지막에 쓴다.
//! 2. 앱 재시작
//! 3. `apply_pending_restore` (시작 시 DB를 열기 전): 현재 데이터를 `restore-backups/<시각>/`으로
//!    옮겨 두고(지우지 않음) 스테이징을 제자리로 옮긴다.
//! 4. `finish_restore_fixups` (DB 연 뒤): 다른 PC의 앱 데이터 경로로 저장된 아바타 경로를 고친다.
//!
//! 실행 중에 DB 파일을 바꾸지 않는 이유: 커넥션 풀·작업 관리자 등이 열어 둔 파일을 바꾸면
//! WAL이 섞이거나 Windows에서 파일 잠금으로 실패한다.
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use uuid::Uuid;

use crate::{
    db::conn::{latest_schema_version, schema_migrations, DbPool},
    error::{AppError, ErrorCode, Result},
};

use super::{
    archive::{extract_file, extract_prefix, open_archive, read_json, write_dir, write_file, write_json, zip_err},
    overview::{totals, DataTotals},
};

pub const BACKUP_FORMAT: &str = "sandcastle-backup";
/// 2: generated·backgrounds 추가, manifest.dirs / 3: plugins(커뮤니티 플러그인, 6-D-1) 추가
pub const BACKUP_VERSION: u32 = 3;
/// 백업에 들어가는 폴더 (album_arts 같은 캐시, exports, tmp는 제외). trash는 DB의 휴지통 항목과 짝이라 같이 담는다 (2-A-3).
/// plugins는 사용자가 설치한 플러그인 코드 (켜짐·허락은 DB의 user_plugins — 같이 복원된다)
pub const BACKUP_DIRS: [&str; 7] = ["characters", "games", "themes", "generated", "backgrounds", "trash", "plugins"];
/// v1 백업에 들어 있던 폴더 (manifest.dirs가 없을 때)
const V1_DIRS: [&str; 3] = ["characters", "games", "themes"];
const DB_FILE: &str = "app.db";
const SETTINGS_FILE: &str = "settings.json";
const STAGING_DIR: &str = "restore-staging";
const READY_FILE: &str = "READY.json";
const FIXUP_FILE: &str = "restore-fixup.json";
const KEEP_DIR: &str = "restore-backups";

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MigrationInfo {
    pub version: i64,
    /// 16진수 체크섬
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub format: String,
    pub version: u32,
    pub app_version: String,
    pub created_at: i64,
    pub schema_version: i64,
    pub migrations: Vec<MigrationInfo>,
    /// 만든 PC의 앱 데이터 경로 (복원 시 절대 경로 보정용)
    pub app_data_dir: String,
    pub totals: DataTotals,
    /// 백업에 든 폴더. 복원은 이 폴더만 바꾸고 나머지는 그대로 둔다 (v1 백업은 비어 있음 → V1_DIRS)
    #[serde(default)]
    pub dirs: Vec<String>,
}

impl BackupManifest {
    /// 이 백업이 담은 폴더 (BACKUP_DIRS 안의 것만 — 모르는 이름은 무시)
    pub fn included_dirs(&self) -> Vec<&'static str> {
        if self.dirs.is_empty() {
            return V1_DIRS.to_vec();
        }
        BACKUP_DIRS.into_iter().filter(|d| self.dirs.iter().any(|x| x == d)).collect()
    }
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub path: String,
    pub bytes: u64,
    pub file_count: u64,
    pub totals: DataTotals,
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub created_at: i64,
    pub app_version: String,
    pub totals: DataTotals,
    /// 백업이 이 앱보다 예전 스키마라 시작할 때 마이그레이션이 이어서 실행되는지
    pub will_migrate: bool,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

async fn read_migrations(db: &DbPool) -> Result<Vec<MigrationInfo>> {
    let rows: Vec<(i64, Vec<u8>)> = sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations WHERE success = 1 ORDER BY version")
        .fetch_all(db)
        .await?;
    Ok(rows.into_iter().map(|(version, c)| MigrationInfo { version, checksum: hex(&c) }).collect())
}

// =======================================
// 내보내기
// =======================================

pub async fn export_backup(db: &DbPool, app_data_dir: &Path, out: &Path) -> Result<BackupSummary> {
    let tmp_dir = app_data_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir)?;
    let snapshot = tmp_dir.join(format!("{}.db", Uuid::new_v4()));

    let result = async {
        sqlx::query("VACUUM INTO ?").bind(snapshot.to_string_lossy().to_string()).execute(db).await?;

        let manifest = BackupManifest {
            format: BACKUP_FORMAT.into(),
            version: BACKUP_VERSION,
            app_version: env!("CARGO_PKG_VERSION").into(),
            created_at: chrono::Utc::now().timestamp_millis(),
            schema_version: latest_schema_version(),
            migrations: read_migrations(db).await?,
            app_data_dir: app_data_dir.to_string_lossy().to_string(),
            totals: totals(db).await?,
            dirs: BACKUP_DIRS.iter().map(|d| d.to_string()).collect(),
        };

        let (base, out_path, snap, m) = (app_data_dir.to_path_buf(), out.to_path_buf(), snapshot.clone(), manifest.clone());
        let file_count = tokio::task::spawn_blocking(move || -> Result<u64> {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&out_path)?);
            write_json(&mut zip, "manifest.json", &m)?;
            write_file(&mut zip, DB_FILE, &snap)?;
            let mut count = 1;
            let settings = base.join(SETTINGS_FILE);
            if settings.is_file() {
                write_file(&mut zip, SETTINGS_FILE, &settings)?;
                count += 1;
            }
            for dir in BACKUP_DIRS {
                count += write_dir(&mut zip, &base.join(dir), &format!("files/{dir}"))?.0;
            }
            zip.finish().map_err(zip_err)?;
            Ok(count)
        })
        .await
        .map_err(|e| AppError::internal(e.to_string()))??;

        Ok(BackupSummary {
            bytes: std::fs::metadata(out).map(|x| x.len()).unwrap_or(0),
            path: out.to_string_lossy().to_string(),
            file_count,
            totals: manifest.totals,
        })
    }
    .await;

    let _ = std::fs::remove_file(&snapshot);
    if result.is_err() {
        let _ = std::fs::remove_file(out);
    }
    result
}

// =======================================
// 복원 준비
// =======================================

fn staging(base: &Path) -> PathBuf {
    base.join(STAGING_DIR)
}

pub fn cancel_restore(app_data_dir: &Path) {
    let _ = std::fs::remove_dir_all(staging(app_data_dir));
}

/// 재시작을 기다리는 복원이 있으면 그 백업 정보
pub fn pending_restore(app_data_dir: &Path) -> Option<BackupManifest> {
    let raw = std::fs::read(staging(app_data_dir).join(READY_FILE)).ok()?;
    serde_json::from_slice(&raw).ok()
}

pub async fn prepare_restore(app_data_dir: &Path, backup: &Path) -> Result<RestorePreview> {
    let stage = staging(app_data_dir);
    let result = prepare_inner(&stage, backup).await;
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&stage);
    }
    result
}

async fn prepare_inner(stage: &Path, backup: &Path) -> Result<RestorePreview> {
    let _ = std::fs::remove_dir_all(stage);
    std::fs::create_dir_all(stage)?;

    // 1. 풀기 (zip 경로 검사 포함)
    let (st, bk) = (stage.to_path_buf(), backup.to_path_buf());
    let manifest: BackupManifest = tokio::task::spawn_blocking(move || -> Result<BackupManifest> {
        let mut a = open_archive(&bk)?;
        let manifest: BackupManifest = read_json(&mut a, "manifest.json")?;
        if manifest.format != BACKUP_FORMAT {
            return Err(AppError::archive(ErrorCode::ArchiveInvalid, "Sandcastle 백업 파일이 아닙니다"));
        }
        if manifest.version > BACKUP_VERSION {
            return Err(AppError::archive(
                ErrorCode::ArchiveInvalid,
                "더 새 버전의 앱에서 만든 백업입니다. 앱을 업데이트해 주세요",
            ));
        }
        extract_file(&mut a, DB_FILE, &st.join(DB_FILE))?;
        if a.by_name(SETTINGS_FILE).is_ok() {
            extract_file(&mut a, SETTINGS_FILE, &st.join(SETTINGS_FILE))?;
        }
        for dir in manifest.included_dirs() {
            // 빈 폴더도 만들어 둔다: 백업에 비어 있던 폴더는 복원 뒤에도 비어 있어야 함
            std::fs::create_dir_all(st.join(dir))?;
            extract_prefix(&mut a, &format!("files/{dir}"), &st.join(dir))?;
        }
        Ok(manifest)
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))??;

    // 2. DB 검사: 무결성 + 스키마 호환
    let opts = SqliteConnectOptions::new().filename(stage.join(DB_FILE)).read_only(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .map_err(|e| AppError::archive(ErrorCode::ArchiveCorrupted, format!("백업 DB를 열 수 없습니다: {e}")))?;
    let check = async {
        let ok: String = sqlx::query_scalar("PRAGMA quick_check").fetch_one(&pool).await?;
        if ok != "ok" {
            return Err(AppError::archive(ErrorCode::ArchiveCorrupted, format!("백업 DB가 손상되었습니다: {ok}")));
        }
        check_schema(&read_migrations(&pool).await?)
    }
    .await;
    pool.close().await;
    check?;

    // 3. 준비 완료 표시 (마지막에 써서, 중간에 실패한 스테이징은 적용되지 않게)
    std::fs::write(stage.join(READY_FILE), serde_json::to_vec_pretty(&manifest)?)?;

    Ok(RestorePreview {
        will_migrate: manifest.schema_version < latest_schema_version(),
        created_at: manifest.created_at,
        app_version: manifest.app_version,
        totals: manifest.totals,
    })
}

/// 백업 DB에 기록된 마이그레이션이 이 빌드와 맞는지.
/// - 이 앱에 없는 버전 → 더 새 앱의 백업
/// - 체크섬이 다름 → 같은 번호의 마이그레이션 내용이 바뀜 (그대로 열면 sqlx가 시작을 거부함)
pub fn check_schema(backup: &[MigrationInfo]) -> Result<()> {
    let app = schema_migrations();
    for m in backup {
        match app.iter().find(|(v, _)| *v == m.version) {
            None => {
                return Err(AppError::archive(
                    ErrorCode::ArchiveInvalid,
                    format!("이 앱에 없는 DB 버전({})의 백업입니다. 앱을 업데이트해 주세요", m.version),
                ))
            }
            Some((_, checksum)) if hex(checksum) != m.checksum => {
                return Err(AppError::archive(
                    ErrorCode::ArchiveInvalid,
                    format!("DB 구조가 다른 빌드에서 만든 백업이라 복원할 수 없습니다 (마이그레이션 {})", m.version),
                ))
            }
            _ => {}
        }
    }
    Ok(())
}

// =======================================
// 시작 시 적용
// =======================================

/// 앱 시작 시 DB를 열기 전에 호출. 준비된 복원이 있으면 적용하고 true.
pub fn apply_pending_restore(app_data_dir: &Path) -> Result<bool> {
    let stage = staging(app_data_dir);
    let Some(manifest) = pending_restore(app_data_dir) else {
        // READY 없는 스테이징은 실패한 준비의 잔재
        if stage.exists() {
            let _ = std::fs::remove_dir_all(&stage);
        }
        return Ok(false);
    };

    let keep = app_data_dir.join(KEEP_DIR).join(chrono::Local::now().format("%Y%m%d-%H%M%S").to_string());
    std::fs::create_dir_all(&keep)?;

    // 옮긴 것 기록 (중간에 실패하면 되돌림)
    let mut moved: Vec<(PathBuf, PathBuf)> = vec![];
    let mv = |from: PathBuf, to: PathBuf, moved: &mut Vec<(PathBuf, PathBuf)>| -> Result<()> {
        if from.exists() {
            std::fs::rename(&from, &to)?;
            moved.push((from, to));
        }
        Ok(())
    };

    // 백업에 든 폴더만 바꾼다 (v1 백업이면 갤러리·배경은 지금 것을 그대로 둠)
    let dirs = manifest.included_dirs();
    let result = (|| -> Result<()> {
        // 현재 데이터 치우기 (WAL/SHM도 같이: 남겨 두면 새 DB에 엉뚱한 로그가 적용된다)
        for name in ["app.db", "app.db-wal", "app.db-shm", SETTINGS_FILE].into_iter().chain(dirs.iter().copied()) {
            mv(app_data_dir.join(name), keep.join(name), &mut moved)?;
        }
        // 백업 넣기
        for name in [DB_FILE, SETTINGS_FILE].into_iter().chain(dirs.iter().copied()) {
            mv(stage.join(name), app_data_dir.join(name), &mut moved)?;
        }
        Ok(())
    })();

    if let Err(e) = result {
        for (from, to) in moved.into_iter().rev() {
            let _ = std::fs::rename(&to, &from);
        }
        return Err(AppError::io(format!("백업 복원 적용 실패 (원래 데이터로 되돌림): {e}")));
    }

    std::fs::write(
        app_data_dir.join(FIXUP_FILE),
        serde_json::to_vec(&serde_json::json!({ "oldAppDataDir": manifest.app_data_dir }))?,
    )?;
    let _ = std::fs::remove_dir_all(&stage);
    Ok(true)
}

/// DB를 연 뒤 호출. 복원된 DB·설정 안의 다른 PC 앱 데이터 경로를 이 PC 경로로 바꾼다.
/// (아바타·커버·테마 배경·메시지 meta의 그림 경로 등 — 어느 칸이든 앱 데이터 경로로 시작하는 글자는 모두)
pub async fn finish_restore_fixups(db: &DbPool, app_data_dir: &Path) -> Result<()> {
    let fixup = app_data_dir.join(FIXUP_FILE);
    let Ok(raw) = std::fs::read(&fixup) else { return Ok(()) };
    // 복원한 백업 안의 커뮤니티 플러그인은 켜진 채 시작하지 않는다 (남이 만든 백업이 동의 없이 코드를 돌리지 않게, ADR-012).
    // 플러그인 폴더는 그대로 두고 켜짐·자동 AI 호출만 끈다 — 설정 > 커뮤니티 플러그인에서 다시 켜서 동의한다
    sqlx::query("UPDATE user_plugins SET enabled = 0, ai_auto = 0").execute(db).await?;
    let v: serde_json::Value = serde_json::from_slice(&raw).unwrap_or_default();
    let old = v["oldAppDataDir"].as_str().unwrap_or_default().to_string();
    let new = app_data_dir.to_string_lossy().to_string();

    if !old.is_empty() && old != new {
        let pairs = path_variants(&old, &new);
        replace_paths_in_db(db, &pairs).await?;
        let settings = app_data_dir.join(SETTINGS_FILE);
        if let Ok(text) = std::fs::read_to_string(&settings) {
            let fixed = pairs.iter().fold(text.clone(), |t, (o, n)| t.replace(o.as_str(), n));
            if fixed != text {
                std::fs::write(&settings, fixed)?;
            }
        }
    }
    let _ = std::fs::remove_file(fixup);
    Ok(())
}

/// 바꿀 경로 모양: 그대로 / JSON 안(역슬래시 두 개) / 슬래시로 쓴 것
fn path_variants(old: &str, new: &str) -> Vec<(String, String)> {
    let mut v = vec![(old.to_string(), new.to_string())];
    if old.contains('\\') {
        let esc = |s: &str| s.replace('\\', "\\\\");
        v.push((esc(old), esc(new)));
        v.push((old.replace('\\', "/"), new.replace('\\', "/")));
    }
    v
}

/// 모든 표의 글자 칸에서 옛 경로를 새 경로로 (마이그레이션 기록 표는 빼고)
async fn replace_paths_in_db(db: &DbPool, pairs: &[(String, String)]) -> Result<()> {
    let tables: Vec<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name <> '_sqlx_migrations'")
            .fetch_all(db)
            .await?;
    let mut tx = db.begin().await?;
    for table in tables {
        let q = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
        let cols: Vec<(String, String)> = sqlx::query_as("SELECT name, type FROM pragma_table_info(?)")
            .bind(&table)
            .fetch_all(&mut *tx)
            .await?;
        for (col, ty) in cols {
            let ty = ty.to_ascii_uppercase();
            if !(ty.is_empty() || ty.contains("TEXT") || ty.contains("CHAR") || ty.contains("JSON")) {
                continue;
            }
            for (o, n) in pairs {
                let sql = format!("UPDATE {t} SET {c} = REPLACE({c}, ?1, ?2) WHERE instr({c}, ?1) > 0", t = q(&table), c = q(&col));
                sqlx::query(sqlx::AssertSqlSafe(sql)).bind(o).bind(n).execute(&mut *tx).await?;
            }
        }
    }
    tx.commit().await?;
    Ok(())
}
