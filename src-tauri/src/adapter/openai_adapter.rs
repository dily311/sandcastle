//! OpenAI 호환 Chat Completions (`/chat/completions`) + 임베딩 (`/embeddings`)
use async_trait::async_trait;
use serde_json::{json, Map, Value};

use crate::{
    adapter::{
        capabilities::is_official_openai, ensure_success, net_err, parse_err, schema::is_strict_ready, sse::read_sse, tools::CallAssembler, AiProvider,
        ChatExtras, EmbedRequest, EmbedTask, Embeddings, Generation, OnDelta, ProviderCall, StopKind, ToolChoice, ToolTurn,
    },
    error::Result,
    models::ai_service::PromptMessage,
};

pub struct OpenAIClient;

/// 임베딩 요청 하나에 담는 글 수 (공급자 한도는 2048개·약 30만 토큰이라 여유를 둠)
const EMBED_BATCH: usize = 100;

/// 도구 호출의 인자를 글로 (깨진 인자는 `{}` — 깨진 JSON을 되돌려 보내면 공급자가 400을 낸다)
fn args_string(c: &crate::adapter::ToolCall) -> String {
    if c.parse_error.is_some() {
        "{}".into()
    } else {
        serde_json::to_string(&c.arguments).unwrap_or_else(|_| "{}".into())
    }
}

/// 도구 루프 꼬리 → chat 메시지. 호출은 assistant의 `tool_calls`, 결과는 호출마다 `role: tool` 하나, `note`는 이어서 user 글
fn tail_messages(tail: &[ToolTurn]) -> Vec<Value> {
    let mut out = vec![];
    for turn in tail {
        match turn {
            ToolTurn::Assistant { text, calls, .. } => {
                let mut m = json!({ "role": "assistant", "content": if text.is_empty() && !calls.is_empty() { Value::Null } else { json!(text) } });
                if !calls.is_empty() {
                    m["tool_calls"] = Value::Array(
                        calls
                            .iter()
                            .map(|c| json!({ "id": c.id, "type": "function", "function": { "name": c.name, "arguments": args_string(c) } }))
                            .collect(),
                    );
                }
                out.push(m);
            }
            ToolTurn::Results { results, note } => {
                for r in results {
                    let content = if r.is_error { format!("[오류] {}", r.content) } else { r.content.clone() };
                    out.push(json!({ "role": "tool", "tool_call_id": r.call_id, "content": content }));
                }
                if let Some(n) = note.as_deref().filter(|n| !n.trim().is_empty()) {
                    out.push(json!({ "role": "user", "content": n }));
                }
            }
        }
    }
    out
}

/// finish_reason → 정규화 (호환 서버는 도구를 불렀는데도 "stop"을 주기도 한다)
fn stop_kind(reason: Option<&str>, has_calls: bool) -> StopKind {
    match reason {
        Some("tool_calls" | "function_call") => StopKind::ToolUse,
        Some("length") => StopKind::MaxTokens,
        Some("content_filter") => StopKind::Filtered,
        Some("stop") if has_calls => StopKind::ToolUse,
        Some("stop") => StopKind::End,
        _ if has_calls => StopKind::ToolUse,
        _ => StopKind::Other,
    }
}

impl OpenAIClient {
    #[cfg(test)]
    pub(crate) fn body(model: &str, prompt: &PromptMessage) -> Value {
        Self::body_with(model, prompt, &ChatExtras::default(), false)
    }

    /// `official`: 공식 OpenAI 주소 — `strict`와 `stream_options`는 공식에서만 보낸다 (호환 서버는 모르는 필드에 400을 내기도 함)
    pub(crate) fn body_with(model: &str, prompt: &PromptMessage, extras: &ChatExtras, official: bool) -> Value {
        let mut messages: Vec<Value> = prompt
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
        messages.extend(tail_messages(&extras.tail));
        let opts = &prompt.chat_options;
        let mut body = Map::new();
        body.insert("model".into(), json!(model));
        body.insert("messages".into(), Value::Array(messages));
        body.insert("stream".into(), json!(opts.stream));
        if official && opts.stream {
            // 스트림 끝에 실제 사용량을 받는다 (없으면 앱이 추정)
            body.insert("stream_options".into(), json!({ "include_usage": true }));
        }
        if let Some(v) = opts.temperature {
            body.insert("temperature".into(), json!(v));
        }
        if let Some(v) = opts.max_tokens {
            body.insert("max_tokens".into(), json!(v));
        }
        if let Some(v) = opts.top_p {
            body.insert("top_p".into(), json!(v));
        }
        if let Some(level) = &opts.reasoning_effort {
            body.insert("reasoning_effort".into(), json!(level));
        }

        if !extras.tools.is_empty() {
            let mut any_strict = false;
            let tools: Vec<Value> = extras
                .tools
                .iter()
                .map(|t| {
                    let strict = official && is_strict_ready(&t.parameters);
                    any_strict |= strict;
                    let mut f = json!({ "name": t.name, "description": t.description, "parameters": t.parameters });
                    if strict {
                        f["strict"] = json!(true);
                    }
                    json!({ "type": "function", "function": f })
                })
                .collect();
            body.insert("tools".into(), Value::Array(tools));
            match &extras.tool_choice {
                ToolChoice::Auto => {}
                ToolChoice::None => {
                    body.insert("tool_choice".into(), json!("none"));
                }
                ToolChoice::Required => {
                    body.insert("tool_choice".into(), json!("required"));
                }
                ToolChoice::Named(n) => {
                    body.insert("tool_choice".into(), json!({ "type": "function", "function": { "name": n } }));
                }
            }
            if any_strict {
                // strict 도구는 병렬 호출과 같이 쓸 수 없다
                body.insert("parallel_tool_calls".into(), json!(false));
            }
        }
        if let Some(rs) = &extras.response_schema {
            let mut js = json!({ "name": rs.name, "schema": rs.schema, "strict": is_strict_ready(&rs.schema) });
            if let Some(d) = &rs.description {
                js["description"] = json!(d);
            }
            body.insert("response_format".into(), json!({ "type": "json_schema", "json_schema": js }));
        }
        Value::Object(body)
    }

    /// 비스트리밍 응답 → Generation
    fn parse_response(json: &Value) -> Result<Generation> {
        let choice = &json["choices"][0];
        let msg = &choice["message"];
        let mut asm = CallAssembler::default();
        if let Some(calls) = msg["tool_calls"].as_array() {
            for (i, c) in calls.iter().enumerate() {
                asm.start(i, c["id"].as_str(), c["function"]["name"].as_str());
                match &c["function"]["arguments"] {
                    Value::String(s) => asm.push_args(i, s),
                    // 일부 호환 서버는 인자를 객체로 준다
                    v @ Value::Object(_) => asm.push_args(i, &v.to_string()),
                    _ => {}
                }
            }
        }
        let tool_calls = asm.finish()?;
        let reason = choice["finish_reason"].as_str();
        Ok(Generation {
            content: msg["content"].as_str().unwrap_or_default().to_string(),
            finish_reason: reason.map(str::to_string),
            usage: super::usage_from(&json["usage"], "prompt_tokens", "completion_tokens"),
            stop: stop_kind(reason, !tool_calls.is_empty()),
            tool_calls,
            state: None,
        })
    }
}

#[async_trait]
impl AiProvider for OpenAIClient {
    async fn list_models(&self, call: &ProviderCall<'_>) -> Result<Vec<String>> {
        let resp = call.http.get(call.url("models")).bearer_auth(call.credential).send().await.map_err(net_err)?;
        let json: Value = ensure_success(resp).await?.json().await.map_err(|e| parse_err("모델 목록", e))?;
        Ok(json["data"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|m| m["id"].as_str().map(str::to_string)).collect())
            .unwrap_or_default())
    }

    async fn generate_with(&self, call: &ProviderCall<'_>, prompt: &PromptMessage, extras: &ChatExtras, on_delta: &mut OnDelta<'_>) -> Result<Generation> {
        let official = is_official_openai(&call.profile.base_url);
        let resp = call
            .http
            .post(call.url("chat/completions"))
            .bearer_auth(call.credential)
            .json(&Self::body_with(&call.profile.model, prompt, extras, official))
            .send()
            .await
            .map_err(net_err)?;
        let resp = ensure_success(resp).await?;

        if !prompt.chat_options.stream {
            let json: Value = resp.json().await.map_err(|e| parse_err("OpenAI", e))?;
            return Self::parse_response(&json);
        }

        let mut out = Generation::default();
        let mut asm = CallAssembler::default();
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
            // 도구 호출 조각: index로 모으고, id·이름은 첫 조각, 인자는 글 조각을 이어 붙인다 (index가 없는 서버는 배열 위치)
            if let Some(calls) = choice["delta"]["tool_calls"].as_array() {
                for (i, c) in calls.iter().enumerate() {
                    let index = c["index"].as_u64().map_or(i, |n| n as usize);
                    asm.start(index, c["id"].as_str(), c["function"]["name"].as_str());
                    match &c["function"]["arguments"] {
                        Value::String(s) => asm.push_args(index, s),
                        v @ Value::Object(_) => asm.push_args(index, &v.to_string()),
                        _ => {}
                    }
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
        out.tool_calls = asm.finish()?;
        out.stop = stop_kind(out.finish_reason.as_deref(), !out.tool_calls.is_empty());
        Ok(out)
    }

    async fn embed(&self, call: &ProviderCall<'_>, req: &EmbedRequest<'_>) -> Result<Embeddings> {
        // OpenAI에는 문서/질의 구분이 없다 (req.task는 쓰지 않음)
        let _: EmbedTask = req.task;
        let mut out = Embeddings::default();
        let mut tokens: Option<u64> = None;
        for chunk in req.inputs.chunks(EMBED_BATCH) {
            let mut body = json!({ "model": req.model, "input": chunk, "encoding_format": "float" });
            if let Some(d) = req.dimensions {
                body["dimensions"] = json!(d);
            }
            let resp = call
                .http
                .post(call.url("embeddings"))
                .bearer_auth(call.credential)
                .json(&body)
                .send()
                .await
                .map_err(net_err)?;
            let json: Value = ensure_success(resp).await?.json().await.map_err(|e| parse_err("임베딩", e))?;
            let mut rows: Vec<(usize, Vec<f32>)> = json["data"]
                .as_array()
                .ok_or_else(|| parse_err("임베딩", "data가 없음"))?
                .iter()
                .enumerate()
                .map(|(i, d)| {
                    let v = floats(&d["embedding"]).ok_or_else(|| parse_err("임베딩", "embedding이 숫자 목록이 아님"))?;
                    Ok((d["index"].as_u64().map_or(i, |n| n as usize), v))
                })
                .collect::<Result<_>>()?;
            if rows.len() != chunk.len() {
                return Err(parse_err("임베딩", format!("{}개를 보냈는데 {}개가 옴", chunk.len(), rows.len())));
            }
            rows.sort_by_key(|(i, _)| *i);
            out.vectors.extend(rows.into_iter().map(|(_, v)| v));
            if let Some(t) = json["usage"]["prompt_tokens"].as_u64().or_else(|| json["usage"]["total_tokens"].as_u64()) {
                *tokens.get_or_insert(0) += t;
            }
        }
        out.tokens = tokens;
        Ok(out)
    }
}

pub(crate) fn floats(v: &Value) -> Option<Vec<f32>> {
    v.as_array()?.iter().map(|x| x.as_f64().map(|f| f as f32)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::testutil::{profile, serve, Reply};
    use crate::adapter::{ResponseSchema, ToolCall, ToolResult, ToolSpec};
    use crate::models::ai_service::{ChatOptions, PromptMessageItem, PromptRole};

    fn prompt(stream: bool) -> PromptMessage {
        PromptMessage {
            chat_options: ChatOptions {
                stream,
                max_tokens: Some(200),
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
            parameters: json!({ "type": "object", "properties": { "city": { "type": "string" } }, "required": ["city"], "additionalProperties": false }),
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
    fn reasoning_effort_is_sent_only_when_set() {
        let mut p = prompt(false);
        assert!(OpenAIClient::body("m", &p).get("reasoning_effort").is_none());
        p.chat_options.reasoning_effort = Some("low".into());
        assert_eq!(OpenAIClient::body("m", &p)["reasoning_effort"], "low");
    }

    #[test]
    fn plain_requests_are_unchanged() {
        let b = OpenAIClient::body("m", &prompt(true));
        assert_eq!(
            b.as_object().unwrap().keys().cloned().collect::<std::collections::BTreeSet<_>>(),
            ["max_tokens", "messages", "model", "stream"].map(String::from).into()
        );
        // 호환 서버에는 stream_options를 보내지 않는다. 공식에서만
        assert!(OpenAIClient::body_with("m", &prompt(true), &ChatExtras::default(), false)
            .get("stream_options")
            .is_none());
        assert_eq!(
            OpenAIClient::body_with("m", &prompt(true), &ChatExtras::default(), true)["stream_options"]["include_usage"],
            true
        );
        assert!(OpenAIClient::body_with("m", &prompt(false), &ChatExtras::default(), true)
            .get("stream_options")
            .is_none());
    }

    #[test]
    fn tools_choice_strict_and_tail() {
        let extras = ChatExtras {
            tools: vec![weather()],
            tool_choice: ToolChoice::Named("get_weather".into()),
            tail: vec![
                ToolTurn::Assistant {
                    text: String::new(),
                    calls: vec![ToolCall {
                        id: "c1".into(),
                        name: "get_weather".into(),
                        arguments: json!({ "city": "서울" }),
                        raw_arguments: String::new(),
                        parse_error: None,
                    }],
                    state: None,
                },
                ToolTurn::Results {
                    results: vec![ToolResult {
                        call_id: "c1".into(),
                        name: "get_weather".into(),
                        content: "맑음".into(),
                        is_error: false,
                    }],
                    note: Some("남은 단계 1".into()),
                },
            ],
            ..Default::default()
        };
        let b = OpenAIClient::body_with("m", &prompt(false), &extras, true);
        assert_eq!(b["tools"][0]["function"]["name"], "get_weather");
        assert_eq!(b["tools"][0]["function"]["strict"], true, "스키마가 strict 조건을 만족하고 공식 주소");
        assert_eq!(b["parallel_tool_calls"], false);
        assert_eq!(b["tool_choice"], json!({ "type": "function", "function": { "name": "get_weather" } }));
        let m = b["messages"].as_array().unwrap();
        assert_eq!(m[1]["role"], "assistant");
        assert!(m[1]["content"].is_null());
        assert_eq!(m[1]["tool_calls"][0]["function"]["arguments"], "{\"city\":\"서울\"}", "인자는 JSON 글");
        assert_eq!(m[2], json!({ "role": "tool", "tool_call_id": "c1", "content": "맑음" }));
        assert_eq!(m[3], json!({ "role": "user", "content": "남은 단계 1" }));
        // 호환 서버: strict·병렬 끄기 없음
        let b = OpenAIClient::body_with("m", &prompt(false), &extras, false);
        assert!(b["tools"][0]["function"].get("strict").is_none() && b.get("parallel_tool_calls").is_none());
        // 선택 종류
        for (choice, want) in [(ToolChoice::None, json!("none")), (ToolChoice::Required, json!("required"))] {
            let b = OpenAIClient::body_with(
                "m",
                &prompt(false),
                &ChatExtras {
                    tools: vec![weather()],
                    tool_choice: choice,
                    ..Default::default()
                },
                false,
            );
            assert_eq!(b["tool_choice"], want);
        }
        assert!(OpenAIClient::body_with(
            "m",
            &prompt(false),
            &ChatExtras {
                tools: vec![weather()],
                ..Default::default()
            },
            false
        )
        .get("tool_choice")
        .is_none());
        // 깨진 인자·오류 결과
        let broken = ChatExtras {
            tools: vec![weather()],
            tail: vec![
                ToolTurn::Assistant {
                    text: "찾아볼게요".into(),
                    calls: vec![ToolCall {
                        id: "c1".into(),
                        name: "get_weather".into(),
                        arguments: json!({}),
                        raw_arguments: "{\"city".into(),
                        parse_error: Some("x".into()),
                    }],
                    state: None,
                },
                ToolTurn::Results {
                    results: vec![ToolResult {
                        call_id: "c1".into(),
                        name: "get_weather".into(),
                        content: "인자가 JSON이 아님".into(),
                        is_error: true,
                    }],
                    note: None,
                },
            ],
            ..Default::default()
        };
        let b = OpenAIClient::body_with("m", &prompt(false), &broken, false);
        assert_eq!(b["messages"][1]["tool_calls"][0]["function"]["arguments"], "{}");
        assert_eq!(b["messages"][1]["content"], "찾아볼게요");
        assert!(b["messages"][2]["content"].as_str().unwrap().starts_with("[오류]"));
    }

    #[test]
    fn structured_response_format() {
        let schema = json!({ "type": "object", "properties": { "a": { "type": "string" } }, "required": ["a"], "additionalProperties": false });
        let b = OpenAIClient::body_with(
            "m",
            &prompt(false),
            &ChatExtras {
                response_schema: Some(ResponseSchema {
                    name: "out".into(),
                    description: Some("결과".into()),
                    schema: schema.clone(),
                }),
                ..Default::default()
            },
            true,
        );
        let rf = &b["response_format"];
        assert_eq!(rf["type"], "json_schema");
        assert_eq!(
            (&rf["json_schema"]["name"], &rf["json_schema"]["description"], &rf["json_schema"]["strict"]),
            (&json!("out"), &json!("결과"), &json!(true))
        );
        assert_eq!(rf["json_schema"]["schema"], schema);
        // 선택 항목이 있는 스키마는 strict를 끈다 (공급자가 400을 내지 않게)
        let open = json!({ "type": "object", "properties": { "a": { "type": "string" } } });
        let b = OpenAIClient::body_with(
            "m",
            &prompt(false),
            &ChatExtras {
                response_schema: Some(ResponseSchema {
                    name: "out".into(),
                    description: None,
                    schema: open,
                }),
                ..Default::default()
            },
            true,
        );
        assert_eq!(b["response_format"]["json_schema"]["strict"], false);
    }

    #[tokio::test]
    async fn non_streaming_tool_call_round_trip() {
        let srv = serve(vec![Reply::ok(json!({
            "choices": [{ "finish_reason": "tool_calls", "message": { "content": null, "tool_calls": [
                { "id": "call_a", "type": "function", "function": { "name": "get_weather", "arguments": "{\"city\":\"서울\"}" } },
                { "id": "call_b", "type": "function", "function": { "name": "get_weather", "arguments": { "city": "부산" } } }
            ] } }],
            "usage": { "prompt_tokens": 30, "completion_tokens": 12 }
        }))])
        .await;
        let (http, p) = profile(&format!("{}/v1", srv.base), "openai", "gpt-4o");
        let extras = ChatExtras {
            tools: vec![weather()],
            ..Default::default()
        };
        let g = OpenAIClient
            .generate_with(&call_of(&http, &p), &prompt(false), &extras, &mut |_| {})
            .await
            .unwrap();
        assert_eq!(g.stop, StopKind::ToolUse);
        assert_eq!(g.usage, Some((30, 12)));
        assert_eq!(g.tool_calls.len(), 2);
        assert_eq!(
            (g.tool_calls[0].id.as_str(), g.tool_calls[0].arguments.clone()),
            ("call_a", json!({ "city": "서울" }))
        );
        assert_eq!(g.tool_calls[1].arguments, json!({ "city": "부산" }), "인자가 객체로 와도 읽음");
        let seen = srv.last();
        assert_eq!(seen.path, "/v1/chat/completions");
        assert_eq!(seen.headers["authorization"], "Bearer test-key");
        assert_eq!(seen.json()["tools"][0]["function"]["name"], "get_weather");
    }

    #[tokio::test]
    async fn streaming_tool_call_fragments_across_chunk_boundaries() {
        let chunk = |v: Value| format!("data: {v}\n\n").into_bytes();
        let mut bytes: Vec<Vec<u8>> = vec![
            chunk(json!({ "choices": [{ "delta": { "role": "assistant", "content": "찾아" } }] })),
            chunk(json!({ "choices": [{ "delta": { "content": "볼게요" } }] })),
            chunk(
                json!({ "choices": [{ "delta": { "tool_calls": [{ "index": 0, "id": "call_a", "type": "function", "function": { "name": "get_weather", "arguments": "" } }] } }] }),
            ),
            chunk(json!({ "choices": [{ "delta": { "tool_calls": [{ "index": 0, "function": { "arguments": "{\"city\": \"서" } }] } }] })),
            chunk(
                json!({ "choices": [{ "delta": { "tool_calls": [{ "index": 1, "id": "call_b", "type": "function", "function": { "name": "get_weather", "arguments": "{\"city\":\"부산\"}" } }] } }] }),
            ),
            chunk(json!({ "choices": [{ "delta": { "tool_calls": [{ "index": 0, "function": { "arguments": "울\"}" } }] } }] })),
            chunk(json!({ "choices": [{ "delta": {}, "finish_reason": "tool_calls" }] })),
            chunk(json!({ "choices": [], "usage": { "prompt_tokens": 40, "completion_tokens": 9 } })),
            b"data: [DONE]\n\n".to_vec(),
        ];
        // 한글(3바이트) 한가운데서 청크가 끊겨도 된다: "서" 조각이 든 청크를 바이트 단위로 쪼갠다
        let i = bytes.iter().position(|b| String::from_utf8_lossy(b).contains("\\\"서")).unwrap();
        let whole = bytes.remove(i);
        let cut = whole.windows(3).position(|w| w == "서".as_bytes()).unwrap() + 1;
        bytes.insert(i, whole[cut..].to_vec());
        bytes.insert(i, whole[..cut].to_vec());
        let srv = serve(vec![Reply::Sse(bytes)]).await;
        let (http, p) = profile(&srv.base, "openai", "gpt-4o");
        let mut text = String::new();
        let extras = ChatExtras {
            tools: vec![weather()],
            ..Default::default()
        };
        let g = OpenAIClient
            .generate_with(&call_of(&http, &p), &prompt(true), &extras, &mut |d| text.push_str(d))
            .await
            .unwrap();
        assert_eq!(text, "찾아볼게요", "글 조각은 그대로 스트리밍");
        assert_eq!(g.content, "찾아볼게요");
        assert_eq!(g.stop, StopKind::ToolUse);
        assert_eq!(g.usage, Some((40, 9)));
        let args: Vec<_> = g.tool_calls.iter().map(|c| (c.id.as_str(), c.arguments.clone())).collect();
        assert_eq!(args, vec![("call_a", json!({ "city": "서울" })), ("call_b", json!({ "city": "부산" }))]);
        assert!(g.tool_calls.iter().all(|c| c.parse_error.is_none()));
    }

    #[tokio::test]
    async fn truncated_stream_keeps_the_broken_call() {
        let chunk = |v: Value| format!("data: {v}\n\n").into_bytes();
        let srv = serve(vec![Reply::Sse(vec![
            chunk(json!({ "choices": [{ "delta": { "tool_calls": [{ "index": 0, "id": "c", "function": { "name": "get_weather", "arguments": "{\"city\": \"서" } }] } }] })),
            chunk(json!({ "choices": [{ "delta": {}, "finish_reason": "length" }] })),
            b"data: [DONE]\n\n".to_vec(),
        ])])
        .await;
        let (http, p) = profile(&srv.base, "openai", "gpt-4o");
        let g = OpenAIClient
            .generate_with(
                &call_of(&http, &p),
                &prompt(true),
                &ChatExtras {
                    tools: vec![weather()],
                    ..Default::default()
                },
                &mut |_| {},
            )
            .await
            .unwrap();
        assert_eq!(g.stop, StopKind::MaxTokens);
        assert!(g.tool_calls[0].parse_error.is_some(), "잘린 인자는 오류로 표시");
    }

    #[tokio::test]
    async fn embeddings_are_batched_sorted_and_counted() {
        let reply = |n: usize, offset: usize, tokens: u64| {
            // 일부러 거꾸로 돌려준다 (index로 정렬해야 함)
            let data: Vec<Value> = (0..n).rev().map(|i| json!({ "index": i, "embedding": [(offset + i) as f64, 0.5] })).collect();
            Reply::ok(json!({ "data": data, "usage": { "prompt_tokens": tokens, "total_tokens": tokens } }))
        };
        let srv = serve(vec![reply(100, 0, 700), reply(5, 100, 40)]).await;
        let (http, p) = profile(&srv.base, "openai", "gpt-4o");
        let inputs: Vec<String> = (0..105).map(|i| format!("글 {i}")).collect();
        let e = OpenAIClient
            .embed(
                &call_of(&http, &p),
                &EmbedRequest {
                    model: "text-embedding-3-small",
                    inputs: &inputs,
                    task: EmbedTask::Document,
                    dimensions: Some(256),
                },
            )
            .await
            .unwrap();
        assert_eq!(e.vectors.len(), 105);
        assert_eq!(e.vectors[0][0], 0.0);
        assert_eq!(e.vectors[104][0], 104.0, "배치를 넘어도 순서가 이어짐");
        assert_eq!(e.tokens, Some(740));
        let reqs = srv.requests();
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[0].path, "/embeddings");
        let b = reqs[0].json();
        assert_eq!(
            (b["model"].clone(), b["dimensions"].clone(), b["input"].as_array().unwrap().len()),
            (json!("text-embedding-3-small"), json!(256), 100)
        );
        // 개수가 어긋나면 오류
        let srv = serve(vec![Reply::ok(json!({ "data": [{ "index": 0, "embedding": [1.0] }] }))]).await;
        let (http, p) = profile(&srv.base, "openai", "m");
        let two = vec!["a".to_string(), "b".to_string()];
        assert!(OpenAIClient
            .embed(
                &call_of(&http, &p),
                &EmbedRequest {
                    model: "m",
                    inputs: &two,
                    task: EmbedTask::Query,
                    dimensions: None
                }
            )
            .await
            .is_err());
    }
}
