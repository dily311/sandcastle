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
//! 테마: 화면이 지금 테마의 색을 CSS 변수(:root{--bg;--text;--accent…})로 넘기면 문서 맨 앞에 넣는다.
//!   AI가 쓴 스타일이 뒤에 오므로 원하면 덮어쓸 수 있고, 안 쓰면 앱과 같은 색으로 보인다.
//!
//! 내용은 메모리에만 둔다 (같은 HTML·테마는 같은 id, 최근 MAX_ENTRIES개만).
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};

use tauri::http::{Request, Response, StatusCode};

use crate::error::{AppError, Result};

const MAX_ENTRIES: usize = 300;
const MAX_BYTES: usize = 1024 * 1024;
const MAX_THEME_BYTES: usize = 8 * 1024;

const CSP: &str = "default-src 'none'; script-src 'unsafe-inline' https:; style-src 'unsafe-inline' https:; \
img-src data: blob: https:; font-src data: https:; media-src data: blob: https:; connect-src 'none'; \
form-action 'none'; frame-src 'none'; base-uri 'none'";

const HEIGHT_SCRIPT: &str = r#"<script>(()=>{const s=()=>parent.postMessage({type:"sc-html-height",h:Math.ceil(document.body?document.body.getBoundingClientRect().bottom+parseFloat(getComputedStyle(document.body).marginBottom||"0"):document.documentElement.scrollHeight)},"*");try{new ResizeObserver(s).observe(document.documentElement)}catch(e){}addEventListener("load",s);s();setTimeout(s,300);setTimeout(s,1000);})();</script>"#;

struct Store {
    /// id → (HTML, 테마 CSS)
    map: HashMap<String, (String, String)>,
    order: VecDeque<String>,
}

static STORE: LazyLock<Mutex<Store>> = LazyLock::new(|| Mutex::new(Store { map: HashMap::new(), order: VecDeque::new() }));

fn id_of(html: &str, theme: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (html, theme).hash(&mut h);
    format!("{:016x}{:x}", h.finish(), html.len())
}

/// 테마 CSS는 스타일 블록 하나에만 들어가게: `<`를 빼서 </style>로 빠져나가지 못하게 하고 길이를 자른다
fn clean_theme(theme: &str) -> String {
    theme.replace('<', "").chars().take(MAX_THEME_BYTES).collect()
}

/// 이 HTML을 보여줄 주소. 같은 HTML·테마면 같은 주소다.
pub fn register(html: &str, theme: Option<&str>) -> Result<String> {
    if html.len() > MAX_BYTES {
        return Err(AppError::validation("HTML이 너무 큽니다 (1MB까지)", None));
    }
    let theme = clean_theme(theme.unwrap_or(""));
    let id = id_of(html, &theme);
    let mut s = STORE.lock().unwrap_or_else(|e| e.into_inner());
    if !s.map.contains_key(&id) {
        s.map.insert(id.clone(), (html.to_string(), theme));
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

/// 조각이면 문서로 감싸고, 테마 CSS를 맨 앞에 넣고, 높이를 알려주는 스크립트를 붙인다.
pub fn document(html: &str, theme: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let theme_style = if theme.trim().is_empty() { String::new() } else { format!("<style id=\"sc-theme\">{theme}</style>") };
    let mut doc = if lower.contains("<html") || lower.contains("<!doctype") {
        // 완성된 문서: <head> 바로 뒤(없으면 맨 앞)에 테마를 넣어 문서의 스타일이 이길 수 있게
        let mut d = html.to_string();
        match d.to_ascii_lowercase().find("<head>") {
            Some(i) => d.insert_str(i + "<head>".len(), &theme_style),
            None => d.insert_str(0, &theme_style),
        }
        d
    } else {
        format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
<style>html,body{{margin:0;padding:8px;font-family:system-ui,sans-serif;color-scheme:light dark;}}</style>{theme_style}</head><body>{html}</body></html>"
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
        Some((h, theme)) => (StatusCode::OK, document(&h, &theme)),
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
/// theme: 지금 테마의 CSS 변수 (화면이 만든 `:root{…}` 등). 문서 맨 앞에 들어간다.
#[tauri::command]
#[specta::specta]
pub fn register_html_view(app: tauri::AppHandle, html: String, theme: Option<String>) -> Result<String> {
    crate::sys::plugins::require(&app, crate::sys::plugins::id::HTML)?;
    register(&html, theme.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_html_same_url_and_fragment_wrapped() {
        let a = register("<b>hi</b>", None).unwrap();
        assert_eq!(a, register("<b>hi</b>", None).unwrap());
        assert_ne!(a, register("<b>bye</b>", None).unwrap());
        assert_ne!(a, register("<b>hi</b>", Some(":root{--bg:#000}")).unwrap());
        let doc = document("<b>hi</b>", "");
        assert!(doc.starts_with("<!doctype html>"));
        assert!(doc.contains("sc-html-height"));
        assert!(doc.find("sc-html-height").unwrap() < doc.find("</body>").unwrap());
    }

    #[test]
    fn rejects_huge_html() {
        assert!(register(&"x".repeat(MAX_BYTES + 1), None).is_err());
    }

    #[test]
    fn theme_goes_first_and_cannot_escape_style() {
        let doc = document("<style>body{color:red}</style><b>x</b>", &clean_theme(":root{--bg:#111}</style><script>alert(1)</script>"));
        let theme_at = doc.find("sc-theme").unwrap();
        assert!(theme_at < doc.find("color:red").unwrap());
        assert!(!doc.contains("<script>alert"));
        let full = document("<!doctype html><html><head><title>t</title></head><body>x</body></html>", ":root{--bg:#111}");
        assert!(full.find("sc-theme").unwrap() < full.find("<title>").unwrap());
    }
}
