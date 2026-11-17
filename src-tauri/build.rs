fn main() {
    google_oauth_client();
    tauri_build::build()
}

/// 구글 드라이브 OAuth 클라이언트 (CLAUDE.md 2-D, ADR-011). 저장소에 넣지 않는다:
/// `src-tauri/google-oauth.env`(.gitignore의 *.env) 또는 빌드 환경 변수(CI는 GitHub Secrets)에서 읽어
/// `option_env!`로 앱에 넣는다. 없으면 드라이브 연결이 없는 빌드가 된다(폴더 동기화·파일 주고받기는 그대로).
fn google_oauth_client() {
    const KEYS: [&str; 2] = ["SANDCASTLE_GOOGLE_CLIENT_ID", "SANDCASTLE_GOOGLE_CLIENT_SECRET"];
    println!("cargo:rerun-if-changed=google-oauth.env");
    for k in KEYS {
        println!("cargo:rerun-if-env-changed={k}");
    }
    let Ok(text) = std::fs::read_to_string("google-oauth.env") else { return };
    for line in text.lines().map(str::trim).filter(|l| !l.starts_with('#')) {
        if let Some((k, v)) = line.split_once('=') {
            let (k, v) = (k.trim(), v.trim().trim_matches('"'));
            // 환경 변수가 이미 있으면 그것이 먼저
            if KEYS.contains(&k) && !v.is_empty() && std::env::var(k).is_err() {
                println!("cargo:rustc-env={k}={v}");
            }
        }
    }
}
