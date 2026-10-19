use tauri::ipc::Channel;
use anyhow::{anyhow, Context, Result};
use futures::stream::{self, StreamExt};
use crate::{adapter::AIServiceAdapter, models::ai_service::*, service::secret_store::{SecretStore, api_key_name}};

pub struct OpenAIClient;
impl AIServiceAdapter for OpenAIClient {
    type ModelList = OpenaiModelList;
    type Message = OpenaiChatMessage;
    type Request = OpenaiChatCompletionRequest;
    type Response = OpenaiChatCompletionResponse;

    fn parse_message(prompt_message: PromptMessage) -> Vec<Self::Message> {
        let result = Vec::new();
        result
    }
    
    async fn get_models(&self, client: reqwest::Client, api_profile: ApiProfile) -> Result<Vec<String>> {
        // osSecretStore에 저장된 api 키를 불러옴
        let api_key = SecretStore::get(&api_key_name(&api_profile.provider))?;

        let url = format!("{}/models", api_profile.base_url.trim_end_matches("/"));
        let resp = client
        .get(url)
        .bearer_auth(api_key)
        .send()
        .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            return Err(anyhow!(err));
        }

        let result = resp.json::<Self::ModelList>().await?;
        let data = result.data.into_iter().map(|r| r.id.clone()).collect();
        Ok(data)
    }

    async fn send_message(&self, client: reqwest::Client, api_profile: ApiProfile, prompt_message: PromptMessage) -> Result<String> {
        // osSecretStore에 저장된 api 키를 불러옴
        let api_key = SecretStore::get(&api_key_name(&api_profile.provider))?;
        let url = format!("{}/chat/completions", api_profile.base_url.trim_end_matches("/"));
        
        // 메시지 구조 파싱
        let messages = Self::parse_message(prompt_message.clone());
        // request 구조체 생성
        let request = Self::Request::from_row(api_profile.model.clone(), messages, prompt_message.chat_options.clone());

        let resp = client
            .post(url)
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            return Err(anyhow!(err));
        }

        let result = resp.json::<OpenaiChatCompletionResponse>().await?;


        Ok(
            result
            .choices
            .first()
            .and_then(|c| c.message.as_ref().unwrap().content.clone())
            .unwrap_or_default()
        )
    }

    async fn stream_message(&self, client: reqwest::Client, api_profile: ApiProfile, prompt_message: PromptMessage, on_event: Channel<StreamPayload>) -> Result<String> {
        // osSecretStore에 저장된 api 키를 불러옴
        let api_key = SecretStore::get(&api_key_name(&api_profile.provider))?;
        
        // 보낼 url과 바디 생성
        let url = format!("{}/chat/completions", api_profile.base_url.trim_end_matches("/"));

        // 메시지 구조 파싱
        let messages = Self::parse_message(prompt_message.clone());
        // request 구조체 생성
        let request = Self::Request::from_row(api_profile.model.clone(), messages, prompt_message.chat_options.clone());


        let resp = client
            .get(url)
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .await?;

        let mut result = String::from("");
        let mut stream = resp.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                        let _ = on_event.send(StreamPayload { chunk: text.clone() });
                        result.push_str(&text);
                    }
                }
                Err(e) => {
                    return Err(anyhow!("StreamError: {}", e));
                }
            }
        }

        Ok(result)
    }
}