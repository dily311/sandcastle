// game-runtime.html으로 게임 파일을 서비스
//! game://localhost/<game_id>/<path> 로 게임 파일을 서빙한다.
//! (Windows/Android에서는 http://game.localhost/<game_id>/<path>)
//! 임포트 시 zip을 <app_data>/games/<game_id>/ 로 풀어둔다고 가정한다.
use std::path::{Path, PathBuf};
use tauri::http::{Request, Response, StatusCode};

const CSP: &str = "default-src 'none'; script-src 'self' 'unsafe-inline'; \
style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' data: blob:; \
font-src 'self' data:; connect-src 'none'; form-action 'none'; frame-src 'none'; \
base-uri 'none'";

pub fn handle(games_root: &Path, req: Request<Vec<u8>>) -> Response<Vec<u8>> {
    match serve(games_root, req.uri().path()) {
        Ok((bytes, mime)) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime)
            .header("Content-Security-Policy", CSP)
            .header("X-Content-Type-Options", "nosniff")
            .header("Cross-Origin-Resource-Policy", "same-origin")
            .body(bytes)
            .unwrap(),
        Err(code) => Response::builder().status(code).body(Vec::new()).unwrap(),
    }
}

fn serve(root: &Path, uri_path: &str) -> Result<(Vec<u8>, String), StatusCode> {
    let rel = uri_path.trim_start_matches('/');
    let mut parts = rel.splitn(2, '/');
    let game_id = parts.next().ok_or(StatusCode::NOT_FOUND)?;
    let file = parts.next().filter(|s| !s.is_empty()).unwrap_or("game-runtime.html");

    if game_id.is_empty()
        || !game_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let base: PathBuf = root.join(game_id).canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    let full = base.join(file).canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    // 경로 탈출(../, 심볼릭 링크) 방지: 정규화 후 base 하위인지 확인
    if !full.starts_with(&base) {
        return Err(StatusCode::FORBIDDEN);
    }
    let bytes = std::fs::read(&full).map_err(|_| StatusCode::NOT_FOUND)?;
    let mime = mime_guess::from_path(&full).first_or_octet_stream().to_string();
    Ok((bytes, mime))
}
