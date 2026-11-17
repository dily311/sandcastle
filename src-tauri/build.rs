fn main() {
    google_oauth_client();
    tauri_build::build()
}

/// 구글 드라이브 OAuth 클라이언트 (CLAUDE.md 2-D, ADR-011). 저장소에 넣지 않는다:
/// `src-tauri/google-oauth.env`(.gitignore의 *.env) 또는 빌드 환경 변수(CI는 GitHub Secrets)에서 읽어
/// `option_env!("SANDCASTLE_GOOGLE_…")`로 앱에 넣는다. 없으면 드라이브 연결이 없는 빌드가 된다.
/// 이름은 `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET`(GitHub Secrets와 같은 이름)과 `SANDCASTLE_` 붙은 것 둘 다 받는다.
/// (`GOOGLE_ANDROID_CLIENT_ID`는 Android 드라이브를 만들 때 쓴다 — 지금은 읽지 않음)
fn google_oauth_client() {
    // (앱이 읽는 이름, 받아 주는 이름들)
    const KEYS: [(&str, [&str; 2]); 2] = [
        ("SANDCASTLE_GOOGLE_CLIENT_ID", ["SANDCASTLE_GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_ID"]),
        ("SANDCASTLE_GOOGLE_CLIENT_SECRET", ["SANDCASTLE_GOOGLE_CLIENT_SECRET", "GOOGLE_CLIENT_SECRET"]),
    ];
    println!("cargo:rerun-if-changed=google-oauth.env");
    for (_, names) in KEYS {
        for n in names {
            println!("cargo:rerun-if-env-changed={n}");
        }
    }
    let file: Vec<(String, String)> = std::fs::read_to_string("google-oauth.env")
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().trim_start_matches('\u{feff}'))
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().trim_matches('"').to_string()))
        .filter(|(_, v)| !v.is_empty())
        .collect();
    for (target, names) in KEYS {
        // 환경 변수가 먼저, 그다음 파일
        let value = names
            .iter()
            .find_map(|n| std::env::var(n).ok().filter(|v| !v.is_empty()))
            .or_else(|| names.iter().find_map(|n| file.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone())));
        if let Some(v) = value {
            println!("cargo:rustc-env={target}={v}");
        }
    }
}
