use gcp_auth::{CustomServiceAccount, TokenProvider};
use tauri::ipc::Channel;
use anyhow::{anyhow, Context, Result};
use futures::stream::{self, StreamExt};
use crate::{adapter::AIServiceAdapter, models::{ai_service::*}, service::secret_store::{SecretStore, api_key_name}};

pub struct VertexAIClient;
impl AIServiceAdapter for VertexAIClient {
    type Message = GeminiContent;
    type ModelList = GeminiModelList;
    type Request = GeminiChatRequest;
    type Response = GeminiChatResponse;

    fn parse_message(prompt_message: PromptMessage) -> Vec<Self::Message> {
        todo!()
    }
    async fn get_models(&self, client: reqwest::Client, api_profile: ApiProfiles) -> Result<Vec<String>> {
        todo!()
    }

    async fn send_message(&self, client: reqwest::Client, api_profile: ApiProfiles, prompt_message: PromptMessage) -> Result<String> {
        // osSecretStore에 저장된 api 키를 불러옴
        let api_key = SecretStore::get(&api_key_name(&api_profile.provider))?;
        let service_json = serde_json::from_str::<VertexChatRequestAPIKey>(&api_key)?;
        // 서비스 계정 키를 통한 GCP 인증 토큰 발급
        let service_account = CustomServiceAccount::from_json(&service_json.service_account)
            .context("service-account 계정 발급 실패")?;
        let token = service_account
            .token(&["https://www.googleapis.com/auth/cloud-platform"])
            .await
            .context("GVP Auth 토큰 발급 실패")?;

        let url = format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent", 
            service_json.location, 
            service_json.project_id, 
            service_json.location,
            api_profile.model.clone()
        );
        // 메시지 구조 파싱
        let messages = Self::parse_message(prompt_message.clone());
        // request 구조체 생성
        let request = Self::Request::from_row(messages, prompt_message.chat_options);

        let resp = client
            .post(url)
            .bearer_auth(token.as_str())
            .json(&request)
            .send()
            .await?;

        // 오류처리
        if !resp.status().is_success() {
            let stats = resp.status();
            let error_text = resp.text().await.unwrap_or_default();
            return  Err(anyhow!("VertexAi 오류 {stats} : {error_text}"));
        }

        let json = resp.json::<GeminiChatResponse>().await?;
        let result = json.candidates
            .first()
            .and_then(|c| c.content.as_ref().clone().unwrap().parts.as_ref().clone())
            .unwrap()
            .first()
            .map(|m| m.text.clone())
            .ok_or_else(|| anyhow!("텍스트 응답 추출 실패"))?;

        Ok(result.unwrap_or_default())
    }
    
    async fn stream_message(&self, client: reqwest::Client, api_profile: ApiProfiles, prompt_message: PromptMessage, on_event: Channel<StreamPayload>) -> Result<String> {
        todo!()
    }


    
}
