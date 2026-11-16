//! Google Gemini (AI Studio) — `models/{model}:generateContent`
//!
//! 요청/응답 형식은 Vertex AI와 같아서 `gemini_body` / `read_gemini_response`를 Vertex 어댑터도 쓴다.
use async_trait::async_trait;
use reqwest::Response;
use serde_json::{json, Map, Value};

use crate::{
    adapter::{
        ensure_success, load_image, net_err, parse_err, split_system, sse::read_sse, AiProvider, Generation, OnDelta,
        ProviderCall,
    },
    error::Result,
    models::ai_service::{PromptMessage, PromptRole},
};

pub struct GeminiClient;

/// "models/gemini-2.5-flash" / "gemini-2.5-flash" 둘 다 허용
pub(crate) fn model_id(model: &str) -> &str {
    model.trim_start_matches("models/").trim_end_matches('/')
}

pub(crate) fn gemini_body(prompt: &PromptMessage) -> Value {
    let (system, turns) = split_system(prompt);
    let opts = &prompt.chat_options;
    let mut body = Map::new();
    if let Some(system) = system {
        body.insert(
            "systemInstruction".into(),
            json!({ "parts": [{ "text": system }] }),
        );
    }
    body.insert(
        "contents".into(),
        Value::Array(
            turns
                .iter()
                .map(|t| {
                    let role = if t.role == PromptRole::Assistant { "model" } else { "user" };
                    let mut parts: Vec<Value> = t
                        .images
                        .iter()
                        .filter_map(|p| load_image(p))
                        .map(|(mime, data)| json!({ "inlineData": { "mimeType": mime, "data": data } }))
                        .collect();
                    if !t.content.trim().is_empty() || parts.is_empty() {
                        parts.push(json!({ "text": t.content }));
                    }
                    json!({ "role": role, "parts": parts })
                })
                .collect(),
        ),
    );
    let mut config = Map::new();
    if let Some(v) = opts.temperature {
        config.insert("temperature".into(), json!(v));
    }
    if let Some(v) = opts.max_tokens {
        config.insert("maxOutputTokens".into(), json!(v));
    }
    if let Some(v) = opts.top_p {
        config.insert("topP".into(), json!(v));
    }
    if let Some(v) = opts.top_k {
        config.insert("topK".into(), json!(v));
    }
    if !config.is_empty() {
        body.insert("generationConfig".into(), Value::Object(config));
    }
    Value::Object(body)
}

/// 응답 한 덩어리에서 (텍스트, finishReason) 추출. thought 파트는 제외.
fn candidate_text(json: &Value) -> (String, Option<String>) {
    let cand = &json["candidates"][0];
    let text = cand["content"]["parts"]
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter(|p| p["thought"] != true)
                .filter_map(|p| p["text"].as_str())
                .collect::<String>()
        })
        .unwrap_or_default();
    (text, cand["finishReason"].as_str().map(str::to_string))
}

/// generateContent / streamGenerateContent?alt=sse 응답 공통 처리
pub(crate) async fn read_gemini_response(
    resp: Response,
    stream: bool,
    on_delta: &mut OnDelta<'_>,
) -> Result<Generation> {
    let resp = ensure_success(resp).await?;
    if !stream {
        let json: Value = resp.json().await.map_err(|e| parse_err("Gemini", e))?;
        let (content, finish_reason) = candidate_text(&json);
        return Ok(Generation {
            content,
            finish_reason,
            usage: super::usage_from(&json["usageMetadata"], "promptTokenCount", "candidatesTokenCount"),
        });
    }

    let mut out = Generation::default();
    read_sse(resp, |ev| {
        let json: Value = serde_json::from_str(ev.data).map_err(|e| parse_err("Gemini 스트림", e))?;
        let (delta, reason) = candidate_text(&json);
        if !delta.is_empty() {
            on_delta(&delta);
            out.content.push_str(&delta);
        }
        if reason.is_some() {
            out.finish_reason = reason;
        }
        // 조각마다 지금까지의 합계가 오므로 마지막 값을 쓴다
        if let Some(u) = super::usage_from(&json["usageMetadata"], "promptTokenCount", "candidatesTokenCount") {
            out.usage = Some(u);
        }
        Ok(true)
    })
    .await?;
    Ok(out)
}

#[async_trait]
impl AiProvider for GeminiClient {
    async fn list_models(&self, call: &ProviderCall<'_>) -> Result<Vec<String>> {
        let resp = call
            .http
            .get(call.url("models"))
            .header("x-goog-api-key", call.credential)
            .query(&[("pageSize", "1000")])
            .send()
            .await
            .map_err(net_err)?;
        let json: Value = ensure_success(resp)
            .await?
            .json()
            .await
            .map_err(|e| parse_err("모델 목록", e))?;
        Ok(json["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|m| {
                        m["supportedGenerationMethods"]
                            .as_array()
                            .map(|ms| ms.iter().any(|x| x == "generateContent"))
                            .unwrap_or(true)
                    })
                    .filter_map(|m| m["name"].as_str().map(|n| model_id(n).to_string()))
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
        let stream = prompt.chat_options.stream;
        let model = model_id(&call.profile.model);
        let url = if stream {
            call.url(&format!("models/{model}:streamGenerateContent?alt=sse"))
        } else {
            call.url(&format!("models/{model}:generateContent"))
        };
        let resp = call
            .http
            .post(url)
            .header("x-goog-api-key", call.credential)
            .json(&gemini_body(prompt))
            .send()
            .await
            .map_err(net_err)?;
        read_gemini_response(resp, stream, on_delta).await
    }
}
