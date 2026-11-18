//! 커뮤니티 플러그인의 `manifest.json` (CLAUDE.md 6-D-1, ADR-012).
//!
//! 여기서 거절하면 설치되지 않는다. 런타임(6-D-2)과 백엔드 권한 확인은 이 검사를 통과한 값만 본다.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, Result};

/// 앱이 지원하는 플러그인 SDK 판. 플러그인의 `apiVersion`이 이보다 크면 설치하지 않는다.
pub const API_VERSION: u32 = 1;

/// 데이터 권한이 다루는 종류 (`data.read:<종류>`, `data.write:<종류>`)
pub const DATA_KINDS: [&str; 4] = ["character", "persona", "lorebook", "chat"];

/// 종류 없는 권한
const SIMPLE_PERMISSIONS: [&str; 5] = ["messages.process", "prompt.modify", "ui.panel", "ui.command", "ai"];
const STORAGE: &str = "storage";

const MAX_ID: usize = 64;
const MAX_NAME: usize = 80;
const MAX_VERSION: usize = 32;
const MAX_DESCRIPTION: usize = 500;
const MAX_SETTINGS: usize = 50;
const MAX_PANELS: usize = 5;
const MAX_COMMANDS: usize = 20;
const MAX_TITLE: usize = 40;

fn valid_local_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 32 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// 플러그인 설정 칸 (6-D-8의 설정 화면이 이것으로 그린다)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginSettingField {
    pub key: String,
    /// text / number / toggle / select
    #[serde(rename = "type")]
    pub kind: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// select의 고를 것
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

/// 오른쪽 패널 탭 (`ui.panel`, 6-D-6). page는 플러그인 폴더 안의 html
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PanelDecl {
    pub id: String,
    pub title: String,
    pub page: String,
}

/// 명령 (`ui.command`, 6-D-6). 사용자가 누를 때만 main.js의 `sandcastle.on("command", …)`이 불린다
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CommandDecl {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    /// 플러그인 폴더 안의 JS (기본 main.js)
    #[serde(default = "default_main")]
    pub main: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub settings: Vec<PluginSettingField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_app_version: Option<String>,
    #[serde(default)]
    pub panels: Vec<PanelDecl>,
    #[serde(default)]
    pub commands: Vec<CommandDecl>,
}

fn default_main() -> String {
    "main.js".into()
}

fn bad(msg: impl Into<String>) -> AppError {
    AppError::validation(format!("플러그인 manifest.json: {}", msg.into()), None)
}

/// 소문자·숫자·`-`, 2~64자, 글자나 숫자로 시작
pub fn valid_id(id: &str) -> bool {
    let b = id.as_bytes();
    (2..=MAX_ID).contains(&b.len()) && b[0].is_ascii_alphanumeric() && b.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

/// 알려진 권한인가
pub fn known_permission(p: &str) -> bool {
    if SIMPLE_PERMISSIONS.contains(&p) || p == STORAGE {
        return true;
    }
    match p.split_once(':') {
        Some(("data.read" | "data.write", kind)) => DATA_KINDS.contains(&kind),
        _ => false,
    }
}

/// 플러그인 폴더 안의 상대 경로로 안전한가 (`/` 구분, `..`·절대 경로·드라이브·빈 조각 없음)
pub fn safe_rel_path(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= 200
        && !p.starts_with('/')
        && !p.contains('\\')
        && !p.contains(':')
        && !p.contains('\0')
        && p.split('/').all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

/// "1.2.3" 같은 판을 숫자로 견준다 (모자란 자리는 0, 숫자가 아닌 꼬리는 무시)
fn version_parts(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches('v')
        .split('.')
        .map(|s| s.chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0))
        .collect()
}

pub fn version_newer(a: &str, b: &str) -> bool {
    let (x, y) = (version_parts(a), version_parts(b));
    for i in 0..x.len().max(y.len()) {
        let (p, q) = (x.get(i).copied().unwrap_or(0), y.get(i).copied().unwrap_or(0));
        if p != q {
            return p > q;
        }
    }
    false
}

/// manifest.json을 읽고 검사한다. 권한은 정리(중복 제거·정렬)해서 돌려준다.
/// 사용자에게 보이는 글에서 제어 문자와 글자 방향을 바꾸는 문자·보이지 않는 문자를 뺀다 (이름이 다른 글자처럼 보이게 속이지 못하게)
pub fn clean(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() && !matches!(*c as u32, 0x061C | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 | 0x2066..=0x2069 | 0xFEFF))
        .collect()
}

pub fn parse(bytes: &[u8], app_version: &str) -> Result<PluginManifest> {
    let mut m: PluginManifest = serde_json::from_slice(bytes).map_err(|e| bad(format!("읽을 수 없음 ({e})")))?;
    m.name = clean(&m.name);
    m.version = clean(&m.version);
    m.author = clean(&m.author);
    m.description = clean(&m.description);
    for p in &mut m.panels {
        p.title = clean(&p.title);
    }
    for c in &mut m.commands {
        c.title = clean(&c.title);
    }
    for f in &mut m.settings {
        f.label = clean(&f.label);
        f.description = f.description.as_deref().map(clean);
        f.options = f.options.iter().map(|o| clean(o)).collect();
    }
    m.id = m.id.trim().to_string();
    m.name = m.name.trim().to_string();
    m.version = m.version.trim().to_string();
    m.author = m.author.trim().to_string();
    m.description = m.description.trim().to_string();
    m.main = m.main.trim().to_string();

    if !valid_id(&m.id) {
        return Err(bad(format!("id는 소문자·숫자·'-'로 2~{MAX_ID}자여야 함: {:?}", m.id)));
    }
    if m.name.is_empty() || m.name.chars().count() > MAX_NAME {
        return Err(bad(format!("name은 1~{MAX_NAME}자")));
    }
    if m.version.is_empty() || m.version.len() > MAX_VERSION {
        return Err(bad(format!("version은 1~{MAX_VERSION}자")));
    }
    if m.author.chars().count() > MAX_NAME || m.description.chars().count() > MAX_DESCRIPTION {
        return Err(bad("author·description이 너무 김"));
    }
    if m.api_version == 0 {
        return Err(bad("apiVersion이 없음"));
    }
    if m.api_version > API_VERSION {
        return Err(bad(format!(
            "이 앱이 지원하는 플러그인 API보다 새것 (apiVersion {} > {API_VERSION}) — 앱을 업데이트하세요",
            m.api_version
        )));
    }
    if let Some(min) = m.min_app_version.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if version_newer(min, app_version) {
            return Err(bad(format!("앱 {min} 이상이 필요함 (지금 {app_version})")));
        }
    }
    if !safe_rel_path(&m.main) || !m.main.ends_with(".js") {
        return Err(bad(format!("main은 플러그인 폴더 안의 .js 파일이어야 함: {:?}", m.main)));
    }

    let mut perms = BTreeSet::new();
    for p in &m.permissions {
        let p = p.trim();
        if !known_permission(p) {
            return Err(bad(format!("모르는 권한: {p:?}")));
        }
        perms.insert(p.to_string());
    }
    m.permissions = perms.into_iter().collect();

    if m.settings.len() > MAX_SETTINGS {
        return Err(bad(format!("settings는 {MAX_SETTINGS}개까지")));
    }
    let mut keys = BTreeSet::new();
    for f in &m.settings {
        if f.key.is_empty() || f.key.len() > MAX_ID || !f.key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return Err(bad(format!("settings의 key가 올바르지 않음: {:?}", f.key)));
        }
        if !keys.insert(f.key.clone()) {
            return Err(bad(format!("settings의 key가 겹침: {}", f.key)));
        }
        if !["text", "number", "toggle", "select"].contains(&f.kind.as_str()) {
            return Err(bad(format!("settings.{}의 type은 text·number·toggle·select 중 하나", f.key)));
        }
        if f.kind == "select" && f.options.is_empty() {
            return Err(bad(format!("settings.{}: select에는 options가 필요함", f.key)));
        }
        if f.label.trim().is_empty() || f.label.chars().count() > MAX_NAME {
            return Err(bad(format!("settings.{}의 label은 1~{MAX_NAME}자", f.key)));
        }
    }
    // 화면 (6-D-6): 선언한 것은 권한이 있어야 한다
    if !m.panels.is_empty() && !m.permissions.iter().any(|p| p == "ui.panel") {
        return Err(bad("panels를 쓰려면 'ui.panel' 권한이 필요함"));
    }
    if !m.commands.is_empty() && !m.permissions.iter().any(|p| p == "ui.command") {
        return Err(bad("commands를 쓰려면 'ui.command' 권한이 필요함"));
    }
    if m.panels.len() > MAX_PANELS || m.commands.len() > MAX_COMMANDS {
        return Err(bad(format!("panels는 {MAX_PANELS}개, commands는 {MAX_COMMANDS}개까지")));
    }
    let mut ids = BTreeSet::new();
    for p in &mut m.panels {
        p.title = p.title.trim().to_string();
        if !valid_local_id(&p.id) || !ids.insert(format!("panel:{}", p.id)) {
            return Err(bad(format!("panels의 id가 올바르지 않거나 겹침: {:?}", p.id)));
        }
        if p.title.is_empty() || p.title.chars().count() > MAX_TITLE {
            return Err(bad(format!("panels.{}의 title은 1~{MAX_TITLE}자", p.id)));
        }
        if !safe_rel_path(&p.page) || !p.page.ends_with(".html") {
            return Err(bad(format!("panels.{}의 page는 플러그인 폴더 안의 .html이어야 함", p.id)));
        }
    }
    for c in &mut m.commands {
        c.title = c.title.trim().to_string();
        if !valid_local_id(&c.id) || !ids.insert(format!("command:{}", c.id)) {
            return Err(bad(format!("commands의 id가 올바르지 않거나 겹침: {:?}", c.id)));
        }
        if c.title.is_empty() || c.title.chars().count() > MAX_TITLE {
            return Err(bad(format!("commands.{}의 title은 1~{MAX_TITLE}자", c.id)));
        }
    }
    Ok(m)
}
