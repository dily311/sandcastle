use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::{db::conn::DbPool, error::AppError, models::conversation::{Message, WorldInfoEntry}};

// =======================================
// API 프로필
// =======================================

#[derive(FromRow, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiProfile {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub format_type: String,
    pub base_url: String,
    pub model: String,
    pub selected_preset_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiProfileInput {
    pub name: String,
    pub provider: String,
    pub format_type: String,
    pub base_url: String,
    pub model: String,
    pub selected_preset_id: Option<String>,
}

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

#[derive(Serialize)]
pub struct ChatContext {
    pub system_rules: Vec<String>,
    pub character_info: Option<String>,
    pub persona_info: String,
    pub example_dialogue: Option<String>,
    pub world_info: Vec<WorldInfoEntry>,
    pub history: Vec<HistoryItem>,
    pub extra_data: serde_json::Value,
}

#[derive(Serialize)]
pub struct HistoryItem {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct PromptRequest {
    pub context: ChatContext,
    pub user_message: String,
    pub preset_id: Option<String>,
    pub context_limit: u32, // 전체 컨텍스트 토큰 예산
    pub world_info_ratio: f32, // 월드인포에 할당할 비율
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub chat_options: ChatOptions,
    pub messages: Vec<PromptMessageItem>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PromptMessageItem {
    pub token: i64,
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>, // Gemini, Anthropic 등에서 주로 사용
    pub stream: bool,
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
    pub error: AppError,
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
