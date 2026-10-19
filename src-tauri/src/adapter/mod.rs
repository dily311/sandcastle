use anyhow::Result;
use tauri::ipc::Channel;

use crate::models::ai_service::{ApiProfile, ChatOptions, PromptMessage, StreamPayload};

pub mod openai_adapter;
pub mod claude_adapter;
pub mod gemini_adapter;
pub mod vertexai_adapter;
// 그림모델
pub mod nai_adapter;

/** 어뎁터 추가를 위한 기초 trait */
pub trait AIServiceAdapter {
    type ModelList;
    type Message;
    type Request;
    type Response;

    fn parse_message(prompt_message: PromptMessage) -> Vec<Self::Message>;

    async fn get_models(&self, client: reqwest::Client, api_profile: ApiProfile) -> Result<Vec<String>>;
    /** 
     * ai에게 메시지를 전송하는 함수
     * ### 매개변수
     * - key 암호키
     * - http_client
     * - message 보낼 메시지. 문자열로 변환해 전송
     */
    async fn send_message(&self, client: reqwest::Client, api_profile: ApiProfile, prompt_message: PromptMessage) -> Result<String>;
    async fn stream_message(&self, client: reqwest::Client, api_profile: ApiProfile, prompt_message: PromptMessage, on_event: Channel<StreamPayload>) -> Result<String>;
}