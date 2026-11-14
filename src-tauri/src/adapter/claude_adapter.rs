use tauri::ipc::Channel;
use anyhow::{anyhow, Context, Result};
use futures::stream::{self, StreamExt};
use crate::{adapter::AIServiceAdapter, models::{ai_service::*}, service::secret_store::{SecretStore, api_key_name}};

pub struct ClaudeClient;
impl AIServiceAdapter for ClaudeClient {
    type ModelList = AnthropicModelList;
    type Message = AnthropicChatMessage;
    type Request = AnthropicChatCompletionRequest;
    type Response = AnthropicChatCompletionRequest;
    
    fn parse_message(prompt_message: PromptMessage) -> Vec<Self::Message> {
        todo!()
    }
    
    async fn get_models(&self, client: reqwest::Client, api_profile: ApiProfiles) -> Result<Vec<String>> {
        let url = format!("{}/models", api_profile.base_url.trim_end_matches("/"));


        todo!()
    }
    
    async fn send_message(&self, client: reqwest::Client, api_profile: ApiProfiles, prompt_message: PromptMessage) -> Result<String> {
        let url = format!("{}/messages", api_profile.base_url.trim_end_matches("/"));

        todo!()
    }
    
    async fn stream_message(&self, client: reqwest::Client, api_profile: ApiProfiles, prompt_message: PromptMessage, on_event: Channel<StreamPayload>) -> Result<String> {
        let url = format!("{}/messages", api_profile.base_url.trim_end_matches("/"));

        todo!()
    }

    
}
