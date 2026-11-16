//! AI 공급자 어댑터.
//!
//! 어댑터는 "공급자별 HTTP 형식 변환"만 담당한다.
//! API 키 조회, 취소, 프론트 이벤트, 작업 상태는 모두 바깥(AiService / TaskManager)의 일이다.
//! 그래서 어댑터는 Tauri 타입(Channel, AppHandle)에 의존하지 않고 `on_delta` 콜백만 받는다.
use async_trait::async_trait;
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;

use crate::{
    error::{AppError, ErrorCode, Result},
    models::ai_service::{ApiProfiles, PromptMessage, PromptRole},
};

pub mod claude_adapter;
pub mod gemini_adapter;
pub mod openai_adapter;
pub mod vertexai_adapter;
// 그림모델
pub mod nai_adapter;

mod sse;

/// 어댑터 호출에 필요한 공통 재료
pub struct ProviderCall<'a> {
    pub http: &'a Client,
    pub profile: &'a ApiProfiles,
    /// keyring에서 꺼낸 비밀값 (API 키 또는 Vertex 서비스 계정 JSON)
    pub credential: &'a str,
}

impl ProviderCall<'_> {
    pub fn url(&self, path: &str) -> String {
        format!("{}/{}", self.profile.base_url.trim_end_matches('/'), path.trim_start_matches('/'))
    }
}

/// 스트리밍 조각 콜백.
/// (async_trait가 생략된 수명을 고정 수명으로 바꾸지 않도록 HRTB를 명시한다)
pub type OnDelta<'a> = dyn for<'s> FnMut(&'s str) + Send + 'a;

/// 생성 결과
#[derive(Debug, Clone, Default)]
pub struct Generation {
    pub content: String,
    pub finish_reason: Option<String>,
    /// 공급자가 알려준 토큰 사용량 (입력, 출력). 없으면 None → 앱이 추정 (ResourceManager)
    pub usage: Option<(u64, u64)>,
}

/// JSON 안의 두 숫자로 (입력, 출력) 토큰. 둘 다 없으면 None
pub(crate) fn usage_from(v: &serde_json::Value, input_key: &str, output_key: &str) -> Option<(u64, u64)> {
    let i = v.get(input_key).and_then(serde_json::Value::as_u64);
    let o = v.get(output_key).and_then(serde_json::Value::as_u64);
    (i.is_some() || o.is_some()).then(|| (i.unwrap_or(0), o.unwrap_or(0)))
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn list_models(&self, call: &ProviderCall<'_>) -> Result<Vec<String>>;

    /// 텍스트 생성.
    /// `prompt.chat_options.stream`이 true면 스트리밍으로 받으며 조각마다 `on_delta`를 부른다.
    /// false면 `on_delta`를 부르지 않고 전체 응답만 돌려준다.
    async fn generate(&self, call: &ProviderCall<'_>, prompt: &PromptMessage, on_delta: &mut OnDelta<'_>) -> Result<Generation>;
}

/// api_profiles.format_type → 어댑터
pub fn provider_for(format_type: &str) -> Result<&'static dyn AiProvider> {
    static OPENAI: openai_adapter::OpenAIClient = openai_adapter::OpenAIClient;
    static CLAUDE: claude_adapter::ClaudeClient = claude_adapter::ClaudeClient;
    static GEMINI: gemini_adapter::GeminiClient = gemini_adapter::GeminiClient;
    static VERTEX: vertexai_adapter::VertexAIClient = vertexai_adapter::VertexAIClient;

    match format_type {
        "openai" => Ok(&OPENAI),
        "claude" | "anthropic" => Ok(&CLAUDE),
        "gemini" => Ok(&GEMINI),
        "vertexai" | "vertex" => Ok(&VERTEX),
        other => Err(AppError::validation(format!("지원하지 않는 API 형식입니다: {other}"), None)),
    }
}

// =======================================
// 메시지 정리 (system 파라미터가 따로 있는 공급자용)
// =======================================

/// 대화 턴 (system 제외)
pub(crate) struct Turn {
    pub role: PromptRole,
    pub content: String,
    pub images: Vec<String>,
}

/// 붙인 이미지 한 장: (MIME, base64). 못 읽거나 너무 크면 건너뜀
pub(crate) fn load_image(path: &str) -> Option<(String, String)> {
    use base64::Engine;
    const MAX: u64 = 20 * 1024 * 1024;
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > MAX {
        eprintln!("[ai] 첨부 이미지가 너무 커서 건너뜀: {path}");
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let ext = std::path::Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return None,
    };
    Some((mime.to_string(), base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// Claude/Gemini처럼 system을 별도 필드로 받고 user/assistant 교대를 기대하는 공급자를 위해
/// - 맨 앞에 연속된 system 메시지 → system 문자열
/// - 대화 중간의 system 메시지(in_chat 블록 등) → user 턴
/// - 같은 역할이 연속되면 하나로 합침
/// - 첫 턴이 user가 아니면 빈 대화 시작용 user 턴을 앞에 붙임
pub(crate) fn split_system(prompt: &PromptMessage) -> (Option<String>, Vec<Turn>) {
    let mut system_parts: Vec<&str> = vec![];
    let mut turns: Vec<Turn> = vec![];
    let mut leading = true;

    for m in &prompt.messages {
        if m.content.trim().is_empty() && m.images.is_empty() {
            continue;
        }
        if leading && m.role == PromptRole::System && m.images.is_empty() {
            system_parts.push(&m.content);
            continue;
        }
        leading = false;
        let role = match m.role {
            PromptRole::System => PromptRole::User,
            r => r,
        };
        match turns.last_mut() {
            Some(last) if last.role == role => {
                if !m.content.trim().is_empty() {
                    last.content.push_str("\n\n");
                    last.content.push_str(&m.content);
                }
                last.images.extend(m.images.iter().cloned());
            }
            _ => turns.push(Turn {
                role,
                content: m.content.clone(),
                images: m.images.clone(),
            }),
        }
    }

    if turns.first().map(|t| t.role) != Some(PromptRole::User) {
        turns.insert(
            0,
            Turn {
                role: PromptRole::User,
                content: "[Start]".to_string(),
                images: vec![],
            },
        );
    }

    let system = if system_parts.is_empty() { None } else { Some(system_parts.join("\n\n")) };
    (system, turns)
}

// =======================================
// 공통 오류 처리
// =======================================

/// 네트워크 단계 오류 (연결 실패, 타임아웃 등)
pub(crate) fn net_err(e: reqwest::Error) -> AppError {
    if e.is_timeout() {
        AppError::ai_provider(ErrorCode::AiProviderTimeout, "AI 서버 응답 시간이 초과되었습니다", true)
    } else if e.is_connect() {
        AppError::ai_provider(ErrorCode::AiProviderConnectionFailed, format!("AI 서버에 연결할 수 없습니다: {e}"), true)
    } else {
        AppError::ai_provider(ErrorCode::AiProviderError, e.to_string(), false)
    }
}

/// 2xx가 아니면 공급자 오류로 바꾼다.
pub(crate) async fn ensure_success(resp: Response) -> Result<Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().await.unwrap_or_default();
    Err(http_error(status, &body))
}

pub(crate) fn http_error(status: StatusCode, body: &str) -> AppError {
    let detail = provider_message(body);
    let (code, retryable, label) = match status.as_u16() {
        401 | 403 => (ErrorCode::AiProviderAuthFailed, false, "API 인증에 실패했습니다"),
        404 => (ErrorCode::AiProviderModelNotFound, false, "요청한 모델을 찾을 수 없습니다"),
        408 => (ErrorCode::AiProviderTimeout, true, "AI 서버 응답 시간이 초과되었습니다"),
        429 => (ErrorCode::AiProviderRateLimit, true, "API 요청 한도를 초과했습니다"),
        500..=599 => (ErrorCode::AiProviderError, true, "AI 서버에서 오류가 발생했습니다"),
        _ => (ErrorCode::AiProviderError, false, "AI 요청이 거부되었습니다"),
    };
    let message = if detail.is_empty() {
        format!("{label} ({status})")
    } else {
        format!("{label} ({status}): {detail}")
    };
    AppError::AiProvider {
        code,
        message,
        retryable,
        details: Some(serde_json::json!({ "status": status.as_u16() })),
    }
}

/// 공급자 오류 본문에서 사람이 읽을 메시지만 뽑는다.
/// (OpenAI/Anthropic/Gemini 모두 `error.message` 형태)
fn provider_message(body: &str) -> String {
    let msg = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| {
            v.pointer("/error/message")
                .or_else(|| v.get("message"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| body.trim().to_string());
    truncate(&msg, 500)
}

pub(crate) fn parse_err(what: &str, e: impl std::fmt::Display) -> AppError {
    AppError::ai_provider(ErrorCode::AiProviderError, format!("{what} 응답을 해석할 수 없습니다: {e}"), false)
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max_chars).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_error_maps_status_and_message() {
        let e = http_error(StatusCode::TOO_MANY_REQUESTS, r#"{"error":{"message":"slow down"}}"#);
        let r = e.to_response();
        assert_eq!(r.code, ErrorCode::AiProviderRateLimit);
        assert!(r.retryable);
        assert!(r.message.contains("slow down"));

        let r = http_error(StatusCode::UNAUTHORIZED, "nope").to_response();
        assert_eq!(r.code, ErrorCode::AiProviderAuthFailed);
        assert!(!r.retryable);
    }

    #[test]
    fn split_system_merges_turns() {
        use crate::models::ai_service::{ChatOptions, PromptMessageItem};
        let item = |role, c: &str| PromptMessageItem {
            images: vec![],
            role,
            content: c.into(),
            token: 0,
        };
        let prompt = PromptMessage {
            chat_options: ChatOptions::default(),
            messages: vec![
                item(PromptRole::System, "rules"),
                item(PromptRole::System, "char"),
                item(PromptRole::Assistant, "hello"),
                item(PromptRole::User, "hi"),
                item(PromptRole::System, "note"),
                item(PromptRole::Assistant, "ok"),
            ],
        };
        let (system, turns) = split_system(&prompt);
        assert_eq!(system.as_deref(), Some("rules\n\nchar"));
        let shape: Vec<_> = turns.iter().map(|t| (t.role, t.content.as_str())).collect();
        assert_eq!(
            shape,
            vec![
                (PromptRole::User, "[Start]"),
                (PromptRole::Assistant, "hello"),
                (PromptRole::User, "hi\n\nnote"),
                (PromptRole::Assistant, "ok"),
            ]
        );
    }

    #[test]
    fn images_reach_every_request_format() {
        use crate::models::ai_service::{ChatOptions, PromptMessageItem};
        let dir = tempfile::tempdir().unwrap();
        let png = dir.path().join("x.png");
        std::fs::write(&png, [137u8, 80, 78, 71]).unwrap();
        let png = png.to_string_lossy().into_owned();
        let prompt = PromptMessage {
            chat_options: ChatOptions::default(),
            messages: vec![
                PromptMessageItem {
                    images: vec![],
                    role: PromptRole::System,
                    content: "rules".into(),
                    token: 0,
                },
                PromptMessageItem {
                    images: vec![png.clone(), "missing.png".into()],
                    role: PromptRole::User,
                    content: "look".into(),
                    token: 0,
                },
            ],
        };
        let (_, turns) = split_system(&prompt);
        assert_eq!(turns[0].images.len(), 2);

        let openai = openai_adapter::OpenAIClient::body("m", &prompt).to_string();
        assert!(openai.contains("data:image/png;base64,iVBORw=="), "{openai}");
        let claude = claude_adapter::ClaudeClient::body("m", &prompt).to_string();
        assert!(
            claude.contains(r#""media_type":"image/png""#) && claude.contains(r#""text":"look""#),
            "{claude}"
        );
        let gemini = gemini_adapter::gemini_body(&prompt).to_string();
        assert!(gemini.contains("inlineData") && !gemini.contains("missing"), "{gemini}");
    }

    #[test]
    fn unknown_format_is_rejected() {
        assert!(provider_for("openai").is_ok());
        assert!(provider_for("unknown").is_err());
    }
}
