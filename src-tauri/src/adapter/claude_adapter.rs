//! Anthropic Messages API (`/messages`)
use async_trait::async_trait;
use reqwest::RequestBuilder;
use serde_json::{json, Map, Value};

use crate::{
    adapter::{
        ensure_success, load_image, net_err, parse_err, split_system, sse::read_sse, AiProvider, Generation, OnDelta,
        ProviderCall,
    },
    error::{AppError, ErrorCode, Result},
    models::ai_service::PromptMessage,
};

const ANTHROPIC_VERSION: &str = "2023-06-01";
/// max_tokens는 Anthropic에서 필수
const DEFAULT_MAX_TOKENS: u32 = 1024;

pub struct ClaudeClient;

impl ClaudeClient {
    fn auth(req: RequestBuilder, call: &ProviderCall<'_>) -> RequestBuilder {
        req.header("x-api-key", call.credential)
            .header("anthropic-version", ANTHROPIC_VERSION)
    }

    pub(crate) fn body(model: &str, prompt: &PromptMessage) -> Value {
        let (system, turns) = split_system(prompt);
        let opts = &prompt.chat_options;
        let mut body = Map::new();
        body.insert("model".into(), json!(model));
        body.insert(
            "max_tokens".into(),
            json!(opts.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
        );
        body.insert("stream".into(), json!(opts.stream));
        if let Some(system) = system {
            body.insert("system".into(), json!(system));
        }
        body.insert(
            "messages".into(),
            Value::Array(
                turns
                    .iter()
                    .map(|t| {
                        if t.images.is_empty() {
                            return json!({ "role": t.role.as_str(), "content": t.content });
                        }
                        let mut parts: Vec<Value> = t
                            .images
                            .iter()
                            .filter_map(|p| load_image(p))
                            .map(|(mime, data)| json!({ "type": "image", "source": { "type": "base64", "media_type": mime, "data": data } }))
                            .collect();
                        if !t.content.trim().is_empty() {
                            parts.push(json!({ "type": "text", "text": t.content }));
                        }
                        json!({ "role": t.role.as_str(), "content": parts })
                    })
                    .collect(),
            ),
        );
        if let Some(v) = opts.temperature {
            body.insert("temperature".into(), json!(v));
        }
        if let Some(v) = opts.top_p {
            body.insert("top_p".into(), json!(v));
        }
        if let Some(v) = opts.top_k {
            body.insert("top_k".into(), json!(v));
        }
        Value::Object(body)
    }
}

#[async_trait]
impl AiProvider for ClaudeClient {
    async fn list_models(&self, call: &ProviderCall<'_>) -> Result<Vec<String>> {
        let resp = Self::auth(call.http.get(call.url("models")), call)
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
        let resp = Self::auth(call.http.post(call.url("messages")), call)
            .json(&Self::body(&call.profile.model, prompt))
            .send()
            .await
            .map_err(net_err)?;
        let resp = ensure_success(resp).await?;

        if !prompt.chat_options.stream {
            let json: Value = resp.json().await.map_err(|e| parse_err("Claude", e))?;
            let content = json["content"]
                .as_array()
                .map(|blocks| {
                    blocks
                        .iter()
                        .filter(|b| b["type"] == "text")
                        .filter_map(|b| b["text"].as_str())
                        .collect::<String>()
                })
                .unwrap_or_default();
            return Ok(Generation {
                content,
                finish_reason: json["stop_reason"].as_str().map(str::to_string),
            });
        }

        let mut out = Generation::default();
        read_sse(resp, |ev| {
            let json: Value = serde_json::from_str(ev.data).map_err(|e| parse_err("Claude 스트림", e))?;
            match json["type"].as_str().unwrap_or(ev.event.unwrap_or_default()) {
                "content_block_delta" => {
                    if let Some(text) = json["delta"]["text"].as_str() {
                        on_delta(text);
                        out.content.push_str(text);
                    }
                }
                "message_delta" => {
                    if let Some(reason) = json["delta"]["stop_reason"].as_str() {
                        out.finish_reason = Some(reason.to_string());
                    }
                }
                "message_stop" => return Ok(false),
                "error" => {
                    let message = json["error"]["message"].as_str().unwrap_or("스트림 오류");
                    let overloaded = json["error"]["type"] == "overloaded_error";
                    return Err(AppError::ai_provider(
                        ErrorCode::AiProviderError,
                        message.to_string(),
                        overloaded,
                    ));
                }
                _ => {}
            }
            Ok(true)
        })
        .await?;
        Ok(out)
    }
}
