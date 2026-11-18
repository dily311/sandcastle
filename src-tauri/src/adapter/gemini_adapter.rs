//! Google Gemini (AI Studio) — `models/{model}:generateContent` + `batchEmbedContents`
//!
//! 요청/응답 형식은 Vertex AI와 같아서 `gemini_body_with` / `read_gemini_response`를 Vertex 어댑터도 쓴다.
//!
//! 도구 호출: `functionDeclarations` / `functionCall` / `functionResponse`. Gemini는 호출 id를 (보통) 주지 않아 앱이 `call_<표>_<번호>`를 만들고,
//! 생각하는 모델의 `thoughtSignature`는 **응답 파트 전체를 원본 그대로** `ToolTurn::Assistant.state`에 보존했다가 다음 턴에 되돌려 보낸다
//! (서명은 호출 파트뿐 아니라 글 파트·빈 글 파트에도 붙을 수 있다). 인접한 순수 글 파트만 합친다.
use async_trait::async_trait;
use reqwest::Response;
use serde_json::{json, Map, Value};

use crate::{
    adapter::{
        ensure_success, l2_normalize, load_image, net_err, openai_adapter::floats, parse_err, schema::to_gemini_schema, split_system, sse::read_sse,
        tools::CallAssembler, AiProvider, ChatExtras, EmbedRequest, EmbedTask, Embeddings, Generation, OnDelta, ProviderCall, ProviderState, StopKind,
        ToolCall, ToolChoice, ToolTurn,
    },
    error::{AppError, ErrorCode, Result},
    models::ai_service::{PromptMessage, PromptRole},
};

pub struct GeminiClient;

/// batchEmbedContents 한 번에 보내는 글 수 (공급자 한도 100)
const EMBED_BATCH: usize = 100;
const STATE_KEY: &str = "gemini";

/// "models/gemini-2.5-flash" / "gemini-2.5-flash" 둘 다 허용
pub(crate) fn model_id(model: &str) -> &str {
    model.trim_start_matches("models/").trim_end_matches('/')
}

/// 사용량: 입력 = promptTokenCount, 출력 = candidatesTokenCount + thoughtsTokenCount.
/// (생각 토큰은 후보 토큰에 들어 있지 않은데 출력 요금으로 청구된다 — 빼면 예산을 낮게 센다)
pub(crate) fn gemini_usage(meta: &Value) -> Option<(u64, u64)> {
    let i = meta["promptTokenCount"].as_u64();
    let c = meta["candidatesTokenCount"].as_u64();
    let t = meta["thoughtsTokenCount"].as_u64();
    (i.is_some() || c.is_some()).then(|| (i.unwrap_or(0), c.unwrap_or(0) + t.unwrap_or(0)))
}

/// 응답 원본에서 공급자가 준 functionCall id들 (있을 때만 functionResponse에 되돌려 보낸다)
fn state_call_ids(state: &Option<ProviderState>) -> Vec<String> {
    state
        .as_ref()
        .filter(|s| s.provider == STATE_KEY)
        .and_then(|s| s.raw.as_array())
        .map(|parts| parts.iter().filter_map(|p| p["functionCall"]["id"].as_str().map(String::from)).collect())
        .unwrap_or_default()
}

/// 도구 루프 꼬리 → contents. 모델 턴은 원본 parts가 있으면 그대로, 없으면 글 + functionCall. 결과는 한 user 턴에 호출 순서대로
fn tail_contents(tail: &[ToolTurn]) -> Vec<Value> {
    let mut out = vec![];
    let mut last_calls: Vec<&ToolCall> = vec![];
    let mut api_ids: Vec<String> = vec![];
    for turn in tail {
        match turn {
            ToolTurn::Assistant { text, calls, state } => {
                let parts = match state.as_ref().filter(|s| s.provider == STATE_KEY && s.raw.is_array()) {
                    Some(s) => s.raw.clone(),
                    None => {
                        let mut parts: Vec<Value> = vec![];
                        if !text.trim().is_empty() {
                            parts.push(json!({ "text": text }));
                        }
                        for c in calls {
                            let args = if c.parse_error.is_some() { json!({}) } else { c.arguments.clone() };
                            parts.push(json!({ "functionCall": { "name": c.name, "args": args } }));
                        }
                        Value::Array(parts)
                    }
                };
                if parts.as_array().is_some_and(|a| !a.is_empty()) {
                    out.push(json!({ "role": "model", "parts": parts }));
                }
                last_calls = calls.iter().collect();
                api_ids = state_call_ids(state);
            }
            ToolTurn::Results { results, note } => {
                let mut ordered: Vec<_> = results.iter().collect();
                ordered.sort_by_key(|r| last_calls.iter().position(|c| c.id == r.call_id).unwrap_or(usize::MAX));
                let mut parts: Vec<Value> = ordered
                    .iter()
                    .map(|r| {
                        // response는 JSON 객체여야 한다
                        let response = if r.is_error {
                            json!({ "error": r.content })
                        } else {
                            json!({ "result": r.content })
                        };
                        let mut fr = json!({ "name": r.name, "response": response });
                        if api_ids.contains(&r.call_id) {
                            fr["id"] = json!(r.call_id);
                        }
                        json!({ "functionResponse": fr })
                    })
                    .collect();
                if let Some(n) = note.as_deref().filter(|n| !n.trim().is_empty()) {
                    parts.push(json!({ "text": n }));
                }
                if !parts.is_empty() {
                    out.push(json!({ "role": "user", "parts": parts }));
                }
            }
        }
    }
    out
}

#[cfg(test)]
pub(crate) fn gemini_body(prompt: &PromptMessage) -> Value {
    gemini_body_with(prompt, &ChatExtras::default())
}

pub(crate) fn gemini_body_with(prompt: &PromptMessage, extras: &ChatExtras) -> Value {
    let (system, turns) = split_system(prompt);
    let opts = &prompt.chat_options;
    let mut body = Map::new();
    if let Some(system) = system {
        body.insert("systemInstruction".into(), json!({ "parts": [{ "text": system }] }));
    }
    let mut contents: Vec<Value> = turns
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
        .collect();
    contents.extend(tail_contents(&extras.tail));
    body.insert("contents".into(), Value::Array(contents));

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
    if let Some(rs) = &extras.response_schema {
        config.insert("responseMimeType".into(), json!("application/json"));
        config.insert("responseSchema".into(), to_gemini_schema(&rs.schema));
    }
    if !config.is_empty() {
        body.insert("generationConfig".into(), Value::Object(config));
    }

    if !extras.tools.is_empty() {
        let decls: Vec<Value> = extras
            .tools
            .iter()
            .map(|t| {
                let mut d = json!({ "name": t.name, "description": t.description });
                // 인자가 없는 함수에 빈 object 스키마를 주면 400이라 parameters를 뺀다
                if t.parameters.get("properties").and_then(Value::as_object).is_some_and(|p| !p.is_empty()) {
                    d["parameters"] = to_gemini_schema(&t.parameters);
                }
                d
            })
            .collect();
        body.insert("tools".into(), json!([{ "functionDeclarations": decls }]));
        let config = match &extras.tool_choice {
            ToolChoice::Auto => None,
            ToolChoice::None => Some(json!({ "mode": "NONE" })),
            ToolChoice::Required => Some(json!({ "mode": "ANY" })),
            // allowedFunctionNames는 mode ANY일 때만 쓴다
            ToolChoice::Named(n) => Some(json!({ "mode": "ANY", "allowedFunctionNames": [n] })),
        };
        if let Some(c) = config {
            body.insert("toolConfig".into(), json!({ "functionCallingConfig": c }));
        }
    }
    Value::Object(body)
}

/// 응답을 모으는 상태 (스트림은 조각마다, 비스트림은 한 번)
#[derive(Default)]
struct Acc {
    content: String,
    /// 응답 파트 원본 (서명 보존용)
    parts: Vec<Value>,
    asm: CallAssembler,
    calls: usize,
    reason: Option<String>,
    usage: Option<(u64, u64)>,
}

/// 서명도 생각 표시도 없는 순수 글 파트인가 (인접한 것끼리 합쳐도 되는)
fn plain_text(p: &Value) -> bool {
    p.as_object().is_some_and(|o| o.len() == 1 && o.get("text").is_some_and(Value::is_string))
}

fn absorb(json: &Value, acc: &mut Acc, on_delta: &mut OnDelta<'_>) {
    let cand = &json["candidates"][0];
    if let Some(parts) = cand["content"]["parts"].as_array() {
        for part in parts {
            if let Some(fc) = part.get("functionCall") {
                let index = acc.calls;
                acc.calls += 1;
                acc.asm.start(index, fc["id"].as_str(), fc["name"].as_str());
                if !fc["args"].is_null() {
                    acc.asm.push_args(index, &fc["args"].to_string());
                }
            } else if part["thought"] != true {
                if let Some(text) = part["text"].as_str() {
                    if !text.is_empty() {
                        on_delta(text);
                        acc.content.push_str(text);
                    }
                }
            }
            if plain_text(part) && acc.parts.last().is_some_and(plain_text) {
                let add = part["text"].as_str().unwrap_or_default().to_string();
                if let Some(last) = acc.parts.last_mut() {
                    let merged = format!("{}{}", last["text"].as_str().unwrap_or_default(), add);
                    last["text"] = json!(merged);
                }
            } else {
                acc.parts.push(part.clone());
            }
        }
    }
    if let Some(r) = cand["finishReason"].as_str() {
        acc.reason = Some(r.to_string());
    }
    // 조각마다 지금까지의 합계가 오므로 마지막 값을 쓴다
    if let Some(u) = gemini_usage(&json["usageMetadata"]) {
        acc.usage = Some(u);
    }
}

fn finish(acc: Acc) -> Result<Generation> {
    let tool_calls = acc.asm.finish()?;
    let reason = acc.reason.clone();
    let has_calls = !tool_calls.is_empty();
    let stop = match reason.as_deref() {
        Some("MALFORMED_FUNCTION_CALL" | "UNEXPECTED_TOOL_CALL") => {
            return Err(AppError::ai_provider(
                ErrorCode::AiProviderError,
                format!("모델이 올바르지 않은 도구 호출을 만들었음 ({})", reason.unwrap_or_default()),
                false,
            ))
        }
        Some("MAX_TOKENS") => StopKind::MaxTokens,
        Some("SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII" | "IMAGE_SAFETY") => StopKind::Filtered,
        // Gemini는 도구를 불러도 "STOP"이라 호출이 있으면 도구 호출로 본다
        Some("STOP") | None if has_calls => StopKind::ToolUse,
        Some("STOP") => StopKind::End,
        _ if has_calls => StopKind::ToolUse,
        _ => StopKind::Other,
    };
    Ok(Generation {
        content: acc.content,
        finish_reason: acc.reason,
        usage: acc.usage,
        stop,
        state: has_calls.then(|| ProviderState {
            provider: STATE_KEY.into(),
            raw: Value::Array(acc.parts),
        }),
        tool_calls,
    })
}

/// generateContent / streamGenerateContent?alt=sse 응답 공통 처리
pub(crate) async fn read_gemini_response(resp: Response, stream: bool, on_delta: &mut OnDelta<'_>) -> Result<Generation> {
    let resp = ensure_success(resp).await?;
    let mut acc = Acc::default();
    if !stream {
        let json: Value = resp.json().await.map_err(|e| parse_err("Gemini", e))?;
        absorb(&json, &mut acc, on_delta);
        return finish(acc);
    }
    read_sse(resp, |ev| {
        let json: Value = serde_json::from_str(ev.data).map_err(|e| parse_err("Gemini 스트림", e))?;
        absorb(&json, &mut acc, on_delta);
        Ok(true)
    })
    .await?;
    finish(acc)
}

// ---------------------------------------------------------------------------
// 임베딩
// ---------------------------------------------------------------------------

pub(crate) fn task_type(task: EmbedTask) -> &'static str {
    match task {
        EmbedTask::Document => "RETRIEVAL_DOCUMENT",
        EmbedTask::Query => "RETRIEVAL_QUERY",
    }
}

pub(crate) fn embed_body(model: &str, chunk: &[String], task: EmbedTask, dimensions: Option<u32>) -> Value {
    let requests: Vec<Value> = chunk
        .iter()
        .map(|t| {
            let mut r = json!({ "model": format!("models/{model}"), "content": { "parts": [{ "text": t }] }, "taskType": task_type(task) });
            if let Some(d) = dimensions {
                r["outputDimensionality"] = json!(d);
            }
            r
        })
        .collect();
    json!({ "requests": requests })
}

pub(crate) fn parse_embeddings(json: &Value, expect: usize, normalize: bool) -> Result<Vec<Vec<f32>>> {
    let rows = json["embeddings"].as_array().ok_or_else(|| parse_err("임베딩", "embeddings가 없음"))?;
    if rows.len() != expect {
        return Err(parse_err("임베딩", format!("{expect}개를 보냈는데 {}개가 옴", rows.len())));
    }
    rows.iter()
        .map(|r| {
            let mut v = floats(&r["values"]).ok_or_else(|| parse_err("임베딩", "values가 숫자 목록이 아님"))?;
            if normalize {
                l2_normalize(&mut v);
            }
            Ok(v)
        })
        .collect()
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
        let json: Value = ensure_success(resp).await?.json().await.map_err(|e| parse_err("모델 목록", e))?;
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

    async fn generate_with(&self, call: &ProviderCall<'_>, prompt: &PromptMessage, extras: &ChatExtras, on_delta: &mut OnDelta<'_>) -> Result<Generation> {
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
            .json(&gemini_body_with(prompt, extras))
            .send()
            .await
            .map_err(net_err)?;
        read_gemini_response(resp, stream, on_delta).await
    }

    async fn embed(&self, call: &ProviderCall<'_>, req: &EmbedRequest<'_>) -> Result<Embeddings> {
        let model = model_id(req.model);
        let mut out = Embeddings::default();
        for chunk in req.inputs.chunks(EMBED_BATCH) {
            let resp = call
                .http
                .post(call.url(&format!("models/{model}:batchEmbedContents")))
                .header("x-goog-api-key", call.credential)
                .json(&embed_body(model, chunk, req.task, req.dimensions))
                .send()
                .await
                .map_err(net_err)?;
            let json: Value = ensure_success(resp).await?.json().await.map_err(|e| parse_err("임베딩", e))?;
            // 차원을 줄이면 길이가 1이 아니라서 직접 정규화한다. 사용량은 응답에 없다 (앱이 추정)
            out.vectors.extend(parse_embeddings(&json, chunk.len(), req.dimensions.is_some())?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::testutil::{profile, serve, Reply};
    use crate::adapter::{ResponseSchema, ToolResult, ToolSpec};
    use crate::models::ai_service::{ChatOptions, PromptMessageItem};

    fn prompt(stream: bool) -> PromptMessage {
        PromptMessage {
            chat_options: ChatOptions {
                stream,
                max_tokens: Some(300),
                ..Default::default()
            },
            messages: vec![PromptMessageItem {
                role: PromptRole::User,
                content: "서울 날씨?".into(),
                token: 0,
                images: vec![],
            }],
        }
    }
    fn weather() -> ToolSpec {
        ToolSpec {
            name: "get_weather".into(),
            description: "날씨".into(),
            parameters: json!({ "type": "object", "properties": { "city": { "type": "string" }, "unit": { "type": ["string", "null"] } }, "required": ["city"], "additionalProperties": false }),
        }
    }
    fn call_of<'a>(http: &'a reqwest::Client, p: &'a crate::models::ai_service::ApiProfiles) -> ProviderCall<'a> {
        ProviderCall {
            http,
            profile: p,
            credential: "test-key",
        }
    }

    #[test]
    fn usage_counts_thinking_tokens_as_output() {
        assert_eq!(
            gemini_usage(&json!({ "promptTokenCount": 10, "candidatesTokenCount": 5, "thoughtsTokenCount": 40 })),
            Some((10, 45))
        );
        assert_eq!(gemini_usage(&json!({ "promptTokenCount": 10 })), Some((10, 0)));
        assert_eq!(gemini_usage(&json!({})), None);
    }

    #[test]
    fn declarations_tool_config_and_response_schema() {
        let no_args = ToolSpec {
            name: "now".into(),
            description: "지금".into(),
            parameters: json!({ "type": "object", "properties": {} }),
        };
        let b = gemini_body_with(
            &prompt(false),
            &ChatExtras {
                tools: vec![weather(), no_args],
                tool_choice: ToolChoice::Named("now".into()),
                ..Default::default()
            },
        );
        let decls = &b["tools"][0]["functionDeclarations"];
        assert_eq!(decls[0]["parameters"]["type"], "OBJECT");
        assert!(decls[0]["parameters"].get("additionalProperties").is_none());
        assert_eq!(decls[0]["parameters"]["properties"]["unit"], json!({ "type": "STRING", "nullable": true }));
        assert!(decls[1].get("parameters").is_none(), "인자 없는 함수는 parameters를 뺌");
        assert_eq!(
            b["toolConfig"]["functionCallingConfig"],
            json!({ "mode": "ANY", "allowedFunctionNames": ["now"] })
        );
        for (c, want) in [(ToolChoice::None, json!({ "mode": "NONE" })), (ToolChoice::Required, json!({ "mode": "ANY" }))] {
            assert_eq!(
                gemini_body_with(
                    &prompt(false),
                    &ChatExtras {
                        tools: vec![weather()],
                        tool_choice: c,
                        ..Default::default()
                    }
                )["toolConfig"]["functionCallingConfig"],
                want
            );
        }
        assert!(gemini_body_with(
            &prompt(false),
            &ChatExtras {
                tools: vec![weather()],
                ..Default::default()
            }
        )
        .get("toolConfig")
        .is_none());

        let s = json!({ "type": "object", "properties": { "a": { "type": "string" } }, "required": ["a"] });
        let b = gemini_body_with(
            &prompt(false),
            &ChatExtras {
                response_schema: Some(ResponseSchema {
                    name: "out".into(),
                    description: None,
                    schema: s,
                }),
                ..Default::default()
            },
        );
        assert_eq!(b["generationConfig"]["responseMimeType"], "application/json");
        assert_eq!(b["generationConfig"]["responseSchema"]["properties"]["a"]["type"], "STRING");
        assert_eq!(b["generationConfig"]["maxOutputTokens"], 300, "기존 설정은 그대로");
        // 평범한 요청에는 도구·구성이 없다
        let plain = gemini_body(&prompt(false));
        assert!(plain.get("tools").is_none() && plain.get("toolConfig").is_none());
    }

    #[test]
    fn tail_returns_original_parts_with_signatures_and_wraps_responses() {
        let original = json!([
            { "text": "찾아볼게요", "thoughtSignature": "sig-text" },
            { "functionCall": { "id": "fc-1", "name": "get_weather", "args": { "city": "서울" } }, "thoughtSignature": "sig-call" },
            { "functionCall": { "name": "get_weather", "args": { "city": "부산" } } }
        ]);
        let calls = vec![
            ToolCall {
                id: "fc-1".into(),
                name: "get_weather".into(),
                arguments: json!({ "city": "서울" }),
                raw_arguments: String::new(),
                parse_error: None,
            },
            ToolCall {
                id: "call_x_2".into(),
                name: "get_weather".into(),
                arguments: json!({ "city": "부산" }),
                raw_arguments: String::new(),
                parse_error: None,
            },
        ];
        let tail = vec![
            ToolTurn::Assistant {
                text: "찾아볼게요".into(),
                calls: calls.clone(),
                state: Some(ProviderState {
                    provider: "gemini".into(),
                    raw: original.clone(),
                }),
            },
            // 결과를 거꾸로 줘도 호출 순서대로 들어간다
            ToolTurn::Results {
                results: vec![
                    ToolResult {
                        call_id: "call_x_2".into(),
                        name: "get_weather".into(),
                        content: "흐림".into(),
                        is_error: true,
                    },
                    ToolResult {
                        call_id: "fc-1".into(),
                        name: "get_weather".into(),
                        content: "맑음".into(),
                        is_error: false,
                    },
                ],
                note: Some("남은 단계 1".into()),
            },
        ];
        let c = tail_contents(&tail);
        assert_eq!(c[0], json!({ "role": "model", "parts": original }), "서명이 든 원본 그대로");
        let parts = c[1]["parts"].as_array().unwrap();
        assert_eq!(
            parts[0],
            json!({ "functionResponse": { "name": "get_weather", "id": "fc-1", "response": { "result": "맑음" } } }),
            "공급자 id는 되돌려 보냄, 응답은 객체"
        );
        assert_eq!(
            parts[1],
            json!({ "functionResponse": { "name": "get_weather", "response": { "error": "흐림" } } }),
            "앱이 만든 id는 보내지 않음"
        );
        assert_eq!(parts[2], json!({ "text": "남은 단계 1" }));
        // 다른 공급자가 만든 상태는 버리고 글과 호출로 다시 만든다
        let foreign = vec![ToolTurn::Assistant {
            text: "x".into(),
            calls: calls.clone(),
            state: Some(ProviderState {
                provider: "claude".into(),
                raw: json!([1]),
            }),
        }];
        assert_eq!(tail_contents(&foreign)[0]["parts"][1]["functionCall"]["args"], json!({ "city": "서울" }));
    }

    #[tokio::test]
    async fn non_streaming_function_calls_keep_parts_and_signatures() {
        let srv = serve(vec![Reply::ok(json!({
            "candidates": [{ "finishReason": "STOP", "content": { "role": "model", "parts": [
                { "text": "찾아볼게요" },
                { "functionCall": { "name": "get_weather", "args": { "city": "서울" } }, "thoughtSignature": "SIG" }
            ] } }],
            "usageMetadata": { "promptTokenCount": 12, "candidatesTokenCount": 6, "thoughtsTokenCount": 30 }
        }))])
        .await;
        let (http, p) = profile(&srv.base, "gemini", "models/gemini-2.5-flash");
        let g = GeminiClient
            .generate_with(
                &call_of(&http, &p),
                &prompt(false),
                &ChatExtras {
                    tools: vec![weather()],
                    ..Default::default()
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        assert_eq!((g.content.as_str(), g.stop, g.usage), ("찾아볼게요", StopKind::ToolUse, Some((12, 36))));
        assert!(g.tool_calls[0].id.starts_with("call_"), "id가 없으면 앱이 만든다");
        assert_eq!(g.tool_calls[0].arguments, json!({ "city": "서울" }));
        let st = g.state.unwrap();
        assert_eq!(st.provider, "gemini");
        assert_eq!(st.raw[1]["thoughtSignature"], "SIG");
        let seen = srv.last();
        assert_eq!(seen.path, "/models/gemini-2.5-flash:generateContent");
        assert_eq!(seen.headers["x-goog-api-key"], "test-key");
    }

    #[tokio::test]
    async fn streaming_text_and_calls_with_a_late_signature_part() {
        let chunk = |v: Value| format!("data: {v}\n\n").into_bytes();
        let srv = serve(vec![Reply::Sse(vec![
            chunk(json!({ "candidates": [{ "content": { "parts": [{ "text": "찾아" }] } }] })),
            chunk(json!({ "candidates": [{ "content": { "parts": [{ "text": "볼게요" }] } }] })),
            chunk(json!({ "candidates": [{ "content": { "parts": [{ "functionCall": { "name": "get_weather", "args": { "city": "서울" } } }] } }] })),
            // 서명은 빈 글 파트로 따로 오기도 한다
            chunk(json!({ "candidates": [{ "content": { "parts": [{ "text": "", "thoughtSignature": "LATE" }] }, "finishReason": "STOP" }], "usageMetadata": { "promptTokenCount": 9, "candidatesTokenCount": 4 } })),
        ])])
        .await;
        let (http, p) = profile(&srv.base, "gemini", "gemini-2.5-flash");
        let mut text = String::new();
        let g = GeminiClient
            .generate_with(
                &call_of(&http, &p),
                &prompt(true),
                &ChatExtras {
                    tools: vec![weather()],
                    ..Default::default()
                },
                &mut |d| text.push_str(d),
            )
            .await
            .unwrap();
        assert_eq!(text, "찾아볼게요");
        assert_eq!((g.stop, g.usage), (StopKind::ToolUse, Some((9, 4))));
        let raw = g.state.unwrap().raw;
        assert_eq!(raw[0], json!({ "text": "찾아볼게요" }), "인접한 순수 글은 합침");
        assert_eq!(raw.as_array().unwrap().len(), 3);
        assert_eq!(raw[2]["thoughtSignature"], "LATE", "서명 파트는 그대로 보존");
        assert_eq!(srv.last().path, "/models/gemini-2.5-flash:streamGenerateContent?alt=sse");
    }

    #[tokio::test]
    async fn malformed_function_call_and_truncation() {
        let srv = serve(vec![
            Reply::ok(json!({ "candidates": [{ "finishReason": "MALFORMED_FUNCTION_CALL" }] })),
            Reply::ok(json!({ "candidates": [{ "finishReason": "MAX_TOKENS", "content": { "parts": [{ "text": "{\"a\": " }] } }] })),
        ])
        .await;
        let (http, p) = profile(&srv.base, "gemini", "gemini-2.5-flash");
        let e = GeminiClient
            .generate_with(&call_of(&http, &p), &prompt(false), &ChatExtras::default(), &mut |_| {})
            .await
            .unwrap_err();
        assert!(e.to_string().contains("MALFORMED_FUNCTION_CALL"));
        let g = GeminiClient
            .generate_with(&call_of(&http, &p), &prompt(false), &ChatExtras::default(), &mut |_| {})
            .await
            .unwrap();
        assert_eq!(g.stop, StopKind::MaxTokens);
    }

    #[tokio::test]
    async fn embeddings_request_shape_and_normalization() {
        let srv = serve(vec![Reply::ok(json!({ "embeddings": [{ "values": [3.0, 4.0] }, { "values": [0.0, 2.0] }] }))]).await;
        let (http, p) = profile(&srv.base, "gemini", "gemini-2.5-flash");
        let inputs = vec!["a".to_string(), "b".to_string()];
        let e = GeminiClient
            .embed(
                &call_of(&http, &p),
                &EmbedRequest {
                    model: "models/gemini-embedding-001",
                    inputs: &inputs,
                    task: EmbedTask::Query,
                    dimensions: Some(2),
                },
            )
            .await
            .unwrap();
        assert_eq!(e.vectors[0], vec![0.6, 0.8], "차원을 줄이면 길이 1로 정규화");
        assert_eq!(e.vectors[1], vec![0.0, 1.0]);
        assert_eq!(e.tokens, None, "사용량은 앱이 추정");
        let seen = srv.last();
        assert_eq!(seen.path, "/models/gemini-embedding-001:batchEmbedContents");
        let r = &seen.json()["requests"][0];
        assert_eq!(
            (
                r["model"].clone(),
                r["taskType"].clone(),
                r["outputDimensionality"].clone(),
                r["content"]["parts"][0]["text"].clone()
            ),
            (json!("models/gemini-embedding-001"), json!("RETRIEVAL_QUERY"), json!(2), json!("a"))
        );
        // 차원을 안 줄이면 정규화하지 않는다
        assert_eq!(
            parse_embeddings(&json!({ "embeddings": [{ "values": [3.0, 4.0] }] }), 1, false).unwrap()[0],
            vec![3.0, 4.0]
        );
        assert!(parse_embeddings(&json!({ "embeddings": [] }), 1, false).is_err());
    }
}
