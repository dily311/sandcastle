//! 새 버전 확인 (CLAUDE.md 1-E-2, ADR-010).
//!
//! GitHub Releases의 최신 릴리스만 본다. 내려받거나 설치하지 않는다 — 새 버전이 있으면 릴리스 페이지 주소를 돌려주고
//! 사용자가 열어서 직접 받는다. 사용자가 "업데이트 확인"을 누르거나, "앱을 켤 때 새 버전 확인"(기본 꺼짐)을 켰을 때만 부른다.
//! 요청에는 일반적인 User-Agent만 붙인다 (사용자 정보 없음).
use serde::{Deserialize, Serialize};

use crate::error::{AppError, ErrorCode, Result};

pub const REPO: &str = "dily311/sandcastle";
const USER_AGENT: &str = "sandcastle-update-check";
const MAX_NOTES: usize = 4000;

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    /// 최신 릴리스 버전 (릴리스가 아직 없으면 None)
    pub latest: Option<String>,
    pub newer: bool,
    /// 릴리스 페이지 (여기서 내려받는다)
    pub url: Option<String>,
    pub notes: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// "v1.2.3", "1.2.3-beta" → (1, 2, 3, 시험판인가)
pub fn parse_version(s: &str) -> Option<(u64, u64, u64, bool)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let (core, pre) = match s.split_once('-') {
        Some((c, _)) => (c, true),
        None => (s, false),
    };
    let mut it = core.split('.').map(|p| p.parse::<u64>().ok());
    let v = (it.next()??, it.next().flatten().unwrap_or(0), it.next().flatten().unwrap_or(0));
    Some((v.0, v.1, v.2, pre))
}

/// latest가 current보다 새 버전인가 (같은 번호면 정식판이 시험판보다 새것)
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => (l.0, l.1, l.2) > (c.0, c.1, c.2) || ((l.0, l.1, l.2) == (c.0, c.1, c.2) && c.3 && !l.3),
        _ => false,
    }
}

fn info_from(release: Option<Release>, current: &str) -> UpdateInfo {
    let Some(r) = release.filter(|r| !r.draft && !r.prerelease) else {
        return UpdateInfo {
            current: current.into(),
            latest: None,
            newer: false,
            url: None,
            notes: None,
            published_at: None,
        };
    };
    let latest = r.tag_name.trim_start_matches(['v', 'V']).to_string();
    UpdateInfo {
        current: current.into(),
        newer: is_newer(&latest, current),
        latest: Some(latest),
        url: Some(r.html_url),
        notes: r.body.map(|b| b.chars().take(MAX_NOTES).collect()),
        published_at: r.published_at,
    }
}

pub async fn check(http: &reqwest::Client) -> Result<UpdateInfo> {
    let current = env!("CARGO_PKG_VERSION");
    let res = http
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| AppError::AiProvider {
            code: ErrorCode::AiProviderConnectionFailed,
            message: format!("새 버전을 확인하지 못함: {e}"),
            retryable: true,
            details: None,
        })?;
    // 릴리스가 하나도 없으면 404
    if res.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(info_from(None, current));
    }
    if !res.status().is_success() {
        return Err(AppError::io(format!("새 버전을 확인하지 못함 (HTTP {})", res.status())));
    }
    let release: Release = res.json().await.map_err(|e| AppError::io(format!("릴리스 정보를 읽지 못함: {e}")))?;
    Ok(info_from(Some(release), current))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3, false)));
        assert_eq!(parse_version("0.2"), Some((0, 2, 0, false)));
        assert_eq!(parse_version("1.0.0-beta.1"), Some((1, 0, 0, true)));
        assert_eq!(parse_version("abc"), None);
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(is_newer("1.0.0", "1.0.0-beta"));
        assert!(!is_newer("garbage", "0.1.0"));
    }

    #[test]
    fn builds_info_and_skips_drafts() {
        let r = |tag: &str, draft: bool| Release {
            tag_name: tag.into(),
            html_url: "https://github.com/dily311/sandcastle/releases/tag/x".into(),
            body: Some("노트".into()),
            published_at: Some("2026-11-20T00:00:00Z".into()),
            draft,
            prerelease: false,
        };
        let i = info_from(Some(r("v0.2.0", false)), "0.1.0");
        assert!(i.newer);
        assert_eq!(i.latest.as_deref(), Some("0.2.0"));
        assert!(i.url.is_some());
        assert!(!info_from(Some(r("v0.2.0", true)), "0.1.0").newer, "초안은 무시");
        let none = info_from(None, "0.1.0");
        assert!(!none.newer && none.latest.is_none());
    }
}
