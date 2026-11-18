//! 커뮤니티 플러그인 — 설치·목록·켜기·삭제 (CLAUDE.md 6-D-1, ADR-012).
//!
//! - 설치된 것 = `<앱 데이터>/plugins/<id>/<판>/` 폴더들(각 판에 manifest.json, `versions.rs`). 지금 쓰는 판은 사용자가 고른다(되돌리기). 백업에는 들어가고(ADR-008) 동기화에는 빠진다.
//! - 기기마다의 상태(켜짐·허락한 권한·코드 해시)는 `user_plugins` 표. 행이 없으면 꺼짐·허락 없음으로 본다
//!   (예전 백업을 되살려 폴더만 있는 경우 등).
//! - 켜기는 manifest의 권한 전부에 대한 동의와 같이 온다. 권한이 늘어난 새 판을 설치하면 꺼진다(다시 동의).
//! - 삭제는 휴지통으로 (`TrashKind::UserPlugin`, 폴더도 같이).
//!
//! 플러그인 코드를 실행하는 것은 6-D-2(런타임)이다. 여기는 파일과 상태만 다룬다.
pub mod ai;
pub mod data;
pub mod manifest;
pub mod package;
pub mod postprocess;
pub mod prompt_hook;
pub mod runtime;
pub mod settings;
pub mod storage;
pub mod versions;

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde::Serialize;
use sqlx::SqlitePool;

use self::{manifest::PluginManifest, package::MANIFEST, versions::PluginVersion};
use crate::{
    error::{AppError, Result},
    service::data::trash::{self, TrashItem, TrashKind},
};

/// 앱 데이터 안의 플러그인 폴더 이름
pub const PLUGINS_DIR: &str = "plugins";

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPluginInfo {
    pub version: String,
    pub enabled: bool,
}

/// 설치 전 미리보기
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginPreview {
    pub manifest: PluginManifest,
    pub files: u32,
    pub size: i64,
    /// 설치할 때 그대로 넘긴다 (그 사이 파일이 바뀌면 설치하지 않음)
    pub hash: String,
    /// 같은 id가 이미 있으면 업데이트
    pub existing: Option<InstalledPluginInfo>,
    /// 지금 허락한 것보다 늘어난 권한 (새로 설치면 전부)
    pub added_permissions: Vec<String>,
    /// 있는 것보다 낮은 판
    pub downgrade: bool,
}

/// 목록의 한 줄. manifest를 읽지 못한 폴더도 보여준다(지울 수 있게)
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UserPlugin {
    pub id: String,
    pub manifest: Option<PluginManifest>,
    /// manifest를 읽지 못한 이유 (그런 플러그인은 켤 수 없음)
    pub error: Option<String>,
    pub enabled: bool,
    pub granted: Vec<String>,
    pub installed_at: i64,
    pub size: i64,
    /// 메시지 후처리: notes(표시만, 기본) / fix(자동 고침) — 6-D-4
    pub process_mode: String,
    /// 사용자가 누른 명령 밖에서 플러그인이 스스로 AI를 불러도 되나 (기본 꺼짐) — 6-D-7
    pub ai_auto: bool,
    /// 켤 때 본 코드와 폴더의 코드가 달라졌나 (파일을 손으로 고침·다른 곳에서 바뀜). 그러면 꺼진 것으로 보고 다시 켜서 동의해야 한다 (6-D-8)
    pub code_changed: bool,
    /// 지금 쓰는 판 폴더 이름 (`plugins/<id>/<이 이름>/`)
    pub active_dir: String,
    /// 폴더로 있는 판들 (쓰는 판 표시, 고르기·지우기에 쓰임)
    pub versions: Vec<PluginVersion>,
}

/// (name, version, enabled, granted, code_hash, installed_at, process_mode, ai_auto, active_dir)
type StateRow = (String, String, i64, String, String, i64, String, i64, String);

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn plugins_dir(base: &Path) -> PathBuf {
    base.join(PLUGINS_DIR)
}

fn plugin_dir(base: &Path, id: &str) -> Result<PathBuf> {
    if !manifest::valid_id(id) {
        return Err(AppError::validation(format!("플러그인 id가 올바르지 않음: {id:?}"), None));
    }
    Ok(plugins_dir(base).join(id))
}

async fn state_of(db: &SqlitePool, id: &str) -> Result<Option<StateRow>> {
    Ok(
        sqlx::query_as("SELECT name, version, enabled, granted, code_hash, installed_at, process_mode, ai_auto, active_dir FROM user_plugins WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await?,
    )
}

fn granted_of(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn dir_size(dir: &Path) -> i64 {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            match std::fs::symlink_metadata(e.path()) {
                Ok(m) if m.is_dir() => stack.push(e.path()),
                Ok(m) => total += m.len() as i64,
                Err(_) => {}
            }
        }
    }
    total
}

/// 설치된 폴더의 manifest를 읽는다 (폴더 이름과 id가 같아야 함)
fn read_installed_manifest(dir: &Path, id: &str) -> std::result::Result<PluginManifest, String> {
    let raw = std::fs::read(dir.join(MANIFEST)).map_err(|e| format!("manifest.json을 읽을 수 없음: {e}"))?;
    let m = manifest::parse(&raw, app_version()).map_err(|e| e.to_string())?;
    if m.id != id {
        return Err(format!("폴더 이름({id})과 manifest의 id({})가 다름", m.id));
    }
    Ok(m)
}

/// 저장된 상태로 쓸 판 폴더를 정한다 (판 폴더가 하나도 없으면 None)
fn active_of(base: &Path, id: &str, state: &Option<StateRow>) -> Option<(String, PathBuf)> {
    let dirs = versions::dirs_of(&plugins_dir(base).join(id));
    let (saved_dir, saved_version) = state.as_ref().map(|s| (s.8.as_str(), s.1.as_str())).unwrap_or(("", ""));
    versions::pick_active(&dirs, saved_dir, saved_version)
}

fn not_subset(want: &[String], have: &[String]) -> Vec<String> {
    let have: BTreeSet<&String> = have.iter().collect();
    want.iter().filter(|p| !have.contains(p)).cloned().collect()
}

// ---------------------------------------------------------------------------
// 공개 함수
// ---------------------------------------------------------------------------

/// 설치된 플러그인 (폴더 이름 순)
pub async fn list(db: &SqlitePool, base: &Path) -> Result<Vec<UserPlugin>> {
    versions::migrate_layout(base)?;
    let mut out = vec![];
    let mut dirs: Vec<(String, PathBuf)> = std::fs::read_dir(plugins_dir(base))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| Some((e.file_name().to_str()?.to_string(), e.path())))
        // .staging-…는 설치 도중의 것
        .filter(|(name, _)| !name.starts_with('.') && manifest::valid_id(name))
        .collect();
    dirs.sort();
    for (id, id_dir) in dirs {
        let state = state_of(db, &id).await?;
        let version_dirs = versions::dirs_of(&id_dir);
        let active = active_of(base, &id, &state);
        let active_dir = active.as_ref().map(|a| a.0.clone()).unwrap_or_default();
        let (manifest, error) = match &active {
            Some((_, d)) => match read_installed_manifest(d, &id) {
                Ok(m) => (Some(m), None),
                Err(e) => (None, Some(e)),
            },
            None => (None, Some("판 폴더가 없음".to_string())),
        };
        // 켤 때 저장한 코드 지문과 지금 폴더가 다르면 켜지 않은 것으로 본다 (읽을 수 없어도 같음)
        let code_changed = state
            .as_ref()
            .is_some_and(|s| s.2 != 0 && active.as_ref().and_then(|a| package::folder_hash(&a.1)).as_deref() != Some(s.4.as_str()));
        let (enabled, granted, installed_at, process_mode, ai_auto) = state
            .map(|s| (s.2 != 0 && !code_changed, granted_of(&s.3), s.5, s.6, s.7 != 0))
            .unwrap_or((false, vec![], 0, postprocess::MODE_NOTES.to_string(), false));
        out.push(UserPlugin {
            versions: versions::describe(&id, &version_dirs, Some(active_dir.as_str()).filter(|d| !d.is_empty()), dir_size),
            id,
            enabled: enabled && manifest.is_some(),
            manifest,
            error,
            granted,
            installed_at,
            size: active.as_ref().map(|a| dir_size(&a.1)).unwrap_or(0),
            process_mode,
            ai_auto,
            code_changed,
            active_dir,
        });
    }
    Ok(out)
}

pub async fn get(db: &SqlitePool, base: &Path, id: &str) -> Result<UserPlugin> {
    let dir = plugin_dir(base, id)?;
    if !dir.join(MANIFEST).exists() && !dir.is_dir() {
        return Err(AppError::not_found(format!("설치된 플러그인이 아님: {id}")));
    }
    list(db, base)
        .await?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| AppError::not_found(format!("설치된 플러그인이 아님: {id}")))
}

/// 설치 전 미리보기 — 아무것도 쓰지 않는다
pub async fn preview(db: &SqlitePool, base: &Path, path: &Path) -> Result<PluginPreview> {
    versions::migrate_layout(base)?;
    let pkg = package::read(path, app_version())?;
    let id = pkg.manifest.id.clone();
    let installed = plugin_dir(base, &id)?.is_dir();
    let state = state_of(db, &id).await?;
    let existing_manifest = if installed {
        active_of(base, &id, &state).and_then(|(_, d)| read_installed_manifest(&d, &id).ok())
    } else {
        None
    };
    let existing = installed.then(|| InstalledPluginInfo {
        version: existing_manifest
            .as_ref()
            .map(|m| m.version.clone())
            .or_else(|| state.as_ref().map(|s| s.1.clone()))
            .unwrap_or_default(),
        enabled: state.as_ref().is_some_and(|s| s.2 != 0),
    });
    let granted = state.as_ref().filter(|_| installed).map(|s| granted_of(&s.3)).unwrap_or_default();
    let downgrade = existing.as_ref().is_some_and(|e| manifest::version_newer(&e.version, &pkg.manifest.version));
    Ok(PluginPreview {
        added_permissions: not_subset(&pkg.manifest.permissions, &granted),
        files: pkg.files.len() as u32,
        size: pkg.size as i64,
        hash: pkg.hash,
        manifest: pkg.manifest,
        existing,
        downgrade,
    })
}

/// 설치 (또는 같은 id의 업데이트). `expected_hash`는 미리보기의 hash — 다르면 설치하지 않는다.
/// 업데이트에서 권한이 늘었으면 끈다(다시 켤 때 늘어난 권한까지 동의).
pub async fn install(db: &SqlitePool, base: &Path, path: &Path, expected_hash: &str) -> Result<UserPlugin> {
    let pkg = package::read(path, app_version())?;
    if pkg.hash != expected_hash {
        return Err(AppError::validation("미리보기 뒤에 플러그인 파일이 바뀌었음 — 다시 열어 확인하세요", None));
    }
    versions::migrate_layout(base)?;
    let id = pkg.manifest.id.clone();
    let root = plugins_dir(base);
    std::fs::create_dir_all(&root)?;
    let id_dir = plugin_dir(base, &id)?;
    let prev = state_of(db, &id).await?;
    let existing = versions::dirs_of(&id_dir);
    let had_old = !existing.is_empty();
    let prev_active = if had_old {
        versions::pick_active(
            &existing,
            prev.as_ref().map(|s| s.8.as_str()).unwrap_or(""),
            prev.as_ref().map(|s| s.1.as_str()).unwrap_or(""),
        )
    } else {
        None
    };

    // 1) 임시 폴더에 다 쓰고 2) 판 폴더로 옮긴다 — 중간에 실패해도 있던 판은 그대로 (옛 판은 지우지 않는다)
    let token = uuid::Uuid::new_v4().simple().to_string();
    let staging = root.join(format!(".staging-{token}"));
    let write = (|| -> Result<()> {
        for (rel, bytes) in &pkg.files {
            let dst = staging.join(rel);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&dst, bytes)?;
        }
        Ok(())
    })();
    if let Err(e) = write {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }
    // 판 폴더 이름: 판 문자열, 같은 이름에 다른 코드가 있으면 코드 지문을 붙인다. 같은 코드가 이미 있으면 그 폴더를 다시 쓴다
    let plain = versions::dir_name(&pkg.manifest.version);
    let short: String = pkg.hash.chars().take(8).collect();
    let (vdir, reuse) = match existing.iter().find(|(n, _)| *n == plain) {
        None => (plain, false),
        Some((_, p)) if versions::hash_of(p).as_deref() == Some(pkg.hash.as_str()) => (plain, true),
        Some(_) => {
            let alt = format!("{plain}-{short}");
            let reuse = existing.iter().any(|(n, _)| *n == alt);
            (alt, reuse)
        }
    };
    let target = id_dir.join(&vdir);
    if reuse {
        let _ = std::fs::remove_dir_all(&staging);
    } else {
        std::fs::create_dir_all(&id_dir)?;
        if let Err(e) = std::fs::rename(&staging, &target) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(AppError::io(format!("플러그인을 설치할 수 없음: {e}")));
        }
    }
    let switching = prev_active.as_ref().is_some_and(|(n, _)| *n != vdir);
    // 판이 바뀌면 떠나는 판의 저장소를 그 판 폴더에 보관한다 (그 판으로 돌아오면 그때 값으로 복원)
    if let (true, Some((_, from))) = (switching, prev_active.as_ref()) {
        if let Ok(snap) = storage::snapshot(db, &id).await {
            let _ = std::fs::write(from.join(versions::STORAGE_SNAPSHOT), serde_json::to_vec(&snap).unwrap_or_default());
        }
    }
    let perms = &pkg.manifest.permissions;
    let has = |p: &str| perms.iter().any(|x| x == p);
    // 업데이트(판 폴더가 있었음)일 때만 이전 상태를 이어받는다. 폴더 없이 행만 남은 id는 새로 시작한다 — 다른 사람의 같은 id 플러그인이
    // 이전 허락·자동 AI 호출·저장한 값을 물려받지 않게
    let carry = prev.as_ref().filter(|_| had_old);
    // 허락은 새 manifest가 요구하는 것과의 교집합으로만 (빠졌다가 다시 생긴 권한을 몰래 되살리지 않음)
    let granted: Vec<String> = carry
        .map(|s| granted_of(&s.3))
        .unwrap_or_default()
        .into_iter()
        .filter(|p| perms.contains(p))
        .collect();
    let keep_enabled = carry.is_some_and(|s| s.2 != 0) && not_subset(perms, &granted).is_empty();
    let ai_auto = carry.is_some_and(|s| s.7 != 0) && has("ai");
    let mode = carry
        .map(|s| s.6.clone())
        .filter(|_| has("messages.process"))
        .unwrap_or_else(|| postprocess::MODE_NOTES.to_string());
    if carry.is_none() {
        sqlx::query("DELETE FROM user_plugin_storage WHERE plugin_id = ?").bind(&id).execute(db).await?;
    } else if switching && reuse {
        // 예전에 쓰던 판을 다시 설치해 돌아온 것: 그 판의 저장소 보관본이 있으면 그것으로
        if let Some(snap) = std::fs::read(target.join(versions::STORAGE_SNAPSHOT))
            .ok()
            .and_then(|raw| serde_json::from_slice::<serde_json::Value>(&raw).ok())
        {
            storage::restore(db, &id, &snap).await?;
        }
    }
    let saved = sqlx::query(
        "INSERT INTO user_plugins (id, name, version, enabled, granted, code_hash, installed_at, process_mode, ai_auto, active_dir) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, version = excluded.version, enabled = excluded.enabled,
           granted = excluded.granted, code_hash = excluded.code_hash, installed_at = excluded.installed_at,
           process_mode = excluded.process_mode, ai_auto = excluded.ai_auto, active_dir = excluded.active_dir",
    )
    .bind(&id)
    .bind(&pkg.manifest.name)
    .bind(&pkg.manifest.version)
    .bind(keep_enabled as i64)
    .bind(serde_json::to_string(&granted).unwrap_or_else(|_| "[]".into()))
    .bind(&pkg.hash)
    .bind(now_ms())
    .bind(&mode)
    .bind(ai_auto as i64)
    .bind(&vdir)
    .execute(db)
    .await;
    if let Err(e) = saved {
        // 상태를 못 남겼으면 새로 만든 판 폴더는 치운다 (옛 판은 그대로)
        if !reuse {
            let _ = std::fs::remove_dir_all(&target);
        }
        return Err(e.into());
    }
    // 도는 중이면 내린다 — 화면이 목록이 바뀐 것을 알고 새 코드로 다시 시작한다
    runtime::stop(&id);
    get(db, base, &id).await
}

/// 쓰는 판 바꾸기(되돌리기 포함). 코드가 바뀌므로 **꺼진 채로** 돌아오고 다시 켜서 동의해야 한다. 허락은 그 판의 manifest가 요구하는 것과의 교집합,
/// 자동 AI 호출은 꺼짐. 떠나는 판의 저장소는 그 판 폴더에 보관하고, 가는 판에 보관본이 있으면 그것으로 복원한다.
pub async fn switch_version(db: &SqlitePool, base: &Path, id: &str, dir: &str) -> Result<UserPlugin> {
    let p = get(db, base, id).await?;
    let Some(target) = p.versions.iter().find(|v| v.dir == dir) else {
        return Err(AppError::not_found(format!("그런 판 폴더가 없음: {dir}")));
    };
    if target.active {
        return Ok(p);
    }
    if let Some(e) = &target.error {
        return Err(AppError::validation(format!("이 판은 쓸 수 없음: {e}"), None));
    }
    let id_dir = plugin_dir(base, id)?;
    let target_path = id_dir.join(dir);
    let m = read_installed_manifest(&target_path, id).map_err(|e| AppError::validation(format!("이 판은 쓸 수 없음: {e}"), None))?;
    runtime::stop(id);
    // 1) 떠나는 판의 저장소 보관 (2) 가는 판의 보관본 복원
    if !p.active_dir.is_empty() {
        let snap = storage::snapshot(db, id).await?;
        std::fs::write(
            id_dir.join(&p.active_dir).join(versions::STORAGE_SNAPSHOT),
            serde_json::to_vec(&snap).unwrap_or_default(),
        )?;
    }
    if let Some(snap) = std::fs::read(target_path.join(versions::STORAGE_SNAPSHOT))
        .ok()
        .and_then(|raw| serde_json::from_slice::<serde_json::Value>(&raw).ok())
    {
        storage::restore(db, id, &snap).await?;
    }
    let granted: Vec<String> = p.granted.iter().filter(|g| m.permissions.contains(g)).cloned().collect();
    let mode = if m.permissions.iter().any(|x| x == "messages.process") {
        p.process_mode.clone()
    } else {
        postprocess::MODE_NOTES.to_string()
    };
    sqlx::query(
        "UPDATE user_plugins SET name = ?, version = ?, enabled = 0, granted = ?, code_hash = '', process_mode = ?, ai_auto = 0, active_dir = ? WHERE id = ?",
    )
    .bind(&m.name)
    .bind(&m.version)
    .bind(serde_json::to_string(&granted).unwrap_or_else(|_| "[]".into()))
    .bind(&mode)
    .bind(dir)
    .bind(id)
    .execute(db)
    .await?;
    get(db, base, id).await
}

/// 쓰지 않는 판 폴더 지우기 (설정 > 데이터). 지금 쓰는 판은 지울 수 없다. 되돌릴 수 없음 — 화면이 확인을 받는다
pub async fn delete_version(db: &SqlitePool, base: &Path, id: &str, dir: &str) -> Result<UserPlugin> {
    let p = get(db, base, id).await?;
    let Some(v) = p.versions.iter().find(|v| v.dir == dir) else {
        return Err(AppError::not_found(format!("그런 판 폴더가 없음: {dir}")));
    };
    if v.active {
        return Err(AppError::validation("지금 쓰는 판은 지울 수 없음 — 다른 판을 고른 뒤에 지우세요", None));
    }
    std::fs::remove_dir_all(plugin_dir(base, id)?.join(dir))?;
    get(db, base, id).await
}

/// 켜기·끄기. 켤 때는 manifest의 권한 전부를 `grant`로 받아야 한다(화면이 보여주고 동의를 받은 목록)
pub async fn set_enabled(db: &SqlitePool, base: &Path, id: &str, enabled: bool, grant: Vec<String>) -> Result<UserPlugin> {
    let p = get(db, base, id).await?;
    let Some(m) = p.manifest.as_ref() else {
        return Err(AppError::validation(format!("켤 수 없는 플러그인: {}", p.error.unwrap_or_default()), None));
    };
    if enabled {
        let want: BTreeSet<&String> = m.permissions.iter().collect();
        let got: BTreeSet<&String> = grant.iter().collect();
        if want != got {
            return Err(AppError::validation("플러그인이 요청한 권한 전부에 동의해야 켤 수 있음", None));
        }
    }
    let granted = if enabled { m.permissions.clone() } else { p.granted.clone() };
    // 켤 때 폴더의 지금 코드를 기억한다 — 나중에 폴더가 바뀌면(code_changed) 다시 켜서 동의해야 한다
    let hash = if enabled {
        package::folder_hash(&plugin_dir(base, id)?.join(&p.active_dir)).ok_or_else(|| AppError::validation("플러그인 폴더를 읽을 수 없음", None))?
    } else {
        String::new()
    };
    sqlx::query(
        "INSERT INTO user_plugins (id, name, version, enabled, granted, code_hash, active_dir) VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, version = excluded.version, enabled = excluded.enabled, granted = excluded.granted,
           code_hash = CASE WHEN excluded.code_hash <> '' THEN excluded.code_hash ELSE user_plugins.code_hash END, active_dir = excluded.active_dir",
    )
    .bind(id)
    .bind(&m.name)
    .bind(&m.version)
    .bind(enabled as i64)
    .bind(serde_json::to_string(&granted).unwrap_or_else(|_| "[]".into()))
    .bind(&hash)
    .bind(&p.active_dir)
    .execute(db)
    .await?;
    if enabled {
        // 사용자가 다시 켜면 멈춤(오류가 많았던 것)을 푼다
        runtime::clear_suspension(id);
    } else {
        runtime::stop(id);
    }
    get(db, base, id).await
}

/// 지우기 = 휴지통으로 (폴더와 상태 같이). 되살리면 꺼진 채로 돌아온다
pub async fn uninstall(db: &SqlitePool, base: &Path, id: &str) -> Result<TrashItem> {
    prepare_uninstall(db, base, id).await?;
    trash::move_to_trash(db, base, TrashKind::UserPlugin, id).await
}

/// 휴지통에 넣기 전: 끄고, 행이 없으면(폴더만 있는 플러그인) 만든다 — 휴지통은 행을 떠서 옮긴다
pub async fn prepare_uninstall(db: &SqlitePool, base: &Path, id: &str) -> Result<()> {
    runtime::stop(id);
    let p = get(db, base, id).await?;
    let name = p.manifest.as_ref().map(|m| m.name.clone()).unwrap_or_else(|| id.to_string());
    let version = p.manifest.as_ref().map(|m| m.version.clone()).unwrap_or_default();
    sqlx::query(
        "INSERT INTO user_plugins (id, name, version, enabled) VALUES (?, ?, ?, 0)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, enabled = 0",
    )
    .bind(id)
    .bind(&name)
    .bind(&version)
    .execute(db)
    .await?;
    Ok(())
}

/// 플러그인 iframe의 호출 하나 (`sandcastle.*`). 권한·횟수 확인은 runtime, 실패는 플러그인 기록에 남고 오류 수에 들어간다
pub async fn call(db: &SqlitePool, id: &str, method: &str, args: &serde_json::Value) -> Result<serde_json::Value> {
    use serde_json::Value;
    if let Err(e) = runtime::check_call(id, method) {
        runtime::record_error(id, &e.to_string());
        return Err(e);
    }
    let arg = |k: &str| args.get(k).cloned().unwrap_or(Value::Null);
    let key = || arg("key").as_str().map(str::to_string).unwrap_or_default();
    let result = match method {
        "log" => {
            let level = if arg("level").as_str() == Some("warn") { "warn" } else { "info" };
            runtime::log(id, level, arg("message").as_str().unwrap_or(""));
            Ok(Value::Null)
        }
        "report" => {
            runtime::record_error(id, arg("message").as_str().unwrap_or("오류"));
            Ok(Value::Null)
        }
        "app.info" => Ok(runtime::app_info(id)),
        // 화면의 호스트가 직접 답한다 (백엔드는 화면 상태를 모름)
        "app.context" => Ok(Value::Null),
        "settings.get" => settings::get(db, id, &runtime::settings_decl(id)).await,
        // AI 호출은 작업·프로필이 필요해 커맨드(call_user_plugin)가 직접 처리한다 (6-D-7)
        "ai.generate" => Err(AppError::validation("ai.generate는 call_user_plugin 커맨드로만 부를 수 있음", None)),
        "storage.get" => storage::get(db, id, &key()).await,
        "storage.set" => storage::set(db, id, &key(), &arg("value")).await.map(|_| Value::Null),
        "storage.delete" => storage::delete(db, id, &key()).await.map(|_| Value::Null),
        "storage.keys" => storage::keys(db, id).await.map(Value::from),
        "data.character.list" => data::character_list(db).await,
        "data.character.get" => data::character_get(db, args).await,
        "data.persona.list" => data::persona_list(db).await,
        "data.persona.get" => data::persona_get(db, args).await,
        "data.lorebook.list" => data::lorebook_list(db).await,
        "data.lorebook.get" => data::lorebook_get(db, args).await,
        "data.chat.list" => data::chat_list(db, args).await,
        "data.chat.messages" => data::chat_messages(db, args).await,
        "data.character.propose" | "data.persona.propose" | "data.lorebook.propose" | "data.chat.propose" => propose(db, id, method, args).await,
        _ => Err(AppError::validation(format!("없는 메서드: {method}"), None)),
    };
    if let Err(e) = &result {
        runtime::record_error(id, &format!("{method}: {e}"));
    }
    result
}

#[cfg(test)]
mod runtime_tests;
/// 플러그인의 고치기 제안 → 사용자가 고를 때까지 원본은 그대로. 돌려주는 것은 제안 id와 상태
async fn propose(db: &SqlitePool, plugin: &str, method: &str, args: &serde_json::Value) -> Result<serde_json::Value> {
    use crate::service::proposals::{self, NewProposal, ProposalKind};
    use serde_json::Value;
    let s = |k: &str| args.get(k).and_then(Value::as_str);
    let entry = s("entity") == Some("entry");
    let kind = match method {
        "data.character.propose" => ProposalKind::Character,
        "data.persona.propose" => ProposalKind::Persona,
        "data.lorebook.propose" if entry => ProposalKind::LorebookEntry,
        "data.lorebook.propose" => ProposalKind::Lorebook,
        _ => ProposalKind::Message,
    };
    // 권한 표와 제안 종류가 어긋나지 않게 한 번 더 (lorebook_entry는 data.write:lorebook)
    debug_assert_eq!(runtime::required_permission(method), Some(Some(kind.write_permission())));
    let changes = args.get("changes").and_then(Value::as_object).cloned().unwrap_or_default();
    let source = format!("plugin:{plugin}");
    let p = proposals::propose(
        db,
        NewProposal {
            source: &source,
            kind,
            op: s("op").unwrap_or("update"),
            target_id: s("id"),
            parent_id: s("parentId"),
            changes: &changes,
            reason: s("reason").unwrap_or(""),
        },
    )
    .await?;
    Ok(serde_json::json!({ "id": p.id, "status": p.status }))
}

#[cfg(test)]
mod tests;
