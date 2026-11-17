//! 구글 드라이브 로그인 (CLAUDE.md 2-D-1, ADR-011). 데스크톱만.
//!
//! - 설치형 앱 방식: 기본 브라우저에서 구글 로그인 → `http://127.0.0.1:<빈 포트>`로 돌아온 code를 PKCE(S256)로 토큰과 바꾼다.
//!   state로 다른 요청을 막고, 5분 안에 돌아오지 않으면 그만둔다.
//! - 권한은 **앱 전용 폴더(`drive.appdata`)만** (Least Privilege). 사용자의 다른 드라이브 파일은 보지도 못한다.
//! - refresh token은 OS keyring(`secret_store`)에만. access token은 메모리에만(1시간). DB·설정·로그·묶음에 넣지 않는다.
//! - 클라이언트 ID는 빌드할 때 넣는다(build.rs, 저장소에 없음). 없는 빌드면 드라이브 연결을 쓸 수 없다고 알린다.
use std::sync::Mutex;
use std::time::Duration;

use base64::Engine;
use reqwest::Url;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{AppError, ErrorCode, Result};
use crate::service::secret_store::SecretStore;

pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata";
const REFRESH_KEY: &str = "google-drive:refresh-token";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

/// 구글 주소 (테스트는 가짜 서버로 바꾼다)
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub auth: String,
    pub token: String,
    pub revoke: String,
    pub api: String,
    pub upload: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            auth: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token: "https://oauth2.googleapis.com/token".into(),
            revoke: "https://oauth2.googleapis.com/revoke".into(),
            api: "https://www.googleapis.com/drive/v3".into(),
            upload: "https://www.googleapis.com/upload/drive/v3".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    pub id: String,
    pub secret: String,
}

/// 이 빌드에 들어 있는 OAuth 클라이언트 (build.rs)
pub fn built_in_client() -> Option<Client> {
    Some(Client {
        id: option_env!("SANDCASTLE_GOOGLE_CLIENT_ID").filter(|s| !s.is_empty())?.to_string(),
        // 설치형 앱의 client secret은 구글 문서상 비밀이 아니지만(앱에 들어감) 저장소에는 넣지 않는다
        secret: option_env!("SANDCASTLE_GOOGLE_CLIENT_SECRET").unwrap_or("").to_string(),
    })
}

/// Android: Play 서비스로 권한을 받은 적이 있음 (refresh token 대신 표시만 — 토큰은 필요할 때마다 폰이 준다)
const ANDROID_KEY: &str = "google-drive:android-granted";

/// 이 빌드에서 드라이브를 쓸 수 있나: 데스크톱은 OAuth 클라이언트가 들어 있을 때, Android는 Play 서비스로 늘
pub fn available() -> bool {
    cfg!(mobile) || built_in_client().is_some()
}

pub fn is_connected() -> bool {
    SecretStore::exists(REFRESH_KEY) || SecretStore::exists(ANDROID_KEY)
}

/// Android: 폰이 Play 서비스에서 받은 access token을 넣는다 (메모리에만). 처음이면 연결됨으로 표시
pub fn use_android_token(token: &str) -> Result<()> {
    if token.trim().is_empty() {
        return Err(AppError::validation("빈 토큰", None));
    }
    if let Ok(mut a) = ACCESS.lock() {
        // Play 서비스 토큰은 보통 1시간. 남은 시간을 모르므로 50분으로 본다
        *a = Some((token.to_string(), now() + 50 * 60));
    }
    SecretStore::set(ANDROID_KEY, "1").map_err(|e| AppError::io(format!("드라이브 연결을 저장하지 못함 (키 저장소): {e}")))?;
    Ok(())
}

/// 화면이 이 오류를 받으면 폰에서 토큰을 새로 받아 다시 부른다 (Android)
fn auth_required() -> AppError {
    AppError::validation("구글 드라이브 권한을 다시 받아야 함", Some(serde_json::json!({ "driveAuthRequired": true })))
}

fn drive_err(msg: impl Into<String>) -> AppError {
    AppError::ai_provider(ErrorCode::AiProviderConnectionFailed, msg, true)
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    refresh_token: Option<String>,
}

/// access token 캐시 (메모리에만)
static ACCESS: Mutex<Option<(String, i64)>> = Mutex::new(None);

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn cache(token: &TokenResponse) {
    if let Ok(mut a) = ACCESS.lock() {
        *a = Some((token.access_token.clone(), now() + token.expires_in.unwrap_or(3600) - 60));
    }
}

pub fn forget_access_token() {
    if let Ok(mut a) = ACCESS.lock() {
        *a = None;
    }
}

/// PKCE: verifier(64자)와 challenge(base64url(sha256))
fn pkce() -> (String, String) {
    let verifier = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

/// 로그인. `open` 으로 브라우저를 열고 돌아올 때까지 기다린다. refresh token을 keyring에 넣는다
pub async fn login(http: &reqwest::Client, client: &Client, ep: &Endpoints, open: impl FnOnce(&str) -> Result<()>) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let redirect = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let (verifier, challenge) = pkce();
    let state = uuid::Uuid::new_v4().simple().to_string();
    let url = Url::parse_with_params(
        &ep.auth,
        &[
            ("client_id", client.id.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("response_type", "code"),
            ("scope", SCOPE),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("state", state.as_str()),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ],
    )
    .map_err(|e| AppError::internal(e.to_string()))?;
    open(url.as_str())?;

    let code = tokio::time::timeout(LOGIN_TIMEOUT, wait_for_code(&listener, &state))
        .await
        .map_err(|_| AppError::validation("구글 로그인을 5분 안에 마치지 않아 그만둠", None))??;

    let res = http
        .post(&ep.token)
        .form(&[
            ("code", code.as_str()),
            ("client_id", client.id.as_str()),
            ("client_secret", client.secret.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| drive_err(format!("구글에 연결하지 못함: {e}")))?;
    if !res.status().is_success() {
        return Err(drive_err(format!("구글 로그인 토큰을 받지 못함 (HTTP {})", res.status())));
    }
    let token: TokenResponse = res.json().await.map_err(|e| drive_err(format!("구글 응답을 읽지 못함: {e}")))?;
    let refresh = token
        .refresh_token
        .clone()
        .ok_or_else(|| drive_err("구글이 refresh token을 주지 않음 — 연결 끊기 후 다시 로그인"))?;
    SecretStore::set(REFRESH_KEY, &refresh).map_err(|e| AppError::io(format!("구글 로그인을 저장하지 못함 (키 저장소): {e}")))?;
    cache(&token);
    Ok(())
}

/// 브라우저가 돌아올 때까지: `GET /?code=…&state=…` 를 받아 확인하고 "닫아도 됨" 페이지를 돌려준다
async fn wait_for_code(listener: &tokio::net::TcpListener, state: &str) -> Result<String> {
    loop {
        let (mut sock, _) = listener.accept().await?;
        let mut buf = vec![0u8; 8192];
        let n = sock.read(&mut buf).await?;
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
        let url = Url::parse(&format!("http://127.0.0.1{path}")).map_err(|e| AppError::internal(e.to_string()))?;
        let q = |k: &str| url.query_pairs().find(|(n, _)| n == k).map(|(_, v)| v.into_owned());
        // 파비콘 같은 다른 요청은 무시
        if q("code").is_none() && q("error").is_none() {
            let _ = sock
                .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await;
            continue;
        }
        let ok = q("state").as_deref() == Some(state) && q("code").is_some();
        let body = if ok {
            "<!doctype html><meta charset=utf-8><title>Sandcastle</title><p style='font:16px sans-serif;margin:3em'>구글 드라이브를 연결했습니다. 이 창을 닫고 앱으로 돌아가세요.</p>"
        } else {
            "<!doctype html><meta charset=utf-8><title>Sandcastle</title><p style='font:16px sans-serif;margin:3em'>연결하지 못했습니다. 앱에서 다시 시도하세요.</p>"
        };
        let _ = sock
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await;
        if let Some(e) = q("error") {
            return Err(AppError::validation(format!("구글 로그인이 취소되었거나 거절됨 ({e})"), None));
        }
        if !ok {
            return Err(AppError::validation("구글 로그인 응답이 이 요청과 맞지 않음 (state)", None));
        }
        return Ok(q("code").unwrap_or_default());
    }
}

/// 쓸 수 있는 access token (만료되면 refresh token으로 새로)
pub async fn access_token(http: &reqwest::Client, client: &Client, ep: &Endpoints) -> Result<String> {
    if let Some((t, exp)) = ACCESS.lock().ok().and_then(|a| a.clone()) {
        if exp > now() {
            return Ok(t);
        }
    }
    let refresh = match SecretStore::get(REFRESH_KEY) {
        Ok(r) => r,
        // Android는 refresh token이 없다 — 화면이 폰에서 새 토큰을 받아 넣는다
        Err(_) if SecretStore::exists(ANDROID_KEY) => return Err(auth_required()),
        Err(_) => return Err(AppError::validation("구글 드라이브가 연결되어 있지 않음 (설정 > 동기화에서 로그인)", None)),
    };
    let res = http
        .post(&ep.token)
        .form(&[
            ("client_id", client.id.as_str()),
            ("client_secret", client.secret.as_str()),
            ("refresh_token", refresh.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| drive_err(format!("구글에 연결하지 못함: {e}")))?;
    if res.status() == reqwest::StatusCode::BAD_REQUEST || res.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(AppError::validation(
            "구글 드라이브 연결이 끊어졌음 (권한이 취소됐거나 만료) — 설정 > 동기화에서 다시 로그인",
            None,
        ));
    }
    if !res.status().is_success() {
        return Err(drive_err(format!("구글 토큰을 새로 받지 못함 (HTTP {})", res.status())));
    }
    let token: TokenResponse = res.json().await.map_err(|e| drive_err(format!("구글 응답을 읽지 못함: {e}")))?;
    cache(&token);
    Ok(token.access_token)
}

/// 연결 끊기: 구글에서 권한을 거두고(가능하면) keyring에서 지운다
pub async fn disconnect(http: &reqwest::Client, ep: &Endpoints) -> Result<()> {
    // 데스크톱은 refresh token, Android는 지금 가진 access token으로 권한을 거둔다 (둘 다 그 허락 전체가 취소됨)
    let token = SecretStore::get(REFRESH_KEY)
        .ok()
        .or_else(|| ACCESS.lock().ok().and_then(|a| a.as_ref().map(|t| t.0.clone())));
    if let Some(t) = token {
        let _ = http
            .post(&ep.revoke)
            .form(&[("token", t.as_str())])
            .timeout(Duration::from_secs(10))
            .send()
            .await;
    }
    forget_access_token();
    let _ = SecretStore::delete(REFRESH_KEY);
    let _ = SecretStore::delete(ANDROID_KEY);
    Ok(())
}

#[cfg(test)]
pub(crate) fn set_refresh_for_test(v: &str) {
    SecretStore::set(REFRESH_KEY, v).unwrap();
    forget_access_token();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_verifier() {
        let (v, c) = pkce();
        assert_eq!(v.len(), 64);
        assert!(v.chars().all(|ch| ch.is_ascii_alphanumeric()));
        assert_eq!(c, base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(v.as_bytes())));
        assert!(!c.contains('=') && !c.contains('+') && !c.contains('/'));
    }

    #[test]
    fn android_token_is_used_until_it_expires_then_asks_again() {
        set_refresh_for_test("x");
        let _ = SecretStore::delete(REFRESH_KEY);
        assert!(use_android_token("").is_err());
        use_android_token("phone-token").unwrap();
        assert!(is_connected());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = Client {
            id: String::new(),
            secret: String::new(),
        };
        let ep = Endpoints::default(); // 요청이 나가면 안 됨: 캐시에서 바로
        assert_eq!(rt.block_on(access_token(&reqwest::Client::new(), &client, &ep)).unwrap(), "phone-token");
        forget_access_token();
        let e = rt.block_on(access_token(&reqwest::Client::new(), &client, &ep)).unwrap_err();
        assert!(
            matches!(e, AppError::Validation { details: Some(ref d), .. } if d["driveAuthRequired"] == true),
            "{e:?}"
        );
        let _ = SecretStore::delete(ANDROID_KEY);
    }

    #[tokio::test]
    async fn loopback_checks_state_and_returns_code() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let browser = tokio::spawn(async move {
            let c = reqwest::Client::new();
            // 다른 요청(파비콘)은 무시되고, 맞는 state로 돌아온 것만 받는다
            let _ = c.get(format!("http://127.0.0.1:{port}/favicon.ico")).send().await;
            c.get(format!("http://127.0.0.1:{port}/?state=s1&code=abc"))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        });
        assert_eq!(wait_for_code(&listener, "s1").await.unwrap(), "abc");
        assert!(browser.await.unwrap().contains("연결했습니다"));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { reqwest::get(format!("http://127.0.0.1:{port}/?state=other&code=abc")).await });
        assert!(wait_for_code(&listener, "s1").await.is_err(), "state가 다르면 거절");
    }
}
