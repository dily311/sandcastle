//! 커뮤니티 플러그인 — 설치·목록·켜기·삭제 (CLAUDE.md 6-D-1, ADR-012).
//!
//! - 설치된 것 = `<앱 데이터>/plugins/<id>/` 폴더(manifest.json). 백업에는 들어가고(ADR-008) 동기화에는 빠진다.
//! - 기기마다의 상태(켜짐·허락한 권한·코드 해시)는 `user_plugins` 표. 행이 없으면 꺼짐·허락 없음으로 본다
//!   (예전 백업을 되살려 폴더만 있는 경우 등).
//! - 켜기는 manifest의 권한 전부에 대한 동의와 같이 온다. 권한이 늘어난 새 판을 설치하면 꺼진다(다시 동의).
//! - 삭제는 휴지통으로 (`TrashKind::UserPlugin`, 폴더도 같이).
//!
//! 플러그인 코드를 실행하는 것은 6-D-2(런타임)이다. 여기는 파일과 상태만 다룬다.
pub mod data;
pub mod manifest;
pub mod package;
pub mod postprocess;
pub mod prompt_hook;
pub mod runtime;
pub mod storage;

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde::Serialize;
use sqlx::SqlitePool;

use self::{manifest::PluginManifest, package::MANIFEST};
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
}

/// (name, version, enabled, granted, code_hash, installed_at, process_mode)
type StateRow = (String, String, i64, String, String, i64, String);

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
        sqlx::query_as("SELECT name, version, enabled, granted, code_hash, installed_at, process_mode FROM user_plugins WHERE id = ?")
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

fn not_subset(want: &[String], have: &[String]) -> Vec<String> {
    let have: BTreeSet<&String> = have.iter().collect();
    want.iter().filter(|p| !have.contains(p)).cloned().collect()
}

// ---------------------------------------------------------------------------
// 공개 함수
// ---------------------------------------------------------------------------

/// 설치된 플러그인 (폴더 이름 순)
pub async fn list(db: &SqlitePool, base: &Path) -> Result<Vec<UserPlugin>> {
    let mut out = vec![];
    let mut dirs: Vec<(String, PathBuf)> = std::fs::read_dir(plugins_dir(base))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| Some((e.file_name().to_str()?.to_string(), e.path())))
        // .staging-… / .old-…는 설치 도중의 것
        .filter(|(name, _)| !name.starts_with('.') && manifest::valid_id(name))
        .collect();
    dirs.sort();
    for (id, dir) in dirs {
        let state = state_of(db, &id).await?;
        let (manifest, error) = match read_installed_manifest(&dir, &id) {
            Ok(m) => (Some(m), None),
            Err(e) => (None, Some(e)),
        };
        let (enabled, granted, installed_at, process_mode) =
            state
                .map(|s| (s.2 != 0, granted_of(&s.3), s.5, s.6))
                .unwrap_or((false, vec![], 0, postprocess::MODE_NOTES.to_string()));
        out.push(UserPlugin {
            id,
            enabled: enabled && manifest.is_some(),
            manifest,
            error,
            granted,
            installed_at,
            size: dir_size(&dir),
            process_mode,
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
    let pkg = package::read(path, app_version())?;
    let id = pkg.manifest.id.clone();
    let installed = plugin_dir(base, &id)?.is_dir();
    let state = state_of(db, &id).await?;
    let existing_manifest = if installed {
        read_installed_manifest(&plugin_dir(base, &id)?, &id).ok()
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
    let id = pkg.manifest.id.clone();
    let root = plugins_dir(base);
    std::fs::create_dir_all(&root)?;
    let target = plugin_dir(base, &id)?;

    // 1) 임시 폴더에 다 쓰고 2) 바꿔 끼운다 — 중간에 실패해도 있던 판은 그대로
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
    let old = root.join(format!(".old-{token}"));
    let had_old = target.exists();
    if had_old {
        if let Err(e) = std::fs::rename(&target, &old) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(AppError::io(format!("있던 플러그인을 옮길 수 없음 (사용 중인지 확인): {e}")));
        }
    }
    if let Err(e) = std::fs::rename(&staging, &target) {
        if had_old {
            let _ = std::fs::rename(&old, &target);
        }
        let _ = std::fs::remove_dir_all(&staging);
        return Err(AppError::io(format!("플러그인을 설치할 수 없음: {e}")));
    }

    let prev = state_of(db, &id).await?;
    let granted = prev.as_ref().filter(|_| had_old).map(|s| granted_of(&s.3)).unwrap_or_default();
    let keep_enabled = prev.as_ref().is_some_and(|s| s.2 != 0) && had_old && not_subset(&pkg.manifest.permissions, &granted).is_empty();
    let saved = sqlx::query(
        "INSERT INTO user_plugins (id, name, version, enabled, granted, code_hash, installed_at) VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, version = excluded.version, enabled = excluded.enabled,
           granted = excluded.granted, code_hash = excluded.code_hash, installed_at = excluded.installed_at",
    )
    .bind(&id)
    .bind(&pkg.manifest.name)
    .bind(&pkg.manifest.version)
    .bind(keep_enabled as i64)
    .bind(serde_json::to_string(&granted).unwrap_or_else(|_| "[]".into()))
    .bind(&pkg.hash)
    .bind(now_ms())
    .execute(db)
    .await;
    if let Err(e) = saved {
        // 상태를 못 남겼으면 파일도 원래대로
        let _ = std::fs::remove_dir_all(&target);
        if had_old {
            let _ = std::fs::rename(&old, &target);
        }
        return Err(e.into());
    }
    if had_old {
        let _ = std::fs::remove_dir_all(&old);
    }
    // 도는 중이면 내린다 — 화면이 목록이 바뀐 것을 알고 새 코드로 다시 시작한다
    runtime::stop(&id);
    get(db, base, &id).await
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
    sqlx::query(
        "INSERT INTO user_plugins (id, name, version, enabled, granted) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, version = excluded.version, enabled = excluded.enabled, granted = excluded.granted",
    )
    .bind(id)
    .bind(&m.name)
    .bind(&m.version)
    .bind(enabled as i64)
    .bind(serde_json::to_string(&granted).unwrap_or_else(|_| "[]".into()))
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
