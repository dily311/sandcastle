use tauri::ipc::Channel;
use anyhow::{anyhow, Context, Result};
use futures::stream::{self, StreamExt};
use crate::{adapter::AIServiceAdapter, models::{ai_service::*}, service::secret_store::{SecretStore, api_key_name}};

pub struct GeminiClient;
impl AIServiceAdapter for GeminiClient {
    type Message = GeminiContent;
    type ModelList = GeminiModelList;
    type Request = GeminiChatRequest;
    type Response = GeminiChatResponse;
    
    fn parse_message(prompt_message: PromptMessage) -> Vec<Self::Message> {
        todo!()
    }
    
    async fn get_models(&self, client: reqwest::Client, api_profile: ApiProfile) -> Result<Vec<String>> {
        let url = format!("{}/models", api_profile.base_url.trim_end_matches("/"));

        todo!()
    }
    
    async fn send_message(&self, client: reqwest::Client, api_profile: ApiProfile, prompt_message: PromptMessage) -> Result<String> {
        // {모델이름}:streamGenerateContent?alt=sse` (또는 `generateContent`)
        let url = format!("{}/models/{}:generateContent", api_profile.base_url.trim_end_matches("/"), api_profile.model.trim_end_matches("/"));

        todo!()
    }
    
    async fn stream_message(&self, client: reqwest::Client, api_profile: ApiProfile, prompt_message: PromptMessage, on_event: Channel<StreamPayload>) -> Result<String> {
        todo!()
    }
    
    

}