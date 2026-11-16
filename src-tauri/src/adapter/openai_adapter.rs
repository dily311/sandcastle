//! OpenAI 호환 Chat Completions (`/chat/completions`)
use async_trait::async_trait;
use serde_json::{json, Map, Value};

use crate::{
    adapter::{ensure_success, net_err, parse_err, sse::read_sse, AiProvider, Generation, OnDelta, ProviderCall},
    error::Result,
    models::ai_service::PromptMessage,
};

pub struct OpenAIClient;

impl OpenAIClient {
    pub(crate) fn body(model: &str, prompt: &PromptMessage) -> Value {
        let messages: Vec<Value> = prompt
            .messages
            .iter()
            .filter(|m| !m.content.is_empty() || !m.images.is_empty())
            .map(|m| {
                if m.images.is_empty() {
                    return json!({ "role": m.role.as_str(), "content": m.content });
                }
                let mut parts: Vec<Value> = vec![];
                if !m.content.trim().is_empty() {
                    parts.push(json!({ "type": "text", "text": m.content }));
                }
                for (mime, data) in m.images.iter().filter_map(|p| crate::adapter::load_image(p)) {
                    parts.push(json!({ "type": "image_url", "image_url": { "url": format!("data:{mime};base64,{data}") } }));
                }
                json!({ "role": m.role.as_str(), "content": parts })
            })
            .collect();
        let opts = &prompt.chat_options;
        let mut body = Map::new();
        body.insert("model".into(), json!(model));
        body.insert("messages".into(), Value::Array(messages));
        body.insert("stream".into(), json!(opts.stream));
        if let Some(v) = opts.temperature {
            body.insert("temperature".into(), json!(v));
        }
        if let Some(v) = opts.max_tokens {
            body.insert("max_tokens".into(), json!(v));
        }
        if let Some(v) = opts.top_p {
            body.insert("top_p".into(), json!(v));
        }
        Value::Object(body)
    }
}

#[async_trait]
impl AiProvider for OpenAIClient {
    async fn list_models(&self, call: &ProviderCall<'_>) -> Result<Vec<String>> {
        let resp = call
            .http
            .get(call.url("models"))
            .bearer_auth(call.credential)
            .send()
            .await
            .map_err(net_err)?;
        let json: Value = ensure_success(resp)
            .await?
            .json()
            .await
            .map_err(|e| parse_err("모델 목록", e))?;
        Ok(json["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn generate(
        &self,
        call: &ProviderCall<'_>,
        prompt: &PromptMessage,
        on_delta: &mut OnDelta<'_>,
    ) -> Result<Generation> {
        let resp = call
            .http
            .post(call.url("chat/completions"))
            .bearer_auth(call.credential)
            .json(&Self::body(&call.profile.model, prompt))
            .send()
            .await
            .map_err(net_err)?;
        let resp = ensure_success(resp).await?;

        if !prompt.chat_options.stream {
            let json: Value = resp.json().await.map_err(|e| parse_err("OpenAI", e))?;
            let choice = &json["choices"][0];
            return Ok(Generation {
                content: choice["message"]["content"].as_str().unwrap_or_default().to_string(),
                finish_reason: choice["finish_reason"].as_str().map(str::to_string),
                usage: super::usage_from(&json["usage"], "prompt_tokens", "completion_tokens"),
            });
        }

        let mut out = Generation::default();
        read_sse(resp, |ev| {
            if ev.data.trim() == "[DONE]" {
                return Ok(false);
            }
            let json: Value = serde_json::from_str(ev.data).map_err(|e| parse_err("OpenAI 스트림", e))?;
            let choice = &json["choices"][0];
            if let Some(delta) = choice["delta"]["content"].as_str() {
                if !delta.is_empty() {
                    on_delta(delta);
                    out.content.push_str(delta);
                }
            }
            if let Some(reason) = choice["finish_reason"].as_str() {
                out.finish_reason = Some(reason.to_string());
            }
            // 스트림 끝에 usage를 보내는 공급자도 있다 (보내지 않으면 앱이 추정)
            if let Some(u) = super::usage_from(&json["usage"], "prompt_tokens", "completion_tokens") {
                out.usage = Some(u);
            }
            Ok(true)
        })
        .await?;
        Ok(out)
    }
}
