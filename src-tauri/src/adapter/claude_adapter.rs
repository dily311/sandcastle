//! Anthropic Messages API (`/messages`)
//!
//! 도구 호출은 `tools`/`tool_use`/`tool_result`. 구조화 출력은 Anthropic의 베타 기능 대신 **도구 하나를 강제**해서(`tool_choice: tool`)
//! 그 입력을 결과로 쓴다 — 정식 기능이고 도구를 지원하는 모든 모델에서 된다. (thinking을 켜면 강제 도구를 못 쓰고 thinking 블록을
//! 되돌려 보내야 하는데, 앱은 thinking을 켜지 않는다.) 임베딩 API는 없다.
use async_trait::async_trait;
use reqwest::RequestBuilder;
use serde_json::{json, Map, Value};

use crate::{
    adapter::{
        ensure_success, load_image, net_err, parse_err, split_system, sse::read_sse, tools::CallAssembler, AiProvider, ChatExtras, Generation, OnDelta,
        ProviderCall, StopKind, ToolChoice, ToolTurn,
    },
    error::{AppError, ErrorCode, Result},
    models::ai_service::PromptMessage,
};

const ANTHROPIC_VERSION: &str = "2023-06-01";
/// max_tokens는 Anthropic에서 필수
const DEFAULT_MAX_TOKENS: u32 = 1024;

pub struct ClaudeClient;

fn stop_kind(reason: Option<&str>, has_calls: bool) -> StopKind {
    match reason {
        Some("tool_use") => StopKind::ToolUse,
        Some("end_turn" | "stop_sequence") if has_calls => StopKind::ToolUse,
        Some("end_turn" | "stop_sequence") => StopKind::End,
        Some("max_tokens") => StopKind::MaxTokens,
        Some("refusal") => StopKind::Filtered,
        _ if has_calls => StopKind::ToolUse,
        _ => StopKind::Other,
    }
}

/// 도구 루프 꼬리 → messages. assistant는 글 + `tool_use` 블록, 결과는 **user** 메시지의 `tool_result` 블록들(글보다 앞) + `note`
fn tail_messages(tail: &[ToolTurn]) -> Vec<Value> {
    let mut out = vec![];
    for turn in tail {
        match turn {
            ToolTurn::Assistant { text, calls, .. } => {
                let mut blocks: Vec<Value> = vec![];
                if !text.trim().is_empty() {
                    blocks.push(json!({ "type": "text", "text": text }));
                }
                for c in calls {
                    let input = if c.parse_error.is_some() { json!({}) } else { c.arguments.clone() };
                    blocks.push(json!({ "type": "tool_use", "id": c.id, "name": c.name, "input": input }));
                }
                if !blocks.is_empty() {
                    out.push(json!({ "role": "assistant", "content": blocks }));
                }
            }
            ToolTurn::Results { results, note } => {
                let mut blocks: Vec<Value> = results
                    .iter()
                    .map(|r| {
                        let mut b = json!({ "type": "tool_result", "tool_use_id": r.call_id, "content": r.content });
                        if r.is_error {
                            b["is_error"] = json!(true);
                        }
                        b
                    })
                    .collect();
                if let Some(n) = note.as_deref().filter(|n| !n.trim().is_empty()) {
                    blocks.push(json!({ "type": "text", "text": n }));
                }
                if !blocks.is_empty() {
                    out.push(json!({ "role": "user", "content": blocks }));
                }
            }
        }
    }
    out
}

impl ClaudeClient {
    fn auth(req: RequestBuilder, call: &ProviderCall<'_>) -> RequestBuilder {
        req.header("x-api-key", call.credential).header("anthropic-version", ANTHROPIC_VERSION)
    }

    #[cfg(test)]
    pub(crate) fn body(model: &str, prompt: &PromptMessage) -> Value {
        Self::body_with(model, prompt, &ChatExtras::default())
    }

    pub(crate) fn body_with(model: &str, prompt: &PromptMessage, extras: &ChatExtras) -> Value {
        let (system, turns) = split_system(prompt);
        let opts = &prompt.chat_options;
        let mut body = Map::new();
        body.insert("model".into(), json!(model));
        body.insert("max_tokens".into(), json!(opts.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)));
        body.insert("stream".into(), json!(opts.stream));
        if let Some(system) = system {
            body.insert("system".into(), json!(system));
        }
        let mut messages: Vec<Value> = turns
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
            .collect();
        messages.extend(tail_messages(&extras.tail));
        body.insert("messages".into(), Value::Array(messages));
        if let Some(v) = opts.temperature {
            body.insert("temperature".into(), json!(v));
        }
        if let Some(v) = opts.top_p {
            body.insert("top_p".into(), json!(v));
        }
        if let Some(v) = opts.top_k {
            body.insert("top_k".into(), json!(v));
        }
        if let Some(level) = &opts.reasoning_effort {
            body.insert("output_config".into(), json!({ "effort": level }));
        }

        if !extras.tools.is_empty() {
            let tools: Vec<Value> = extras
                .tools
                .iter()
                .map(|t| json!({ "name": t.name, "description": t.description, "input_schema": t.parameters }))
                .collect();
            body.insert("tools".into(), Value::Array(tools));
            match &extras.tool_choice {
                ToolChoice::Auto => {}
                ToolChoice::None => {
                    body.insert("tool_choice".into(), json!({ "type": "none" }));
                }
                ToolChoice::Required => {
                    body.insert("tool_choice".into(), json!({ "type": "any" }));
                }
                ToolChoice::Named(n) => {
                    body.insert("tool_choice".into(), json!({ "type": "tool", "name": n }));
                }
            }
        }
        if let Some(rs) = &extras.response_schema {
            // 구조화 출력 = 이 도구 하나를 강제
            let mut tool = json!({ "name": rs.name, "input_schema": rs.schema });
            tool["description"] = json!(rs.description.clone().unwrap_or_else(|| "응답을 이 형식으로 낸다".into()));
            body.insert("tools".into(), json!([tool]));
            body.insert("tool_choice".into(), json!({ "type": "tool", "name": rs.name }));
        }
        Value::Object(body)
    }

    /// 강제한 구조화 도구가 있으면 그 입력을 응답 글로 (도구 호출로는 숨긴다)
    fn finish(mut g: Generation, structured: Option<&str>) -> Generation {
        if let Some(name) = structured {
            if let Some(c) = g.tool_calls.iter().find(|c| c.name == name) {
                g.content = c.raw_arguments.clone();
            }
            g.tool_calls.clear();
            if g.stop == StopKind::ToolUse {
                g.stop = StopKind::End;
            }
        }
        g
    }
}

#[async_trait]
impl AiProvider for ClaudeClient {
    async fn list_models(&self, call: &ProviderCall<'_>) -> Result<Vec<String>> {
        let resp = Self::auth(call.http.get(call.url("models")), call).send().await.map_err(net_err)?;
        let json: Value = ensure_success(resp).await?.json().await.map_err(|e| parse_err("모델 목록", e))?;
        Ok(json["data"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|m| m["id"].as_str().map(str::to_string)).collect())
            .unwrap_or_default())
    }

    async fn generate_with(&self, call: &ProviderCall<'_>, prompt: &PromptMessage, extras: &ChatExtras, on_delta: &mut OnDelta<'_>) -> Result<Generation> {
        let structured = extras.response_schema.as_ref().map(|r| r.name.as_str());
        let resp = Self::auth(call.http.post(call.url("messages")), call)
            .json(&Self::body_with(&call.profile.model, prompt, extras))
            .send()
            .await
            .map_err(net_err)?;
        let resp = ensure_success(resp).await?;

        if !prompt.chat_options.stream {
            let json: Value = resp.json().await.map_err(|e| parse_err("Claude", e))?;
            let blocks = json["content"].as_array().cloned().unwrap_or_default();
            let content = blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect::<String>();
            let mut asm = CallAssembler::default();
            for (i, b) in blocks.iter().filter(|b| b["type"] == "tool_use").enumerate() {
                asm.start(i, b["id"].as_str(), b["name"].as_str());
                asm.push_args(i, &b["input"].to_string());
            }
            let tool_calls = asm.finish()?;
            let reason = json["stop_reason"].as_str();
            return Ok(Self::finish(
                Generation {
                    content,
                    finish_reason: reason.map(str::to_string),
                    usage: super::usage_from(&json["usage"], "input_tokens", "output_tokens"),
                    stop: stop_kind(reason, !tool_calls.is_empty()),
                    tool_calls,
                    state: None,
                },
                structured,
            ));
        }

        let mut out = Generation::default();
        let mut asm = CallAssembler::default();
        read_sse(resp, |ev| {
            let json: Value = serde_json::from_str(ev.data).map_err(|e| parse_err("Claude 스트림", e))?;
            match json["type"].as_str().unwrap_or(ev.event.unwrap_or_default()) {
                "content_block_start" => {
                    let block = &json["content_block"];
                    if block["type"] == "tool_use" {
                        let index = json["index"].as_u64().unwrap_or(0) as usize;
                        asm.start(index, block["id"].as_str(), block["name"].as_str());
                    }
                }
                "content_block_delta" => {
                    let delta = &json["delta"];
                    match delta["type"].as_str() {
                        // 도구 인자 JSON 조각 (빈 글자로 오기도 한다)
                        Some("input_json_delta") => {
                            if let Some(part) = delta["partial_json"].as_str() {
                                asm.push_args(json["index"].as_u64().unwrap_or(0) as usize, part);
                            }
                        }
                        // thinking_delta·signature_delta 등은 글이 아니다
                        Some("thinking_delta" | "signature_delta" | "citations_delta") => {}
                        _ => {
                            if let Some(text) = delta["text"].as_str() {
                                on_delta(text);
                                out.content.push_str(text);
                            }
                        }
                    }
                }
                "message_start" => {
                    // 입력 토큰은 시작 때, 출력 토큰은 message_delta에서 온다
                    if let Some((i, o)) = super::usage_from(&json["message"]["usage"], "input_tokens", "output_tokens") {
                        out.usage = Some((i, o));
                    }
                }
                "message_delta" => {
                    if let Some(reason) = json["delta"]["stop_reason"].as_str() {
                        out.finish_reason = Some(reason.to_string());
                    }
                    if let Some(o) = json["usage"]["output_tokens"].as_u64() {
                        let input = out.usage.map_or(0, |(i, _)| i);
                        out.usage = Some((input, o));
                    }
                }
                "message_stop" => return Ok(false),
                "error" => {
                    let message = json["error"]["message"].as_str().unwrap_or("스트림 오류");
                    let overloaded = json["error"]["type"] == "overloaded_error";
                    return Err(AppError::ai_provider(ErrorCode::AiProviderError, message.to_string(), overloaded));
                }
                _ => {}
            }
            Ok(true)
        })
        .await?;
        out.tool_calls = asm.finish()?;
        out.stop = stop_kind(out.finish_reason.as_deref(), !out.tool_calls.is_empty());
        Ok(Self::finish(out, structured))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::testutil::{profile, serve, Reply};
    use crate::adapter::{EmbedRequest, EmbedTask, ResponseSchema, ToolCall, ToolResult, ToolSpec};
    use crate::models::ai_service::{ChatOptions, PromptMessageItem, PromptRole};

    fn prompt(stream: bool) -> PromptMessage {
        PromptMessage {
            chat_options: ChatOptions {
                stream,
                max_tokens: Some(300),
                ..Default::default()
            },
            messages: vec![
                PromptMessageItem {
                    role: PromptRole::System,
                    content: "규칙".into(),
                    token: 0,
                    images: vec![],
                    source: None,
                },
                PromptMessageItem {
                    role: PromptRole::User,
                    content: "서울 날씨?".into(),
                    token: 0,
                    images: vec![],
                    source: None,
                },
            ],
        }
    }
    fn weather() -> ToolSpec {
        ToolSpec {
            name: "get_weather".into(),
            description: "날씨".into(),
            parameters: json!({ "type": "object", "properties": { "city": { "type": "string" } }, "required": ["city"] }),
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
    fn reasoning_effort_goes_to_output_config() {
        let mut p = prompt(false);
        assert!(ClaudeClient::body("claude-opus-5-5", &p).get("output_config").is_none());
        p.chat_options.reasoning_effort = Some("high".into());
        assert_eq!(ClaudeClient::body("claude-opus-5-5", &p)["output_config"]["effort"], "high");
    }

    #[test]
    fn tools_choice_and_tail_blocks() {
        let extras = ChatExtras {
            tools: vec![weather()],
            tool_choice: ToolChoice::Required,
            tail: vec![
                ToolTurn::Assistant {
                    text: "찾아볼게요".into(),
                    calls: vec![ToolCall {
                        id: "tu1".into(),
                        name: "get_weather".into(),
                        arguments: json!({ "city": "서울" }),
                        raw_arguments: String::new(),
                        parse_error: None,
                    }],
                    state: None,
                },
                ToolTurn::Results {
                    results: vec![ToolResult {
                        call_id: "tu1".into(),
                        name: "get_weather".into(),
                        content: "맑음".into(),
                        is_error: true,
                    }],
                    note: Some("남은 단계 1".into()),
                },
            ],
            ..Default::default()
        };
        let b = ClaudeClient::body_with("m", &prompt(false), &extras);
        assert_eq!(
            b["tools"][0],
            json!({ "name": "get_weather", "description": "날씨", "input_schema": weather().parameters })
        );
        assert_eq!(b["tool_choice"], json!({ "type": "any" }));
        let m = b["messages"].as_array().unwrap();
        assert_eq!(m[1]["role"], "assistant");
        assert_eq!(
            m[1]["content"][1],
            json!({ "type": "tool_use", "id": "tu1", "name": "get_weather", "input": { "city": "서울" } })
        );
        assert_eq!(m[2]["role"], "user");
        assert_eq!(
            m[2]["content"][0],
            json!({ "type": "tool_result", "tool_use_id": "tu1", "content": "맑음", "is_error": true }),
            "결과 블록이 글보다 앞"
        );
        assert_eq!(m[2]["content"][1], json!({ "type": "text", "text": "남은 단계 1" }));
        for (c, want) in [
            (ToolChoice::None, json!({ "type": "none" })),
            (ToolChoice::Named("get_weather".into()), json!({ "type": "tool", "name": "get_weather" })),
        ] {
            assert_eq!(
                ClaudeClient::body_with(
                    "m",
                    &prompt(false),
                    &ChatExtras {
                        tools: vec![weather()],
                        tool_choice: c,
                        ..Default::default()
                    }
                )["tool_choice"],
                want
            );
        }
        // 기존 요청은 그대로
        let plain = ClaudeClient::body("m", &prompt(true));
        assert!(plain.get("tools").is_none() && plain.get("tool_choice").is_none());
    }

    #[test]
    fn structured_output_forces_one_tool() {
        let schema = json!({ "type": "object", "properties": { "a": { "type": "string" } }, "required": ["a"] });
        let b = ClaudeClient::body_with(
            "m",
            &prompt(false),
            &ChatExtras {
                response_schema: Some(ResponseSchema {
                    name: "out".into(),
                    description: None,
                    schema: schema.clone(),
                }),
                ..Default::default()
            },
        );
        assert_eq!(b["tools"][0]["name"], "out");
        assert_eq!(b["tools"][0]["input_schema"], schema);
        assert_eq!(b["tool_choice"], json!({ "type": "tool", "name": "out" }));
    }

    #[tokio::test]
    async fn non_streaming_tool_use_and_structured_result() {
        let srv = serve(vec![
            Reply::ok(json!({
                "content": [{ "type": "text", "text": "찾아볼게요" }, { "type": "tool_use", "id": "tu1", "name": "get_weather", "input": { "city": "서울" } }],
                "stop_reason": "tool_use", "usage": { "input_tokens": 20, "output_tokens": 8 }
            })),
            Reply::ok(json!({
                "content": [{ "type": "tool_use", "id": "tu2", "name": "out", "input": { "a": "값" } }],
                "stop_reason": "tool_use", "usage": { "input_tokens": 10, "output_tokens": 4 }
            })),
        ])
        .await;
        let (http, p) = profile(&format!("{}/v1", srv.base), "claude", "claude-sonnet-5-5");
        let g = ClaudeClient
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
        assert_eq!((g.content.as_str(), g.stop, g.usage), ("찾아볼게요", StopKind::ToolUse, Some((20, 8))));
        assert_eq!(
            (g.tool_calls[0].id.as_str(), g.tool_calls[0].arguments.clone()),
            ("tu1", json!({ "city": "서울" }))
        );
        let seen = srv.requests();
        assert_eq!(seen[0].path, "/v1/messages");
        assert_eq!(
            (seen[0].headers["x-api-key"].as_str(), seen[0].headers["anthropic-version"].as_str()),
            ("test-key", "2023-06-01")
        );

        let extras = ChatExtras {
            response_schema: Some(ResponseSchema {
                name: "out".into(),
                description: None,
                schema: json!({ "type": "object", "properties": { "a": { "type": "string" } } }),
            }),
            ..Default::default()
        };
        let g = ClaudeClient
            .generate_with(&call_of(&http, &p), &prompt(false), &extras, &mut |_| {})
            .await
            .unwrap();
        assert!(g.tool_calls.is_empty(), "구조화용 도구는 호출로 보이지 않음");
        assert_eq!(serde_json::from_str::<Value>(&g.content).unwrap(), json!({ "a": "값" }));
        assert_eq!(g.stop, StopKind::End);
    }

    #[tokio::test]
    async fn streaming_tool_use_with_partial_json_and_empty_input() {
        let ev = |name: &str, v: Value| format!("event: {name}\ndata: {v}\n\n").into_bytes();
        let srv = serve(vec![Reply::Sse(vec![
            ev(
                "message_start",
                json!({ "type": "message_start", "message": { "usage": { "input_tokens": 25, "output_tokens": 1 } } }),
            ),
            ev(
                "content_block_start",
                json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "text", "text": "" } }),
            ),
            ev(
                "content_block_delta",
                json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "text_delta", "text": "찾아" } }),
            ),
            ev(
                "content_block_delta",
                json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "text_delta", "text": "볼게요" } }),
            ),
            ev("content_block_stop", json!({ "type": "content_block_stop", "index": 0 })),
            ev(
                "content_block_start",
                json!({ "type": "content_block_start", "index": 1, "content_block": { "type": "tool_use", "id": "tu1", "name": "get_weather", "input": {} } }),
            ),
            ev(
                "content_block_delta",
                json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "input_json_delta", "partial_json": "" } }),
            ),
            ev(
                "content_block_delta",
                json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "input_json_delta", "partial_json": "{\"city\": \"서" } }),
            ),
            ev(
                "content_block_delta",
                json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "input_json_delta", "partial_json": "울\"}" } }),
            ),
            ev("content_block_stop", json!({ "type": "content_block_stop", "index": 1 })),
            ev(
                "content_block_start",
                json!({ "type": "content_block_start", "index": 2, "content_block": { "type": "tool_use", "id": "tu2", "name": "now", "input": {} } }),
            ),
            ev(
                "content_block_delta",
                json!({ "type": "content_block_delta", "index": 2, "delta": { "type": "input_json_delta", "partial_json": "" } }),
            ),
            ev("content_block_stop", json!({ "type": "content_block_stop", "index": 2 })),
            ev(
                "message_delta",
                json!({ "type": "message_delta", "delta": { "stop_reason": "tool_use" }, "usage": { "output_tokens": 33 } }),
            ),
            ev("message_stop", json!({ "type": "message_stop" })),
        ])])
        .await;
        let (http, p) = profile(&srv.base, "claude", "claude-sonnet-5-5");
        let mut text = String::new();
        let g = ClaudeClient
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
        assert_eq!(g.stop, StopKind::ToolUse);
        assert_eq!(g.usage, Some((25, 33)));
        let calls: Vec<_> = g.tool_calls.iter().map(|c| (c.id.as_str(), c.name.as_str(), c.arguments.clone())).collect();
        assert_eq!(
            calls,
            vec![("tu1", "get_weather", json!({ "city": "서울" })), ("tu2", "now", json!({}))],
            "인자 없는 도구는 빈 객체"
        );
    }

    #[tokio::test]
    async fn claude_has_no_embeddings() {
        let (http, p) = profile("http://127.0.0.1:1", "claude", "m");
        let e = ClaudeClient
            .embed(
                &call_of(&http, &p),
                &EmbedRequest {
                    model: "x",
                    inputs: &["a".to_string()],
                    task: EmbedTask::Document,
                    dimensions: None,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(e.to_response().code, ErrorCode::AiFeatureUnsupported);
    }
}
