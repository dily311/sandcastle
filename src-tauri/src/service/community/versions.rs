//! 플러그인의 판 폴더 (CLAUDE.md 1-F-1, ADR-012 갱신).
//!
//! 플러그인 `a`의 폴더는 `plugins/a/<판>/` 하위 폴더들이다 (예: `plugins/a/1.0.0/`, `plugins/a/1.1.0/`). 업데이트하면 새 판 폴더가 생기고
//! 옛 판은 그대로 남는다 — 사용자가 어느 판을 쓸지 고르고(되돌리기), 설정 > 데이터에서 쓰지 않는 판 폴더를 지운다.
//! 지금 쓰는 판은 `user_plugins.active_dir`. 판 폴더에는 그 판을 떠날 때의 플러그인 저장소를 `.storage.json`으로 보관한다(코드 지문에서 빠짐 —
//! 점으로 시작하는 파일은 읽지 않는다).
//!
//! 예전 배치(`plugins/a/manifest.json`)는 처음 만나는 순간 `migrate_layout`이 한 판 폴더로 옮긴다.
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::{manifest, package, package::MANIFEST, plugins_dir};
use crate::error::{AppError, Result};

/// 판 폴더에 보관하는 저장소 파일 (점으로 시작해 패키지·코드 지문에서 제외됨)
pub const STORAGE_SNAPSHOT: &str = ".storage.json";

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginVersion {
    /// 폴더 이름 (판 문자열을 폴더 이름으로 쓸 수 있게 고친 것)
    pub dir: String,
    /// manifest의 version (읽지 못하면 폴더 이름)
    pub version: String,
    pub size: i64,
    /// 폴더를 만든(마지막으로 쓴) 시각 (ms)
    pub modified_at: i64,
    /// 지금 쓰는 판
    pub active: bool,
    /// manifest를 읽지 못한 이유 (그런 판은 고를 수 없고 지울 수만 있음)
    pub error: Option<String>,
    /// 이 판의 저장소 보관본이 있음 (이 판으로 돌아가면 그때 값으로 복원)
    pub has_storage: bool,
}

/// 판 문자열 → 폴더 이름: 영문·숫자·`.`·`_`·`+`·`-`만, 점으로 시작하지 않게, 비면 `v`
pub fn dir_name(version: &str) -> String {
    let s: String = version
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-') {
                c
            } else {
                '_'
            }
        })
        .take(40)
        .collect();
    let s = s.trim_start_matches('.').to_string();
    if s.is_empty() {
        "v".into()
    } else {
        s
    }
}

pub fn valid_dir_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 60 && !name.starts_with('.') && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}

/// 플러그인 폴더 안의 판 폴더들 (이름 순)
pub fn dirs_of(plugin_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(plugin_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| Some((e.file_name().to_str()?.to_string(), e.path())))
        .filter(|(n, _)| valid_dir_name(n))
        .collect();
    v.sort();
    v
}

/// 지금 쓸 판 폴더: 저장된 이름이 있으면 그것, 없거나 사라졌으면 판 문자열과 같은 이름, 그것도 없으면 가장 새 판
pub fn pick_active(dirs: &[(String, PathBuf)], saved_dir: &str, saved_version: &str) -> Option<(String, PathBuf)> {
    if let Some(d) = dirs.iter().find(|(n, _)| n == saved_dir && !saved_dir.is_empty()) {
        return Some(d.clone());
    }
    let by_version = dir_name(saved_version);
    if let Some(d) = dirs.iter().find(|(n, _)| *n == by_version && !saved_version.is_empty()) {
        return Some(d.clone());
    }
    dirs.iter()
        .max_by(|a, b| {
            let (va, vb) = (read_version(&a.1).unwrap_or_default(), read_version(&b.1).unwrap_or_default());
            if manifest::version_newer(&va, &vb) {
                std::cmp::Ordering::Greater
            } else if manifest::version_newer(&vb, &va) {
                std::cmp::Ordering::Less
            } else {
                a.0.cmp(&b.0)
            }
        })
        .cloned()
}

fn read_version(dir: &Path) -> Option<String> {
    let raw = std::fs::read(dir.join(MANIFEST)).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&raw).ok()?;
    v.get("version")
        .and_then(serde_json::Value::as_str)
        .map(|s| manifest::clean(s).trim().to_string())
}

fn modified_ms(dir: &Path) -> i64 {
    std::fs::metadata(dir)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 판 목록 (화면용)
pub fn describe(id: &str, dirs: &[(String, PathBuf)], active: Option<&str>, size_of: impl Fn(&Path) -> i64) -> Vec<PluginVersion> {
    dirs.iter()
        .map(|(name, path)| {
            let (version, error) = match std::fs::read(path.join(MANIFEST))
                .map_err(|e| format!("manifest.json을 읽을 수 없음: {e}"))
                .and_then(|raw| manifest::parse(&raw, env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string()))
            {
                Ok(m) if m.id == id => (m.version, None),
                Ok(m) => (name.clone(), Some(format!("폴더의 플러그인 id({id})와 manifest의 id({})가 다름", m.id))),
                Err(e) => (read_version(path).unwrap_or_else(|| name.clone()), Some(e)),
            };
            PluginVersion {
                dir: name.clone(),
                version,
                size: size_of(path),
                modified_at: modified_ms(path),
                active: active == Some(name.as_str()),
                error,
                has_storage: path.join(STORAGE_SNAPSHOT).is_file(),
            }
        })
        .collect()
}

/// 예전 배치(`plugins/<id>/manifest.json`)를 판 폴더 하나로 옮긴다. 이미 옮겼거나 폴더가 비었으면 아무 일도 없다 (여러 번 불러도 안전)
pub fn migrate_layout(base: &Path) -> Result<u32> {
    let root = plugins_dir(base);
    let mut moved = 0;
    for e in std::fs::read_dir(&root).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || !manifest::valid_id(&name) || !e.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let dir = e.path();
        if !dir.join(MANIFEST).is_file() {
            continue;
        }
        let version = read_version(&dir).unwrap_or_else(|| "0".into());
        let tmp = dir.join(format!(".moving-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir(&tmp)?;
        for item in std::fs::read_dir(&dir)?.flatten() {
            if item.path() == tmp {
                continue;
            }
            std::fs::rename(item.path(), tmp.join(item.file_name()))
                .map_err(|err| AppError::io(format!("플러그인 폴더를 새 배치로 옮기지 못함 ({name}): {err}")))?;
        }
        let target = dir.join(dir_name(&version));
        std::fs::rename(&tmp, &target).map_err(|err| AppError::io(format!("플러그인 폴더를 새 배치로 옮기지 못함 ({name}): {err}")))?;
        moved += 1;
    }
    Ok(moved)
}

/// 이 판 폴더의 코드 지문 (보관 파일은 점 파일이라 빠짐)
pub fn hash_of(dir: &Path) -> Option<String> {
    package::folder_hash(dir)
}
