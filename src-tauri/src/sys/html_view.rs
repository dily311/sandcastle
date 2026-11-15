//! AI 답변의 HTML 코드 블록을 iframe으로 보여주기 위한 서빙.
//!
//! 왜 srcdoc이 아니라 프로토콜인가: srcdoc/blob 문서는 앱 창의 CSP(script-src 'self')를 물려받아 인라인 JS가 막힌다.
//! 그래서 `htmlview://localhost/<id>`(Windows/Android는 `http://htmlview.localhost/<id>`)로 따로 서빙하고
//! 이 응답에만 자기 CSP를 붙인다.
//!
//! 격리:
//! - 화면은 iframe에 `sandbox="allow-scripts"`만 준다 (allow-same-origin 없음 → 불투명 origin).
//!   앱 창의 DOM·저장소·Tauri IPC에 닿지 못한다.
//! - CSP의 connect-src 'none'으로 fetch/XHR/WebSocket을 막는다 (IPC 요청도 여기서 막힌다). form 전송·하위 프레임도 막는다.
//! - 외부 https 이미지·글꼴·스타일·스크립트(CDN 라이브러리)는 허용한다. AI가 흔히 CDN을 쓰기 때문이다.
//! - 높이: 문서 끝에 작은 스크립트를 붙여 parent에 {type:"sc-html-height", h}를 postMessage 한다.
//!
//! 내용은 메모리에만 둔다 (같은 HTML은 같은 id, 최근 MAX_ENTRIES개만).
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};

use tauri::http::{Request, Response, StatusCode};

use crate::error::{AppError, Result};

const MAX_ENTRIES: usize = 300;
const MAX_BYTES: usize = 1024 * 1024;

const CSP: &str = "default-src 'none'; script-src 'unsafe-inline' https:; style-src 'unsafe-inline' https:; \
img-src data: blob: https:; font-src data: https:; media-src data: blob: https:; connect-src 'none'; \
form-action 'none'; frame-src 'none'; base-uri 'none'";

const HEIGHT_SCRIPT: &str = r#"<script>(()=>{const s=()=>parent.postMessage({type:"sc-html-height",h:Math.ceil(document.body?document.body.getBoundingClientRect().bottom+parseFloat(getComputedStyle(document.body).marginBottom||"0"):document.documentElement.scrollHeight)},"*");try{new ResizeObserver(s).observe(document.documentElement)}catch(e){}addEventListener("load",s);s();setTimeout(s,300);setTimeout(s,1000);})();</script>"#;

struct Store {
    map: HashMap<String, String>,
    order: VecDeque<String>,
}

static STORE: LazyLock<Mutex<Store>> = LazyLock::new(|| Mutex::new(Store { map: HashMap::new(), order: VecDeque::new() }));

fn id_of(html: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    html.hash(&mut h);
    format!("{:016x}{:x}", h.finish(), html.len())
}

/// 이 HTML을 보여줄 주소. 같은 HTML이면 같은 주소다.
pub fn register(html: &str) -> Result<String> {
    if html.len() > MAX_BYTES {
        return Err(AppError::validation("HTML이 너무 큽니다 (1MB까지)", None));
    }
    let id = id_of(html);
    let mut s = STORE.lock().unwrap_or_else(|e| e.into_inner());
    if !s.map.contains_key(&id) {
        s.map.insert(id.clone(), html.to_string());
        s.order.push_back(id.clone());
        while s.order.len() > MAX_ENTRIES {
            if let Some(old) = s.order.pop_front() {
                s.map.remove(&old);
            }
        }
    }
    Ok(url_of(&id))
}

pub fn url_of(id: &str) -> String {
    if cfg!(any(target_os = "windows", target_os = "android")) {
        format!("http://htmlview.localhost/{id}")
    } else {
        format!("htmlview://localhost/{id}")
    }
}

/// 조각이면 문서로 감싸고, 높이를 알려주는 스크립트를 붙인다.
pub fn document(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut doc = if lower.contains("<html") || lower.contains("<!doctype") {
        html.to_string()
    } else {
        format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<style>html,body{{margin:0;padding:8px;font-family:system-ui,sans-serif;color-scheme:light dark;}}</style></head><body>{html}</body></html>"
        )
    };
    match doc.to_ascii_lowercase().rfind("</body>") {
        Some(i) => doc.insert_str(i, HEIGHT_SCRIPT),
        None => doc.push_str(HEIGHT_SCRIPT),
    }
    doc
}

pub fn handle(req: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let id = req.uri().path().trim_start_matches('/').to_string();
    let html = STORE.lock().unwrap_or_else(|e| e.into_inner()).map.get(&id).cloned();
    let (status, body) = match html {
        Some(h) => (StatusCode::OK, document(&h)),
        None => (StatusCode::NOT_FOUND, "<p>이 HTML은 더 이상 없습니다. 메시지를 다시 열어 주세요.</p>".to_string()),
    };
    Response::builder()
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Content-Security-Policy", CSP)
        .header("X-Content-Type-Options", "nosniff")
        .header("Cache-Control", "no-store")
        .body(body.into_bytes())
        .unwrap_or_else(|_| Response::new(Vec::new()))
}

/// 화면이 HTML 코드 블록을 보여주기 전에 부른다. iframe의 src로 쓸 주소를 돌려준다.
#[tauri::command]
#[specta::specta]
pub fn register_html_view(app: tauri::AppHandle, html: String) -> Result<String> {
    crate::sys::plugins::require(&app, crate::sys::plugins::id::HTML)?;
    register(&html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_html_same_url_and_fragment_wrapped() {
        let a = register("<b>hi</b>").unwrap();
        assert_eq!(a, register("<b>hi</b>").unwrap());
        assert_ne!(a, register("<b>bye</b>").unwrap());
        let doc = document("<b>hi</b>");
        assert!(doc.starts_with("<!doctype html>"));
        assert!(doc.contains("sc-html-height"));
        assert!(doc.find("sc-html-height").unwrap() < doc.find("</body>").unwrap());
    }

    #[test]
    fn rejects_huge_html() {
        assert!(register(&"x".repeat(MAX_BYTES + 1)).is_err());
    }
}
