//! 공급자·모델별 지원 표 (CLAUDE.md 3-A-4, ADR-013).
//!
//! 임베딩·도구 호출·구조화 출력이 이 연결(공급자 형식 + 주소 + 모델)에서 되는지 **호출하지 않고** 알려 준다.
//! 상태는 셋: 지원함 / 알 수 없음(막지 않고 시도 가능 — 호환 서버·새 모델) / 지원 안 함(이유와 함께, 공급자를 부르기 전에 거절).
//! 이름 규칙은 아는 것만 적고, 모르면 "알 수 없음"으로 둔다 — 틀리게 막는 것보다 시도해 보게 하는 쪽이 낫다.
//! 구조화 출력은 "지원함"일 때만 공급자의 네이티브 기능을 쓰고, 그 밖에는 지시 + 검증으로 대신한다(`service/ai/structured.rs`).
use serde::Serialize;

use crate::error::{AppError, ErrorCode, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SupportStatus {
    Supported,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Support {
    pub status: SupportStatus,
    /// 화면에 보일 이유·안내
    pub reason: String,
}

impl Support {
    fn yes(reason: &str) -> Self {
        Self {
            status: SupportStatus::Supported,
            reason: reason.into(),
        }
    }
    fn unknown(reason: &str) -> Self {
        Self {
            status: SupportStatus::Unknown,
            reason: reason.into(),
        }
    }
    fn no(reason: &str) -> Self {
        Self {
            status: SupportStatus::Unsupported,
            reason: reason.into(),
        }
    }
    pub fn is_supported(&self) -> bool {
        self.status == SupportStatus::Supported
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Embeddings,
    Tools,
    Structured,
}

impl Feature {
    pub fn label(self) -> &'static str {
        match self {
            Self::Embeddings => "임베딩",
            Self::Tools => "도구 호출",
            Self::Structured => "구조화 출력",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AiCapabilities {
    pub embeddings: Support,
    pub tools: Support,
    pub structured: Support,
}

impl AiCapabilities {
    pub fn get(&self, f: Feature) -> &Support {
        match f {
            Feature::Embeddings => &self.embeddings,
            Feature::Tools => &self.tools,
            Feature::Structured => &self.structured,
        }
    }

    /// "지원 안 함"이면 공급자를 부르기 전에 거절한다 (요금 없음). 알 수 없음은 통과
    pub fn ensure(&self, f: Feature, profile_name: &str) -> Result<()> {
        let s = self.get(f);
        if s.status == SupportStatus::Unsupported {
            return Err(AppError::ai_provider(
                ErrorCode::AiFeatureUnsupported,
                format!("'{profile_name}' 연결은 {}을(를) 지원하지 않음: {}", f.label(), s.reason),
                false,
            ));
        }
        Ok(())
    }
}

/// 공식 OpenAI 주소인가 (호환 서버는 주소가 제각각이라 기능을 알 수 없다)
pub(crate) fn is_official_openai(base_url: &str) -> bool {
    let host = base_url.trim().trim_start_matches("https://").trim_start_matches("http://");
    host.split('/').next().is_some_and(|h| h.eq_ignore_ascii_case("api.openai.com"))
}

pub fn capabilities(format_type: &str, base_url: &str, model: &str) -> AiCapabilities {
    let mut c = by_name(format_type, base_url, model);
    // 모델 목록(default_text_models.json)에 이 모델이 할 수 있다고 적어 둔 것은 이름 규칙보다 우선한다.
    // 적어 두지 않은 것은 이름 규칙 그대로 (없다고 단정하지 않음). OpenAI 호환 서버는 같은 id여도 서버마다 달라 건드리지 않는다
    let official = !matches!(format_type, "openai") || is_official_openai(base_url);
    if let (true, Some(meta)) = (official, crate::adapter::catalog::meta(model)) {
        if meta.has("tools") && c.tools.status != SupportStatus::Unsupported {
            c.tools = Support::yes("모델 목록에 도구 호출이 적혀 있음");
        }
        if meta.has("structuredOutput") && c.structured.status != SupportStatus::Unsupported {
            c.structured = Support::yes("모델 목록에 구조화 출력이 적혀 있음");
        }
    }
    c
}

fn by_name(format_type: &str, base_url: &str, model: &str) -> AiCapabilities {
    let m = model.trim().to_ascii_lowercase();
    let m = m.trim_start_matches("models/");
    match format_type {
        "openai" => openai(base_url, m),
        "claude" | "anthropic" => claude(m),
        "gemini" | "google" => gemini(m, false),
        "vertexai" | "vertex" => gemini(m, true),
        other => {
            let none = Support::no(&format!("지원하지 않는 API 형식 ({other})"));
            AiCapabilities {
                embeddings: none.clone(),
                tools: none.clone(),
                structured: none,
            }
        }
    }
}

fn openai(base_url: &str, m: &str) -> AiCapabilities {
    if !is_official_openai(base_url) {
        let why = "OpenAI 호환 서버라 서버마다 다름 — 보내 봐야 앎";
        return AiCapabilities {
            embeddings: Support::unknown(why),
            tools: Support::unknown(why),
            // 네이티브 json_schema는 서버가 모를 수 있어 지시 + 검증으로 대신한다
            structured: Support::unknown("호환 서버는 지시 + 검증으로 대신함"),
        };
    }
    let reasoning_old = m.starts_with("o1-mini") || m.starts_with("o1-preview");
    let chat_family = m.starts_with("gpt-") || m.starts_with("chatgpt-") || m.starts_with("o1") || m.starts_with("o3") || m.starts_with("o4");
    let not_text = m.contains("embedding") || m.contains("whisper") || m.contains("tts") || m.contains("dall-e");
    let tools = if not_text {
        Support::no("글 생성 모델이 아님")
    } else if reasoning_old || m.contains("instruct") {
        Support::no("이 모델은 도구 호출을 지원하지 않음")
    } else if chat_family {
        Support::yes("OpenAI 도구 호출")
    } else {
        Support::unknown("모델 이름을 몰라 시도해 봐야 앎")
    };
    let native_json_schema = ((m.starts_with("gpt-4o") && !m.contains("2024-05-13")) && !m.starts_with("gpt-4o-audio"))
        || m.starts_with("gpt-4.1")
        || m.starts_with("gpt-4.5")
        || m.starts_with("gpt-5")
        || (m.starts_with("o1") && !reasoning_old)
        || m.starts_with("o3")
        || m.starts_with("o4");
    // 도구는 못 써도 글은 쓰므로 구조화 출력은 지시 + 검증으로 대신할 수 있다 (글 생성 모델이 아니면 그것도 못 함)
    let structured = if not_text {
        tools.clone()
    } else if native_json_schema {
        Support::yes("OpenAI json_schema 응답 형식")
    } else {
        Support::unknown("이 모델은 지시 + 검증으로 대신함")
    };
    AiCapabilities {
        embeddings: Support::yes("OpenAI /embeddings"),
        tools,
        structured,
    }
}

fn claude(m: &str) -> AiCapabilities {
    let old = m.starts_with("claude-2") || m.starts_with("claude-instant");
    let tools = if old {
        Support::no("Claude 2 이하는 도구 호출이 없음")
    } else {
        Support::yes("Claude 도구 호출")
    };
    let structured = if old {
        Support::unknown("이 모델은 지시 + 검증으로 대신함")
    } else {
        Support::yes("도구 하나를 강제해 스키마에 맞춘 값을 받음")
    };
    AiCapabilities {
        embeddings: Support::no("Anthropic은 임베딩 API가 없음 — 임베딩은 OpenAI 호환(예: Voyage)이나 Gemini 연결로"),
        tools,
        structured,
    }
}

fn gemini(m: &str, vertex: bool) -> AiCapabilities {
    let embeddings = Support::yes(if vertex { "Vertex predict 임베딩" } else { "Gemini batchEmbedContents" });
    let (tools, structured) = if m.starts_with("gemini") && !m.contains("vision") && !m.contains("embedding") {
        (Support::yes("Gemini 함수 호출"), Support::yes("Gemini responseSchema"))
    } else if m.contains("embedding") {
        let why = Support::no("임베딩 전용 모델이라 글 생성·도구를 쓸 수 없음");
        (why.clone(), why)
    } else {
        let why = "모델 이름을 몰라 시도해 봐야 앎";
        (Support::unknown(why), Support::unknown(why))
    };
    AiCapabilities { embeddings, tools, structured }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(c: &AiCapabilities) -> [SupportStatus; 3] {
        [c.embeddings.status, c.tools.status, c.structured.status]
    }
    use SupportStatus::{Supported as Y, Unknown as U, Unsupported as N};

    #[test]
    fn table_by_provider_and_model() {
        let off = "https://api.openai.com/v1";
        assert_eq!(s(&capabilities("openai", off, "gpt-4o")), [Y, Y, Y]);
        assert_eq!(s(&capabilities("openai", off, "gpt-4o-2024-05-13")), [Y, Y, U], "옛 4o는 json_schema 모름");
        assert_eq!(s(&capabilities("openai", off, "gpt-5-mini")), [Y, Y, Y]);
        assert_eq!(s(&capabilities("openai", off, "gpt-3.5-turbo")), [Y, Y, U]);
        assert_eq!(
            s(&capabilities("openai", off, "o1-mini")),
            [Y, N, U],
            "도구는 못 써도 글은 쓰니 구조화는 대체 경로"
        );
        assert_eq!(s(&capabilities("openai", off, "o3-mini")), [Y, Y, Y]);
        assert_eq!(s(&capabilities("openai", off, "gpt-3.5-turbo-instruct")), [Y, N, U]);
        assert_eq!(
            s(&capabilities("openai", off, "text-embedding-3-small")),
            [Y, N, N],
            "임베딩 모델로는 글을 못 씀"
        );
        assert_eq!(s(&capabilities("openai", off, "my-finetune"))[1], U);
        // 호환 서버·로컬 모델은 전부 알 수 없음 (막지 않는다)
        for url in [
            "http://localhost:1234/v1",
            "https://openrouter.ai/api/v1",
            "https://api.openai.com.evil.example/v1",
        ] {
            assert_eq!(s(&capabilities("openai", url, "gpt-4o")), [U, U, U], "{url}");
        }
        assert_eq!(s(&capabilities("claude", "https://api.anthropic.com/v1", "claude-sonnet-5-5")), [N, Y, Y]);
        assert_eq!(s(&capabilities("anthropic", "", "claude-2.1")), [N, N, U]);
        assert_eq!(s(&capabilities("gemini", "", "models/gemini-2.5-flash")), [Y, Y, Y]);
        assert_eq!(s(&capabilities("vertexai", "", "gemini-2.5-pro")), [Y, Y, Y]);
        assert_eq!(s(&capabilities("gemini", "", "gemma-3-27b-it")), [Y, U, U]);
        assert_eq!(s(&capabilities("gemini", "", "gemini-embedding-001")), [Y, N, N]);
        assert_eq!(s(&capabilities("nai", "", "x")), [N, N, N]);
    }

    #[test]
    fn unsupported_is_refused_with_the_reason_and_unknown_passes() {
        let c = capabilities("claude", "", "claude-sonnet-5-5");
        let e = c.ensure(Feature::Embeddings, "내 Claude").unwrap_err().to_response();
        assert_eq!(e.code, ErrorCode::AiFeatureUnsupported);
        assert!(
            !e.retryable && e.message.contains("내 Claude") && e.message.contains("임베딩") && e.message.contains("Voyage"),
            "{}",
            e.message
        );
        assert!(c.ensure(Feature::Tools, "x").is_ok());
        assert!(capabilities("openai", "http://localhost:1/v1", "m").ensure(Feature::Tools, "x").is_ok());
    }
}
