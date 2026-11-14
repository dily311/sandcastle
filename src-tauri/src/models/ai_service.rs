use crate::{
    error::{AppError, AppErrorResponse}, models::conversation::{Message, WorldInfoEntry},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::prelude::FromRow;

// =======================================
// DB 내 테이블 정의
// =======================================
// ---------------------------------------
// API 프로필
// ---------------------------------------
#[derive(FromRow, Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ApiProfiles {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub format_type: String,
    pub base_url: String,
    pub model: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize)]
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
#[derive(Deserialize, Serialize, Clone, Debug)]
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
            config_json: serde_json::from_str(&r.config_json)
                .ok()
                .unwrap_or_default(),
            settings_json: r.settings_json.and_then(|s| serde_json::from_str(&s).ok()),
            compatibility_json: r
                .compatibility_json
                .and_then(|s| serde_json::from_str(&s).ok()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize)]
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
#[derive(Deserialize, Serialize, Clone, Debug)]
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
            triggers_json: serde_json::from_str(&r.triggers_json)
                .ok()
                .unwrap_or_default(),
            extra_json: r.extra_json.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize)]
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
    pub triggers_json: String,
    pub extra_json: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptConfig {}
// =======================================
// AI 연결
// =======================================

#[derive(Serialize)]
pub struct ContextRequest {
    pub conversation_id: String,
    pub character_id: String,
    pub persona_id: String,
    pub history_limit: i64, // 불러올 메시지 수
}

#[derive(Serialize, Clone, Default)]
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
    pub world_info: Vec<WorldInfoEntry>,
    pub history: Vec<HistoryItem>,
    pub extra_data: serde_json::Value,
}

#[derive(Serialize, Clone)]
pub struct HistoryItem {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessageItem {
    pub role: PromptRole,
    pub content: String,
    /// 이 메시지의 추정 토큰 수
    pub token: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>, // Gemini, Anthropic 등에서 주로 사용
    pub stream: bool,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            top_k: None,
            stream: true,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessageResponse {
    pub content: String,
    // 완료 이유 (stop, length 등)
    pub finish_reason: Option<String>,
}

// ---------------------------------------------------------
// Event
// 프론트로 emit되는 이벤트 페이로드
// ---------------------------------------------------------
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]

pub struct DialogueTokenEvent {
    pub conversation_id: String,
    pub delta: String,
}
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]

pub struct DialogueDoneEvent {
    pub conversation_id: String,
    pub full_message: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]

pub struct DialogueErrorEvent {
    pub conversation_id: String,
    pub error: AppErrorResponse,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamPayload {
    pub chunk: String,
}

// =======================================
// openai 형식
// =======================================

#[derive(Clone, Serialize, Deserialize)]
pub struct OpenaiChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
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

#[derive(Deserialize)]
pub struct OpenaiChatCompletionResponse {
    pub choices: Vec<OpenaiChoice>,
}

#[derive(Deserialize)]
pub struct OpenaiChoice {
    pub delta: Option<OpenaiDelta>,
    pub message: Option<OpenaiMessage>,
    pub finish_reason: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenaiMessage {
    pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenaiDelta {
    pub content: Option<String>,
}

// 모델 리스트
#[derive(Deserialize)]
pub struct OpenaiModelList {
    pub data: Vec<OpenaiModel>,
}

#[derive(Deserialize)]
pub struct OpenaiModel {
    pub id: String,
}

// =======================================
// Anthropic 형식
// =======================================

#[derive(Clone, Serialize, Deserialize)]
pub struct AnthropicChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
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
    pub fn from_row(
        model: String,
        messages: Vec<AnthropicChatMessage>,
        options: ChatOptions,
    ) -> Self {
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

#[derive(Deserialize)]
pub struct AnthropicChatCompletionResponse {
    pub content: Option<Vec<AnthropicContentBlock>>,
    pub stop_reason: Option<String>,
}

#[derive(Deserialize)]
pub struct AnthropicContentBlock {
    pub text: Option<String>,
}

// 스트리밍용
#[derive(Deserialize)]
pub struct AnthropicStreamEvent {
    pub r#type: String,
    pub delta: Option<AnthropicDelta>,
}

#[derive(Deserialize)]
pub struct AnthropicDelta {
    pub text: Option<String>,
}

// 모델 리스트
#[derive(Deserialize)]
pub struct AnthropicModelList {
    pub data: Vec<AnthropicModel>,
}

#[derive(Deserialize)]
pub struct AnthropicModel {
    pub id: String,
    pub display_name: Option<String>,
}

// =======================================
// 제미니 & 버텍스
// =======================================
#[derive(Clone, Deserialize)]
pub struct VertexChatRequestAPIKey {
    pub service_account: String,
    pub location: String,
    pub project_id: String,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Clone)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Option<Vec<GeminiPart>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GeminiPart {
    pub text: Option<String>,
}

#[derive(Deserialize)]

pub struct GeminiChatResponse {
    pub candidates: Vec<GeminiCandidates>,
    pub usage_metadata: serde_json::Value, // 객체 형태 또는 Option 처리
    pub model_version: Option<String>,
    pub create_time: Option<String>,
    pub response_id: Option<String>,
}

#[derive(Deserialize)]

pub struct GeminiCandidates {
    pub content: Option<GeminiContent>, // Vec<Content>가 아닌 단일 Content 객체입니다!
    pub finish_reason: Option<String>,
}

// 모델리스트
#[derive(Deserialize)]
pub struct GeminiModelList {
    pub models: Vec<GeminiModel>,
}

#[derive(Deserialize)]
pub struct GeminiModel {
    pub name: String,
    pub display_name: Option<String>,
}
