use crate::{error::AppErrorResponse, models::conversation::WorldInfoEntry};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::prelude::FromRow;

// =======================================
// DB 내 테이블 정의
// =======================================
// ---------------------------------------
// API 프로필
// ---------------------------------------
#[derive(Deserialize, Serialize, Clone, Debug, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApiProfiles {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub format_type: String,
    pub base_url: String,
    /// 이 프로필의 기본 모델. 대화·기능이 따로 정하지 않으면 이것을 쓴다
    pub model: String,
    /// 사용자가 직접 입력해 남겨 둔 모델 이름 (목록에 계속 보임, ADR-020)
    #[serde(default)]
    pub custom_models: Vec<String>,
    /// 연결 확인 여부 (테스트 또는 채팅·게임·폰에서 실제 응답을 받으면 true)
    #[serde(default)]
    pub verified: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

// custom_models가 JSON 문자열로 저장돼 있어 손으로 읽는다 (깨져 있으면 빈 목록)
impl<'r> FromRow<'r, sqlx::sqlite::SqliteRow> for ApiProfiles {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        let custom: String = row.try_get("custom_models").unwrap_or_default();
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            provider: row.try_get("provider")?,
            format_type: row.try_get("format_type")?,
            base_url: row.try_get("base_url")?,
            model: row.try_get("model")?,
            custom_models: serde_json::from_str(&custom).unwrap_or_default(),
            verified: row.try_get::<bool, _>("verified").unwrap_or(false),
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ApiProfilesInput {
    pub name: String,
    pub provider: String,
    pub format_type: String,
    pub base_url: String,
    pub model: String,
}

// ---------------------------------------
// 프롬프트
// ---------------------------------------

#[derive(FromRow)]
pub struct PromptPresetsRows {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub preset_type: String,   // chat_completion / text_completion
    pub source_format: String, // native / sillytavern
    pub config_json: String,
    pub settings_json: Option<String>,
    pub compatibility_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Deserialize, Serialize, Clone, Debug, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptPresets {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub preset_type: String,   // chat_completion / text_completion
    pub source_format: String, // native / sillytavern
    pub config_json: Value,
    pub settings_json: Option<Value>,
    pub compatibility_json: Option<Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<PromptPresetsRows> for PromptPresets {
    fn from(r: PromptPresetsRows) -> Self {
        Self {
            id: r.id,
            name: r.name,
            description: r.description,
            preset_type: r.preset_type,
            source_format: r.source_format,
            config_json: serde_json::from_str(&r.config_json).ok().unwrap_or_default(),
            settings_json: r.settings_json.and_then(|s| serde_json::from_str(&s).ok()),
            compatibility_json: r.compatibility_json.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptPresetsInput {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub preset_type: String,   // chat_completion / text_completion
    pub source_format: String, // native / sillytavern
    pub config_json: Value,
    pub settings_json: Option<Value>,
    pub compatibility_json: Option<Value>,
}

#[derive(FromRow)]
pub struct PromptBlocksRows {
    pub id: String,
    pub preset_id: String,
    pub name: String,
    pub identifier: String,
    pub block_type: String,
    pub role: String,
    pub content: String,
    pub enabled: bool,
    pub position: String,
    pub depth: i64,
    pub sort_order: i64,
    pub triggers_json: String,
    pub extra_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Deserialize, Serialize, Clone, Debug, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptBlocks {
    pub id: String,
    pub preset_id: String,
    pub name: String,
    pub identifier: String,
    pub block_type: String,
    pub role: String,
    pub content: String,
    pub enabled: bool,
    pub position: String,
    pub depth: i64,
    pub sort_order: i64,
    pub triggers_json: Value,
    pub extra_json: Option<Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<PromptBlocksRows> for PromptBlocks {
    fn from(r: PromptBlocksRows) -> Self {
        Self {
            id: r.id,
            preset_id: r.preset_id,
            name: r.name,
            identifier: r.identifier,
            block_type: r.block_type,
            role: r.role,
            content: r.content,
            enabled: r.enabled,
            position: r.position,
            depth: r.depth,
            sort_order: r.sort_order,
            triggers_json: serde_json::from_str(&r.triggers_json).ok().unwrap_or_default(),
            extra_json: r.extra_json.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptBlocksInput {
    pub id: Option<String>,
    pub preset_id: String,
    pub name: String,
    pub identifier: String,
    pub block_type: String,
    pub role: String,
    pub content: String,
    pub enabled: bool,
    pub position: String,
    pub depth: i64,
    pub sort_order: i64,
    /// 생성 유형 배열 (예: ["normal", "continue"]). 비면 항상 사용
    pub triggers_json: Value,
    pub extra_json: Option<Value>,
}

#[derive(Deserialize, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptConfig {}
// =======================================
// AI 연결
// =======================================

#[derive(Serialize, specta::Type)]
pub struct ContextRequest {
    pub conversation_id: String,
    pub character_id: String,
    pub persona_id: String,
    pub history_limit: i64, // 불러올 메시지 수
}

#[derive(Serialize, Clone, Default, specta::Type)]
pub struct ChatContext {
    /// 앱이 고정한 규칙. 프리셋보다 항상 앞에 들어가며 프리셋으로 바꿀 수 없다 (ADR-003)
    pub system_rules: Vec<String>,
    /// 매크로 {{char}}
    pub char_name: Option<String>,
    /// 매크로 {{user}}
    pub user_name: Option<String>,
    pub character_info: Option<String>,
    pub persona_info: String,
    pub example_dialogue: Option<String>,
    /// 캐릭터의 시스템 프롬프트 덮어쓰기: 프리셋의 main 블록 내용을 대신함 (비면 프리셋 그대로)
    pub main_prompt: Option<String>,
    pub world_info: Vec<WorldInfoEntry>,
    pub history: Vec<HistoryItem>,
    pub extra_data: serde_json::Value,
    /// 이번 사용자 입력에 붙인 이미지 (파일 경로)
    pub user_images: Vec<String>,
    /// 캐릭터 이미지(<cg>) 규칙 (앱 프롬프트 슬롯). 프리셋 블록의 {{img_inprompt}} 자리에만 들어간다 (ADR-030 추가)
    pub image_prompt: Option<String>,
    /// 매크로 {{img_keys}}: 이 대화에서 쓸 수 있는 이미지 키 목록
    pub image_keys: Option<String>,
    /// 대화 유틸 지시문 (매크로 이름 → 글): {{auto_image}}·{{multi_char}}·{{vars}}. 프리셋 블록이 매크로를 쓴 자리에만 들어간다 (ADR-030 추가)
    pub util_prompts: std::collections::BTreeMap<String, String>,
}

/// 프롬프트 메시지가 어느 시스템에서 주입되었는지 ('보낸 컨텍스트' 표시용, 공급자 요청에는 쓰지 않음)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptSource {
    /// 주입한 시스템 종류 (preset, world_info, system_rules, character, persona, examples, chat, user_input, qr, author_note, inject, recall, regen_extra, plugin, ghost)
    pub kind: String,
    /// 프리셋 블록 이름, 플러그인 이름, /inject id 등 세부 이름
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl PromptSource {
    pub fn of(kind: &str) -> Self {
        Self {
            kind: kind.to_string(),
            name: None,
        }
    }

    pub fn named(kind: &str, name: impl Into<String>) -> Self {
        let name = name.into();
        let trimmed = name.trim();
        Self {
            kind: kind.to_string(),
            name: (!trimmed.is_empty()).then(|| trimmed.to_string()),
        }
    }
}

#[derive(Serialize, Clone, specta::Type)]
pub struct HistoryItem {
    pub role: String,
    pub content: String,
    /// 메시지에 붙인 이미지 (파일 경로, meta.attachments)
    pub images: Vec<String>,
    /// 대화 기록에 끼워 넣은 시스템 출처 (없으면 일반 대화 기록)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PromptSource>,
}

#[derive(Serialize, specta::Type)]
pub struct PromptRequest {
    pub context: ChatContext,
    pub user_message: String,
    pub preset_id: Option<String>,
    pub context_limit: u32,    // 전체 컨텍스트 토큰 예산 (응답 토큰 포함)
    pub world_info_ratio: f32, // 월드인포에 할당할 비율
    /// 어떤 생성인지 (프롬프트 블록 triggers_json 필터에 사용)
    pub generation_type: GenerationType,
}

/// 생성 유형. prompt_blocks.triggers_json 값과 같은 표기를 쓴다.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum GenerationType {
    #[default]
    Normal,
    Continue,
    Regenerate,
    Swipe,
    Impersonate,
    Quiet,
}

impl GenerationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Continue => "continue",
            Self::Regenerate => "regenerate",
            Self::Swipe => "swipe",
            Self::Impersonate => "impersonate",
            Self::Quiet => "quiet",
        }
    }
}

/// PromptManager가 만든 최종 프롬프트. 공급자 형식과 무관하며 어댑터가 각자 형식으로 바꾼다.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptMessage {
    pub chat_options: ChatOptions,
    pub messages: Vec<PromptMessageItem>,
}

impl PromptMessage {
    pub fn total_tokens(&self) -> i64 {
        self.messages.iter().map(|m| m.token).sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
    System,
    User,
    Assistant,
}

impl PromptRole {
    /// DB/히스토리의 문자열 role → PromptRole
    pub fn from_history(role: &str) -> Self {
        match role.to_ascii_lowercase().as_str() {
            "system" => Self::System,
            "assistant" | "ai" | "char" | "model" | "narrator" => Self::Assistant,
            _ => Self::User,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct PromptMessageItem {
    pub role: PromptRole,
    pub content: String,
    /// 이 메시지의 추정 토큰 수
    pub token: i64,
    /// 함께 보낼 이미지 (파일 경로). 어댑터가 읽어서 공급자 형식으로 넣는다
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    /// 어느 시스템이 주입한 메시지인지 ('보낸 컨텍스트' 표시용)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PromptSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>, // Gemini, Anthropic 등에서 주로 사용
    pub stream: bool,
    /// 추론 강도 (low·medium·high 등). 모델 목록에 이 값을 받는다고 적힌 모델에서만 보냄 (AiService가 걸러냄)
    #[serde(default)]
    pub reasoning_effort: Option<String>,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            top_k: None,
            stream: true,
            reasoning_effort: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct ChatMessageResponse {
    pub content: String,
    // 완료 이유 (stop, length 등)
    pub finish_reason: Option<String>,
}

// ---------------------------------------------------------
// Event
// 프론트로 emit되는 이벤트 페이로드
// ---------------------------------------------------------
#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]

pub struct DialogueTokenEvent {
    pub conversation_id: String,
    pub delta: String,
}
#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]

pub struct DialogueDoneEvent {
    pub conversation_id: String,
    pub full_message: String,
    /// full_message 안의 <cg>키</cg>가 가리키는 이미지 (등장 순서)
    pub images: Vec<crate::models::character_image::ChatImageRef>,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]

pub struct DialogueErrorEvent {
    pub conversation_id: String,
    pub error: AppErrorResponse,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StreamPayload {
    pub chunk: String,
}

// =======================================
// openai 형식
// =======================================

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct OpenaiChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, specta::Type)]
pub struct OpenaiChatCompletionRequest {
    pub model: String,
    pub messages: Vec<OpenaiChatMessage>,
    pub stream: bool,

    // 생성 옵션
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

impl OpenaiChatCompletionRequest {
    pub fn from_row(model: String, messages: Vec<OpenaiChatMessage>, options: ChatOptions) -> Self {
        Self {
            model: model,
            messages: messages,
            stream: options.stream,
            temperature: options.temperature,
            max_tokens: options.max_tokens,
            top_p: options.top_p,
        }
    }
}

#[derive(Deserialize, specta::Type)]
pub struct OpenaiChatCompletionResponse {
    pub choices: Vec<OpenaiChoice>,
}

#[derive(Deserialize, specta::Type)]
pub struct OpenaiChoice {
    pub delta: Option<OpenaiDelta>,
    pub message: Option<OpenaiMessage>,
    pub finish_reason: Option<String>,
}

#[derive(Deserialize, specta::Type)]
pub struct OpenaiMessage {
    pub content: Option<String>,
}

#[derive(Deserialize, specta::Type)]
pub struct OpenaiDelta {
    pub content: Option<String>,
}

// 모델 리스트
#[derive(Deserialize, specta::Type)]
pub struct OpenaiModelList {
    pub data: Vec<OpenaiModel>,
}

#[derive(Deserialize, specta::Type)]
pub struct OpenaiModel {
    pub id: String,
}

// =======================================
// Anthropic 형식
// =======================================

#[derive(Clone, Serialize, Deserialize, specta::Type)]
pub struct AnthropicChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, specta::Type)]
pub struct AnthropicChatCompletionRequest {
    pub model: String,
    pub stream: bool,
    pub messages: Vec<AnthropicChatMessage>,
    // 생성 옵션
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
}

impl AnthropicChatCompletionRequest {
    pub fn from_row(model: String, messages: Vec<AnthropicChatMessage>, options: ChatOptions) -> Self {
        Self {
            model: model,
            stream: options.stream,
            messages: messages,
            max_tokens: options.max_tokens.unwrap_or_default(),
            temperature: options.temperature,
            top_p: options.top_p,
            top_k: options.top_k,
        }
    }
}

#[derive(Deserialize, specta::Type)]
pub struct AnthropicChatCompletionResponse {
    pub content: Option<Vec<AnthropicContentBlock>>,
    pub stop_reason: Option<String>,
}

#[derive(Deserialize, specta::Type)]
pub struct AnthropicContentBlock {
    pub text: Option<String>,
}

// 스트리밍용
#[derive(Deserialize, specta::Type)]
pub struct AnthropicStreamEvent {
    pub r#type: String,
    pub delta: Option<AnthropicDelta>,
}

#[derive(Deserialize, specta::Type)]
pub struct AnthropicDelta {
    pub text: Option<String>,
}

// 모델 리스트
#[derive(Deserialize, specta::Type)]
pub struct AnthropicModelList {
    pub data: Vec<AnthropicModel>,
}

#[derive(Deserialize, specta::Type)]
pub struct AnthropicModel {
    pub id: String,
    pub display_name: Option<String>,
}

// =======================================
// 제미니 & 버텍스
// =======================================
#[derive(Clone, Deserialize, specta::Type)]
pub struct VertexChatRequestAPIKey {
    pub service_account: String,
    pub location: String,
    pub project_id: String,
}

#[derive(Serialize, Deserialize, specta::Type)]
pub struct GeminiChatRequest {
    pub contents: Vec<GeminiContent>,

    // 생성 옵션
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

impl GeminiChatRequest {
    pub fn from_row(contents: Vec<GeminiContent>, options: ChatOptions) -> Self {
        Self {
            contents: contents,
            generation_config: Some(GeminiGenerationConfig {
                max_output_tokens: options.max_tokens,
                temperature: options.temperature,
                top_k: options.top_k,
                top_p: options.top_p,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Option<Vec<GeminiPart>>,
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
pub struct GeminiPart {
    pub text: Option<String>,
}

#[derive(Deserialize, specta::Type)]

pub struct GeminiChatResponse {
    pub candidates: Vec<GeminiCandidates>,
    pub usage_metadata: serde_json::Value, // 객체 형태 또는 Option 처리
    pub model_version: Option<String>,
    pub create_time: Option<String>,
    pub response_id: Option<String>,
}

#[derive(Deserialize, specta::Type)]

pub struct GeminiCandidates {
    pub content: Option<GeminiContent>, // Vec<Content>가 아닌 단일 Content 객체입니다!
    pub finish_reason: Option<String>,
}

// 모델리스트
#[derive(Deserialize, specta::Type)]
pub struct GeminiModelList {
    pub models: Vec<GeminiModel>,
}

#[derive(Deserialize, specta::Type)]
pub struct GeminiModel {
    pub name: String,
    pub display_name: Option<String>,
}
