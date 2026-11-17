//! 커뮤니티 플러그인 런타임 (CLAUDE.md 6-D-2, ADR-012).
//!
//! - **도는 플러그인 목록**(메모리): 화면의 플러그인 호스트가 `start`를 부를 때 켜진 플러그인으로 채운다.
//!   부르기 전에는 비어 있다(Lazy Initialization). 끄기·지우기·업데이트는 곧바로 목록에서 뺀다.
//! - **서빙** (`plugin://localhost/…`, Windows/Android는 `http://plugin.localhost/…`): 목록에 있는 플러그인의
//!   폴더 안 파일만. `/<id>/__host`는 SDK와 main을 부르는 문서, `/__sdk.js`는 SDK.
//!   CSP는 플러그인 자기 경로만 허용하고 네트워크(connect-src)·하위 프레임·form을 막는다.
//! - **호출** (`call_user_plugin`): 메서드마다 필요한 권한(`API`)을 허락받았는지, 초당 호출 수, 오류 수를 본다.
//!   1분에 오류가 많으면 이번 실행 동안 멈춘다(suspended). 기록은 플러그인마다 최근 것만 메모리에.
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::http::{Request, Response, StatusCode};

use super::{manifest::API_VERSION, package, UserPlugin};
use crate::error::{AppError, Result};

/// 메서드 → 필요한 권한 (없으면 누구나). 여기 없는 메서드는 거절한다. 화면도 같은 표로 먼저 거른다.
pub const API: &[(&str, Option<&str>)] = &[
    ("log", None),
    ("report", None),
    ("app.info", None),
    ("storage.get", Some("storage")),
    ("storage.set", Some("storage")),
    ("storage.delete", Some("storage")),
    ("storage.keys", Some("storage")),
    // 데이터 읽기 (6-D-3)
    ("data.character.list", Some("data.read:character")),
    ("data.character.get", Some("data.read:character")),
    ("data.persona.list", Some("data.read:persona")),
    ("data.persona.get", Some("data.read:persona")),
    ("data.lorebook.list", Some("data.read:lorebook")),
    ("data.lorebook.get", Some("data.read:lorebook")),
    ("data.chat.list", Some("data.read:chat")),
    ("data.chat.messages", Some("data.read:chat")),
    // 고치기 제안 (6-D-3) — 원본은 사용자가 승인해야 바뀐다
    ("data.character.propose", Some("data.write:character")),
    ("data.persona.propose", Some("data.write:persona")),
    ("data.lorebook.propose", Some("data.write:lorebook")),
    ("data.chat.propose", Some("data.write:chat")),
];

pub fn required_permission(method: &str) -> Option<Option<&'static str>> {
    API.iter().find(|(m, _)| *m == method).map(|(_, p)| *p)
}

/// 초당 호출 상한
const MAX_CALLS_PER_SEC: usize = 100;
/// 이만큼 1분 안에 오류가 나면 멈춤
const MAX_ERRORS_PER_MIN: usize = 20;
const LOG_KEEP: usize = 200;
const MAX_LOG_CHARS: usize = 2000;

#[derive(Debug, Clone)]
struct Active {
    version: String,
    main: String,
    granted: BTreeSet<String>,
    dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginLogEntry {
    pub at: i64,
    /// info / warn / error
    pub level: String,
    pub message: String,
}

#[derive(Default)]
struct Health {
    calls: VecDeque<Instant>,
    errors: VecDeque<Instant>,
    suspended: bool,
    log: VecDeque<PluginLogEntry>,
}

static ACTIVE: LazyLock<Mutex<HashMap<String, Active>>> = LazyLock::new(Default::default);
static HEALTH: LazyLock<Mutex<HashMap<String, Health>>> = LazyLock::new(Default::default);

fn active() -> std::sync::MutexGuard<'static, HashMap<String, Active>> {
    ACTIVE.lock().unwrap_or_else(|e| e.into_inner())
}
fn health() -> std::sync::MutexGuard<'static, HashMap<String, Health>> {
    HEALTH.lock().unwrap_or_else(|e| e.into_inner())
}

/// 화면에 주는 도는 플러그인 정보
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub host_url: String,
    pub granted: Vec<String>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStart {
    pub plugins: Vec<RuntimePlugin>,
    /// 메서드 → 필요한 권한 (화면이 IPC 전에 거른다)
    pub api: HashMap<String, Option<String>>,
}

pub fn origin() -> &'static str {
    if cfg!(any(target_os = "windows", target_os = "android")) {
        "http://plugin.localhost"
    } else {
        "plugin://localhost"
    }
}

pub fn host_url(id: &str) -> String {
    format!("{}/{id}/__host", origin())
}

/// 켜진 플러그인으로 목록을 다시 채운다 (멈춘 플러그인은 빼고)
pub fn start(plugins: &[UserPlugin], base: &Path) -> RuntimeStart {
    let mut map = active();
    map.clear();
    let suspended: BTreeSet<String> = health().iter().filter(|(_, h)| h.suspended).map(|(k, _)| k.clone()).collect();
    let mut out = vec![];
    for p in plugins {
        let Some(m) = p.manifest.as_ref().filter(|_| p.enabled && !suspended.contains(&p.id)) else {
            continue;
        };
        let granted: BTreeSet<String> = p.granted.iter().cloned().collect();
        // 허락한 것보다 많은 권한을 요구하면(파일을 손으로 바꾼 경우 등) 돌리지 않는다
        if !m.permissions.iter().all(|x| granted.contains(x)) {
            log(&p.id, "error", "manifest의 권한이 허락한 것과 달라 시작하지 않음 — 다시 켜서 동의하세요");
            continue;
        }
        map.insert(
            p.id.clone(),
            Active {
                version: m.version.clone(),
                main: m.main.clone(),
                granted,
                dir: base.join(super::PLUGINS_DIR).join(&p.id),
            },
        );
        out.push(RuntimePlugin {
            id: p.id.clone(),
            name: m.name.clone(),
            version: m.version.clone(),
            host_url: host_url(&p.id),
            granted: m.permissions.clone(),
        });
    }
    RuntimeStart {
        plugins: out,
        api: API.iter().map(|(m, p)| (m.to_string(), p.map(str::to_string))).collect(),
    }
}

/// 모두 내림 (코어 플러그인을 껐을 때)
pub fn stop_all() {
    active().clear();
}

/// 하나 내림 (끄기·지우기·업데이트)
pub fn stop(id: &str) {
    active().remove(id);
}

pub fn is_running(id: &str) -> bool {
    active().contains_key(id)
}

pub fn log(id: &str, level: &str, message: &str) {
    let mut h = health();
    let e = h.entry(id.to_string()).or_default();
    e.log.push_back(PluginLogEntry {
        at: chrono::Utc::now().timestamp_millis(),
        level: level.to_string(),
        message: message.chars().take(MAX_LOG_CHARS).collect(),
    });
    while e.log.len() > LOG_KEEP {
        e.log.pop_front();
    }
}

pub fn log_of(id: &str) -> Vec<PluginLogEntry> {
    health().get(id).map(|h| h.log.iter().cloned().collect()).unwrap_or_default()
}

pub fn is_suspended(id: &str) -> bool {
    health().get(id).is_some_and(|h| h.suspended)
}

/// 멈춤을 풀어 다시 시작할 수 있게 (사용자가 다시 켤 때)
pub fn clear_suspension(id: &str) {
    if let Some(h) = health().get_mut(id) {
        h.suspended = false;
        h.errors.clear();
    }
}

fn prune(q: &mut VecDeque<Instant>, window: Duration, now: Instant) {
    while q.front().is_some_and(|t| now.duration_since(*t) > window) {
        q.pop_front();
    }
}

/// 오류 하나 기록. 1분에 너무 많으면 멈추고 true
pub fn record_error(id: &str, message: &str) -> bool {
    log(id, "error", message);
    let now = Instant::now();
    let mut h = health();
    let e = h.entry(id.to_string()).or_default();
    e.errors.push_back(now);
    prune(&mut e.errors, Duration::from_secs(60), now);
    if !e.suspended && e.errors.len() >= MAX_ERRORS_PER_MIN {
        e.suspended = true;
        drop(h);
        stop(id);
        log(id, "error", "오류가 너무 많아 이번 실행 동안 멈춤");
        return true;
    }
    false
}

/// 호출 전 확인: 도는 중인가, 멈추지 않았나, 알려진 메서드인가, 권한, 초당 호출 수
pub fn check_call(id: &str, method: &str) -> Result<()> {
    let refuse = |msg: String| AppError::validation(msg, Some(serde_json::json!({ "plugin": id, "method": method })));
    let granted = match active().get(id) {
        Some(a) => a.granted.clone(),
        None => return Err(refuse(format!("플러그인이 실행 중이 아님: {id}"))),
    };
    let Some(need) = required_permission(method) else {
        return Err(refuse(format!("없는 메서드: {method}")));
    };
    if let Some(p) = need {
        if !granted.contains(p) {
            return Err(AppError::validation(
                format!("권한이 없음: {method}에는 '{p}' 권한이 필요"),
                Some(serde_json::json!({ "plugin": id, "method": method, "permission": p })),
            ));
        }
    }
    let now = Instant::now();
    let mut h = health();
    let e = h.entry(id.to_string()).or_default();
    if e.suspended {
        return Err(refuse(format!("멈춘 플러그인: {id}")));
    }
    prune(&mut e.calls, Duration::from_secs(1), now);
    if e.calls.len() >= MAX_CALLS_PER_SEC {
        return Err(refuse(format!("호출이 너무 잦음 (초당 {MAX_CALLS_PER_SEC}번까지)")));
    }
    e.calls.push_back(now);
    Ok(())
}

pub fn app_info(id: &str) -> serde_json::Value {
    let a = active().get(id).cloned();
    serde_json::json!({
        "appVersion": env!("CARGO_PKG_VERSION"),
        "apiVersion": API_VERSION,
        "pluginId": id,
        "version": a.as_ref().map(|a| a.version.clone()),
        "permissions": a.map(|a| a.granted.into_iter().collect::<Vec<_>>()).unwrap_or_default(),
    })
}

// ---------------------------------------------------------------------------
// 서빙
// ---------------------------------------------------------------------------

/// 플러그인 iframe에 넣는 SDK. 부모(앱 화면)에게만 postMessage로 묻고, 부모가 준 답만 받는다.
pub const SDK_JS: &str = r#"(()=>{"use strict";
let seq=0;const pending=new Map();
addEventListener("message",e=>{if(e.source!==parent)return;const d=e.data;if(!d||d.type!=="sc-plugin-reply")return;const p=pending.get(d.id);if(!p)return;pending.delete(d.id);clearTimeout(p.t);if(d.error){const err=new Error(String(d.error.message||"error"));err.code=d.error.code;p.reject(err)}else p.resolve(d.ok)});
const call=(method,args)=>new Promise((resolve,reject)=>{const id=++seq;const t=setTimeout(()=>{pending.delete(id);reject(new Error("응답 없음: "+method))},15000);pending.set(id,{resolve,reject,t});parent.postMessage({type:"sc-plugin-call",id,method:String(method),args:args===undefined?null:args},"*")});
const fmt=a=>a.map(x=>{if(typeof x==="string")return x;try{return JSON.stringify(x)}catch(_){return String(x)}}).join(" ").slice(0,2000);
addEventListener("error",e=>{call("report",{message:String(e.message||e.error||"error")}).catch(()=>{})});
addEventListener("unhandledrejection",e=>{const r=e.reason;call("report",{message:"처리 안 된 오류: "+String(r&&r.message||r)}).catch(()=>{})});
window.sandcastle=Object.freeze({apiVersion:1,call,
log:(...a)=>call("log",{level:"info",message:fmt(a)}),
warn:(...a)=>call("log",{level:"warn",message:fmt(a)}),
app:Object.freeze({info:()=>call("app.info")}),
storage:Object.freeze({get:k=>call("storage.get",{key:k}),set:(k,v)=>call("storage.set",{key:k,value:v===undefined?null:v}),delete:k=>call("storage.delete",{key:k}),keys:()=>call("storage.keys")}),
data:Object.freeze({
characters:Object.freeze({list:()=>call("data.character.list"),get:id=>call("data.character.get",{id}),propose:(id,changes,reason)=>call("data.character.propose",{id,changes,reason})}),
personas:Object.freeze({list:()=>call("data.persona.list"),get:id=>call("data.persona.get",{id}),propose:(id,changes,reason)=>call("data.persona.propose",{id,changes,reason})}),
lorebooks:Object.freeze({list:()=>call("data.lorebook.list"),get:id=>call("data.lorebook.get",{id}),propose:(id,changes,reason)=>call("data.lorebook.propose",{entity:"book",id,changes,reason}),proposeEntry:(id,changes,reason)=>call("data.lorebook.propose",{entity:"entry",id,changes,reason}),proposeNewEntry:(bookId,changes,reason)=>call("data.lorebook.propose",{entity:"entry",op:"create",parentId:bookId,changes,reason})}),
chats:Object.freeze({list:o=>call("data.chat.list",o||{}),messages:(conversationId,o)=>call("data.chat.messages",Object.assign({},o||{},{conversationId})),proposeMessage:(id,content,reason)=>call("data.chat.propose",{id,changes:{content},reason})})})});
})();"#;

fn csp(id: &str) -> String {
    let o = origin();
    let own = format!("{o}/{id}/");
    format!(
        "default-src 'none'; script-src {o}/__sdk.js {own}; style-src {own} 'unsafe-inline'; img-src {own} data: blob:; \
font-src {own} data:; media-src {own} data: blob:; connect-src 'none'; frame-src 'none'; form-action 'none'; base-uri 'none'; object-src 'none'"
    )
}

fn host_document(id: &str, main: &str) -> String {
    let o = origin();
    // main은 manifest 검사(safe_rel_path)를 통과한 상대 경로라 따옴표·꺾쇠가 없지만, 그래도 HTML 특수문자를 막는다
    let main: String = main.chars().filter(|c| !matches!(c, '"' | '<' | '>' | '&' | '\'')).collect();
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{id}</title>\
<script src=\"{o}/__sdk.js\"></script><script src=\"{o}/{id}/{main}\"></script></head><body></body></html>"
    )
}

fn content_type(rel: &str) -> &'static str {
    match Path::new(rel).extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("md" | "txt") => "text/plain; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        _ => "application/octet-stream",
    }
}

fn respond(status: StatusCode, ctype: &str, csp: Option<&str>, body: Vec<u8>) -> Response<Vec<u8>> {
    let mut b = Response::builder()
        .status(status)
        .header("Content-Type", ctype)
        .header("X-Content-Type-Options", "nosniff")
        .header("Cache-Control", "no-store");
    if let Some(c) = csp {
        b = b.header("Content-Security-Policy", c);
    }
    b.body(body).unwrap_or_else(|_| Response::new(Vec::new()))
}

fn not_found() -> Response<Vec<u8>> {
    respond(
        StatusCode::NOT_FOUND,
        "text/plain; charset=utf-8",
        Some("default-src 'none'"),
        b"not found".to_vec(),
    )
}

/// 프로토콜 처리. 도는 플러그인의 것만 준다
pub fn handle(req: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = req.uri().path().trim_start_matches('/');
    let path = percent_decode(path);
    if path == "__sdk.js" {
        return respond(StatusCode::OK, "text/javascript; charset=utf-8", None, SDK_JS.as_bytes().to_vec());
    }
    let Some((id, rel)) = path.split_once('/') else {
        return not_found();
    };
    let Some(a) = active().get(id).cloned() else {
        return not_found();
    };
    let csp = csp(id);
    if rel == "__host" {
        return respond(StatusCode::OK, "text/html; charset=utf-8", Some(&csp), host_document(id, &a.main).into_bytes());
    }
    if !super::manifest::safe_rel_path(rel) || !package::allowed(rel) || rel.split('/').any(|s| s.starts_with('.')) {
        return not_found();
    }
    let file = a.dir.join(rel);
    // 링크로 폴더 밖을 가리키지 못하게 실제 경로가 플러그인 폴더 안인지 본다
    let (Ok(real), Ok(root)) = (file.canonicalize(), a.dir.canonicalize()) else {
        return not_found();
    };
    if !real.starts_with(&root) || !real.is_file() {
        return not_found();
    }
    match std::fs::read(&real) {
        Ok(bytes) => respond(StatusCode::OK, content_type(rel), Some(&csp), bytes),
        Err(_) => not_found(),
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
pub fn reset_for_test() {
    active().clear();
    health().clear();
}
