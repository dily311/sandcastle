//! Google Vertex AI (Gemini 퍼블리셔 모델)
//!
//! credential은 API 키가 아니라 JSON 문자열이다:
//! `{ "service_account": "<서비스 계정 JSON 문자열>", "location": "us-central1", "project_id": "..." }`
use async_trait::async_trait;
use gcp_auth::{CustomServiceAccount, TokenProvider};
use serde_json::Value;

use crate::{
    adapter::{
        ensure_success,
        gemini_adapter::{gemini_body, model_id, read_gemini_response},
        net_err, parse_err, AiProvider, Generation, OnDelta, ProviderCall,
    },
    error::{AppError, ErrorCode, Result},
    models::ai_service::{PromptMessage, VertexChatRequestAPIKey},
};

const SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

pub struct VertexAIClient;

struct VertexAuth {
    token: String,
    location: String,
    project_id: String,
}

impl VertexAIClient {
    async fn auth(credential: &str) -> Result<VertexAuth> {
        let auth_err = |msg: String| AppError::ai_provider(ErrorCode::AiProviderAuthFailed, msg, false);
        let key: VertexChatRequestAPIKey = serde_json::from_str(credential).map_err(|e| auth_err(format!("Vertex 인증 정보 형식이 올바르지 않습니다: {e}")))?;
        let account = CustomServiceAccount::from_json(&key.service_account).map_err(|e| auth_err(format!("서비스 계정을 읽을 수 없습니다: {e}")))?;
        let token = account.token(&[SCOPE]).await.map_err(|e| auth_err(format!("GCP 인증 토큰 발급 실패: {e}")))?;
        Ok(VertexAuth {
            token: token.as_str().to_string(),
            location: key.location,
            project_id: key.project_id,
        })
    }

    fn host(location: &str) -> String {
        if location == "global" {
            "https://aiplatform.googleapis.com".to_string()
        } else {
            format!("https://{location}-aiplatform.googleapis.com")
        }
    }
}

#[async_trait]
impl AiProvider for VertexAIClient {
    async fn list_models(&self, call: &ProviderCall<'_>) -> Result<Vec<String>> {
        let auth = Self::auth(call.credential).await?;
        let url = format!("{}/v1beta1/publishers/google/models", Self::host(&auth.location));
        let resp = call
            .http
            .get(url)
            .bearer_auth(&auth.token)
            .header("x-goog-user-project", &auth.project_id)
            .send()
            .await
            .map_err(net_err)?;
        let json: Value = ensure_success(resp).await?.json().await.map_err(|e| parse_err("모델 목록", e))?;
        Ok(json["publisherModels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str())
                    .map(|n| n.rsplit('/').next().unwrap_or(n).to_string())
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn generate(&self, call: &ProviderCall<'_>, prompt: &PromptMessage, on_delta: &mut OnDelta<'_>) -> Result<Generation> {
        let auth = Self::auth(call.credential).await?;
        let stream = prompt.chat_options.stream;
        let method = if stream { "streamGenerateContent?alt=sse" } else { "generateContent" };
        let url = format!(
            "{}/v1/projects/{}/locations/{}/publishers/google/models/{}:{method}",
            Self::host(&auth.location),
            auth.project_id,
            auth.location,
            model_id(&call.profile.model),
        );
        let resp = call
            .http
            .post(url)
            .bearer_auth(&auth.token)
            .json(&gemini_body(prompt))
            .send()
            .await
            .map_err(net_err)?;
        read_gemini_response(resp, stream, on_delta).await
    }
}
