//! game://localhost/<game_id>/<path> 로 게임 파일을 서빙한다.
//! (Windows/Android에서는 http://game.localhost/<game_id>/<path>)
//! 임포트 시 zip을 <app_data>/games/<game_id>/ 로 풀어둔다고 가정한다.
//!
//! 에셋(이미지/오디오/폰트) 서빙을 위해:
//!   - 확장자 허용 목록: 목록 밖의 파일은 서빙하지 않는다 (SVG는 일부러 뺐다).
//!   - Range 요청(206): <audio>의 탐색과 긴 BGM 재생(특히 Android WebView)에 필요하다. 필요한 구간만 파일에서 읽어
//!     통째로 메모리에 올리지 않는다. 한 응답의 길이는 CHUNK_BYTES로 자른다(브라우저가 이어서 요청한다).
//!   - 종류별 파일 크기 상한.
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use tauri::http::{Request, Response, StatusCode};

const CSP: &str = "default-src 'none'; script-src 'self' 'unsafe-inline'; \
style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' data: blob:; \
font-src 'self' data:; connect-src 'none'; form-action 'none'; frame-src 'none'; \
base-uri 'none'";

/// Range 응답 하나의 최대 길이. 열린 구간(bytes=0-)이 와도 이만큼만 돌려준다.
const CHUNK_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    Image,
    Audio,
    Font,
}

/// (확장자, 종류). 목록에 없으면 403.
const ALLOWED: &[(&str, Kind)] = &[
    ("html", Kind::Text),
    ("htm", Kind::Text),
    ("js", Kind::Text),
    ("mjs", Kind::Text),
    ("css", Kind::Text),
    ("json", Kind::Text),
    ("txt", Kind::Text),
    ("png", Kind::Image),
    ("jpg", Kind::Image),
    ("jpeg", Kind::Image),
    ("webp", Kind::Image),
    ("gif", Kind::Image),
    ("avif", Kind::Image),
    ("mp3", Kind::Audio),
    ("ogg", Kind::Audio),
    ("wav", Kind::Audio),
    ("m4a", Kind::Audio),
    ("aac", Kind::Audio),
    ("opus", Kind::Audio),
    ("flac", Kind::Audio),
    ("woff2", Kind::Font),
    ("woff", Kind::Font),
];

/// 이 확장자의 파일을 game:// 로 서빙하는가 (validate가 쓴다).
pub fn is_servable(ext: &str) -> bool {
    let e = ext.to_ascii_lowercase();
    ALLOWED.iter().any(|(a, _)| *a == e)
}

fn max_bytes(kind: Kind) -> u64 {
    match kind {
        Kind::Text => 5 * 1024 * 1024,
        Kind::Image => 30 * 1024 * 1024,
        Kind::Audio => 150 * 1024 * 1024,
        Kind::Font => 10 * 1024 * 1024,
    }
}

struct Served {
    status: StatusCode,
    body: Vec<u8>,
    mime: String,
    content_range: Option<String>,
}

/// 이 게임의 파일을 불러오는 기준 URL (끝에 `/`). 프론트는 에셋의 상대경로 path를 여기에 이어 붙인다.
/// Windows/Android의 WebView는 사용자 지정 프로토콜을 http://<scheme>.localhost 로, 나머지는 <scheme>://localhost 로 부른다.
/// (tauri.conf의 useHttpsScheme를 켰다면 Windows는 https가 된다.)
pub fn base_url(game_id: &str) -> String {
    if cfg!(any(target_os = "windows", target_os = "android")) {
        format!("http://game.localhost/{game_id}/")
    } else {
        format!("game://localhost/{game_id}/")
    }
}

pub fn handle(games_root: &Path, req: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let range = req.headers().get("range").and_then(|v| v.to_str().ok()).map(str::to_string);
    match serve(games_root, req.uri().path(), range.as_deref()) {
        Ok(s) => {
            let mut b = Response::builder()
                .status(s.status)
                .header("Content-Type", s.mime)
                .header("Accept-Ranges", "bytes")
                .header("Content-Security-Policy", CSP)
                .header("X-Content-Type-Options", "nosniff")
                // 에셋은 호스트 창(다른 origin)과 샌드박스 iframe(불투명 origin)이 둘 다 불러온다. CORP가 same-origin이면 둘 다
                // 막히고, pixi.js의 텍스처 로더/WebAudio는 fetch(CORS)를 써서 ACAO도 필요하다. 이 프로토콜은 앱 안에서만 열린다.
                .header("Cross-Origin-Resource-Policy", "cross-origin")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Expose-Headers", "Content-Range, Accept-Ranges, Content-Length");
            if let Some(cr) = s.content_range {
                b = b.header("Content-Range", cr);
            }
            b.body(s.body).unwrap()
        }
        Err(StatusCode::RANGE_NOT_SATISFIABLE) => Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header("Content-Range", "bytes */0")
            .body(Vec::new())
            .unwrap(),
        Err(code) => Response::builder().status(code).body(Vec::new()).unwrap(),
    }
}

/// "bytes=a-b" / "bytes=a-" / "bytes=-n" 한 구간만 지원한다. Err = 만족할 수 없는 구간(416).
fn parse_range(header: &str, total: u64) -> Result<(u64, u64), ()> {
    let spec = header.trim().strip_prefix("bytes=").ok_or(())?;
    if spec.contains(',') || total == 0 {
        return Err(()); // 여러 구간은 지원하지 않는다
    }
    let (a, b) = spec.split_once('-').ok_or(())?;
    let (start, end) = if a.is_empty() {
        let n: u64 = b.parse().map_err(|_| ())?; // 마지막 n바이트
        if n == 0 {
            return Err(());
        }
        (total.saturating_sub(n), total - 1)
    } else {
        let s: u64 = a.parse().map_err(|_| ())?;
        let e: u64 = if b.is_empty() { total - 1 } else { b.parse().map_err(|_| ())? };
        (s, e.min(total - 1))
    };
    if start > end || start >= total {
        return Err(());
    }
    Ok((start, end))
}

fn serve(root: &Path, uri_path: &str, range: Option<&str>) -> Result<Served, StatusCode> {
    let rel = uri_path.trim_start_matches('/');
    let mut parts = rel.splitn(2, '/');
    let game_id = parts.next().ok_or(StatusCode::NOT_FOUND)?;
    let file = parts.next().filter(|s| !s.is_empty()).unwrap_or("index.html");

    if game_id.is_empty() || !game_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(StatusCode::BAD_REQUEST);
    }
    // 정의/매니페스트 사본(옛 임포트가 풀어둔 것)은 서빙하지 않는다 — 정의의 원본은 DB다.
    if file == "manifest.json" || file == "data" || file.starts_with("data/") {
        return Err(StatusCode::FORBIDDEN);
    }
    let base: PathBuf = root.join(game_id).canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    let full = base.join(file).canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    // 경로 탈출(../, 심볼릭 링크) 방지: 정규화 후 base 하위인지 확인
    if !full.starts_with(&base) {
        return Err(StatusCode::FORBIDDEN);
    }

    let ext = full.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).unwrap_or_default();
    let kind = ALLOWED.iter().find(|(e, _)| *e == ext).map(|(_, k)| *k).ok_or(StatusCode::FORBIDDEN)?;
    let meta = std::fs::metadata(&full).map_err(|_| StatusCode::NOT_FOUND)?;
    if !meta.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }
    let total = meta.len();
    if total > max_bytes(kind) {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let mime = mime_guess::from_path(&full).first_or_octet_stream().to_string();

    let mut f = std::fs::File::open(&full).map_err(|_| StatusCode::NOT_FOUND)?;
    match range {
        Some(h) => {
            let (start, mut end) = parse_range(h, total).map_err(|_| StatusCode::RANGE_NOT_SATISFIABLE)?;
            end = end.min(start + CHUNK_BYTES - 1);
            let len = end - start + 1;
            f.seek(SeekFrom::Start(start)).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let mut body = Vec::with_capacity(len as usize);
            f.take(len).read_to_end(&mut body).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Served {
                status: StatusCode::PARTIAL_CONTENT,
                body,
                mime,
                content_range: Some(format!("bytes {start}-{end}/{total}")),
            })
        }
        None => {
            let mut body = Vec::with_capacity(total as usize);
            f.read_to_end(&mut body).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Served {
                status: StatusCode::OK,
                body,
                mime,
                content_range: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_forms() {
        assert_eq!(parse_range("bytes=0-99", 1000), Ok((0, 99)));
        assert_eq!(parse_range("bytes=500-", 1000), Ok((500, 999)));
        assert_eq!(parse_range("bytes=-100", 1000), Ok((900, 999)));
        assert_eq!(parse_range("bytes=900-5000", 1000), Ok((900, 999))); // 끝이 넘치면 파일 끝으로
        assert!(parse_range("bytes=1000-1001", 1000).is_err()); // 시작이 파일 밖
        assert!(parse_range("bytes=0-1,5-9", 1000).is_err()); // 여러 구간 미지원
        assert!(parse_range("items=0-1", 1000).is_err());
    }
}
