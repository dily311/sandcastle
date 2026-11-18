//! AiService의 3-A 경로 (임베딩·도구·구조화 출력). 가짜 공급자 서버 + 실제 DB·키 저장소 (실제 공급자에 요청하지 않는다)
use serde_json::json;
use tokio_util::sync::CancellationToken;

use super::{ai_service::MAX_EMBED_INPUTS, *};
use crate::{
    adapter::{
        testutil::{serve, FakeProvider, Reply},
        tools::ToolSpec,
        ChatExtras, EmbedTask, ResponseSchema, StopKind, ToolChoice,
    },
    db::{self, conn::DbPool},
    db::{
        budget::{upsert_budget_limit, BudgetLimit},
        usage::{record, UsageEntry, UsageKind},
    },
    error::ErrorCode,
    models::ai_service::{ApiProfilesInput, ChatOptions, PromptMessage, PromptMessageItem, PromptRole},
    service::secret_store::{api_key_name, SecretStore},
    task::RunBy,
    task::{task_context_scope_for_test as scope, CurrentTask},
};

struct Env {
    ai: AiService,
    db: DbPool,
    profile_id: String,
    srv: FakeProvider,
    _dir: tempfile::TempDir,
}

async fn env(format: &str, model: &str, base_suffix: &str, replies: Vec<Reply>) -> Env {
    let dir = tempfile::tempdir().unwrap();
    let db = crate::db::conn::init_pool(dir.path().join("a")).await.unwrap();
    let srv = serve(replies).await;
    let p = db::ai_service::create_api_profile(
        &db,
        ApiProfilesInput {
            name: "시험".into(),
            provider: format.into(),
            format_type: format.into(),
            base_url: format!("{}{base_suffix}", srv.base),
            model: model.into(),
        },
    )
    .await
    .unwrap();
    SecretStore::set(&api_key_name(&p.id), "test-key").unwrap();
    Env {
        ai: AiService::new(db.clone(), reqwest::Client::new()),
        db,
        profile_id: p.id,
        srv,
        _dir: dir,
    }
}

fn prompt(max_tokens: u32) -> PromptMessage {
    PromptMessage {
        chat_options: ChatOptions {
            stream: false,
            max_tokens: Some(max_tokens),
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
        parameters: json!({ "type": "object", "properties": { "city": { "type": "string" } }, "required": ["city"] }),
    }
}

async fn usage_rows(db: &DbPool) -> Vec<(String, String, i64, i64, bool, bool)> {
    sqlx::query_as("SELECT kind, model, input_tokens, output_tokens, estimated, ok FROM usage_log ORDER BY id")
        .fetch_all(db)
        .await
        .unwrap()
}

fn embed_input(e: &Env, model: &str, inputs: &[&str]) -> EmbedInput {
    EmbedInput {
        profile_id: e.profile_id.clone(),
        model: model.into(),
        inputs: inputs.iter().map(|s| s.to_string()).collect(),
        task: EmbedTask::Document,
        dimensions: None,
    }
}

#[tokio::test]
async fn embeddings_record_usage_under_the_embedding_model() {
    let cancel = CancellationToken::new();
    // 공급자가 토큰 수를 알려 주는 경우
    let e = env(
        "openai",
        "gpt-4o",
        "/v1",
        vec![Reply::ok(
            json!({ "data": [{ "index": 0, "embedding": [1.0, 0.0] }, { "index": 1, "embedding": [0.0, 1.0] }], "usage": { "prompt_tokens": 11 } }),
        )],
    )
    .await;
    let out =
        e.ai.embed(embed_input(&e, "text-embedding-3-small", &["안녕", "잘 가"]), &cancel)
            .await
            .unwrap();
    assert_eq!((out.vectors.len(), out.input_tokens, out.estimated), (2, 11, false));
    assert_eq!(
        usage_rows(&e.db).await,
        vec![("embedding".into(), "text-embedding-3-small".into(), 11, 0, false, true)],
        "프로필의 채팅 모델이 아니라 임베딩 모델 이름으로 기록"
    );

    // 알려 주지 않으면(Gemini) 앱이 입력 글을 세어 추정으로 기록
    let g = env(
        "gemini",
        "gemini-2.5-flash",
        "",
        vec![Reply::ok(json!({ "embeddings": [{ "values": [1.0, 2.0] }] }))],
    )
    .await;
    let out =
        g.ai.embed(embed_input(&g, "gemini-embedding-001", &["고양이가 좋아요"]), &cancel)
            .await
            .unwrap();
    assert!(out.estimated && out.input_tokens > 0);
    let rows = usage_rows(&g.db).await;
    assert_eq!((rows[0].0.as_str(), rows[0].4, rows[0].2 as u64), ("embedding", true, out.input_tokens));

    // 실패한 호출도 기록 (요금이 나갈 수 있음)
    let f = env("openai", "gpt-4o", "", vec![Reply::Json(429, json!({ "error": { "message": "slow down" } }))]).await;
    assert!(f.ai.embed(embed_input(&f, "m", &["a"]), &cancel).await.is_err());
    let rows = usage_rows(&f.db).await;
    assert_eq!((rows.len(), rows[0].5), (1, false));
}

#[tokio::test]
async fn embeddings_are_refused_before_any_request_when_they_cannot_work() {
    let cancel = CancellationToken::new();
    // Claude에는 임베딩 API가 없다
    let c = env("claude", "claude-sonnet-5-5", "/v1", vec![]).await;
    let e = c.ai.embed(embed_input(&c, "x", &["a"]), &cancel).await.unwrap_err().to_response();
    assert_eq!(e.code, ErrorCode::AiFeatureUnsupported);
    assert!(e.message.contains("임베딩") && !e.retryable);
    assert!(c.srv.requests().is_empty() && usage_rows(&c.db).await.is_empty(), "요청도 사용량 기록도 없음");

    let o = env("openai", "gpt-4o", "", vec![]).await;
    for (inputs, why) in [(vec![], "없음"), (vec!["a", " "], "빈 글"), (vec![""], "빈 글")] {
        assert!(o.ai.embed(embed_input(&o, "m", &inputs), &cancel).await.is_err(), "{why}");
    }
    let many: Vec<String> = (0..MAX_EMBED_INPUTS + 1).map(|i| format!("글 {i}")).collect();
    let big = EmbedInput {
        inputs: many,
        ..embed_input(&o, "m", &["a"])
    };
    assert!(o.ai.embed(big, &cancel).await.unwrap_err().to_string().contains("나눠서"));
    assert!(o.ai.embed(embed_input(&o, " ", &["a"]), &cancel).await.is_err(), "모델 이름이 빔");
    assert!(o
        .ai
        .embed(
            EmbedInput {
                dimensions: Some(0),
                ..embed_input(&o, "m", &["a"])
            },
            &cancel
        )
        .await
        .is_err());
    assert!(o.srv.requests().is_empty());
}

#[tokio::test]
async fn embeddings_count_against_the_monthly_budget() {
    let e = env("openai", "gpt-4o", "", vec![]).await;
    upsert_budget_limit(
        &e.db,
        &BudgetLimit {
            api_profile_id: e.profile_id.clone(),
            monthly_usd: Some(1.0),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let mut used = UsageEntry::new(UsageKind::Chat, &e.profile_id, "gpt-4o");
    used.input_tokens = 5;
    used.cost_usd = Some(1.0);
    record(&e.db, used).await;
    let task = CurrentTask {
        task_id: "t".into(),
        feature: Some("memory".into()),
        run_by: RunBy::Auto,
    };
    let err = scope(task, e.ai.embed(embed_input(&e, "m", &["a"]), &CancellationToken::new()))
        .await
        .unwrap_err()
        .to_response();
    assert_eq!(err.code, ErrorCode::BudgetExceeded, "임베딩은 글 호출이라 같은 금액 한도에 센다");
    assert!(e.srv.requests().is_empty(), "한도를 넘으면 공급자를 부르지 않음");
}

#[tokio::test]
async fn tool_calls_come_back_with_floor_on_output_limit() {
    let reply = Reply::ok(json!({
        "choices": [{ "finish_reason": "tool_calls", "message": { "content": null, "tool_calls": [{ "id": "c1", "type": "function", "function": { "name": "get_weather", "arguments": "{\"city\":\"서울\"}" } }] } }],
        "usage": { "prompt_tokens": 20, "completion_tokens": 7 }
    }));
    let e = env("openai", "gpt-4o", "/v1", vec![reply]).await;
    let extras = ChatExtras {
        tools: vec![weather()],
        ..Default::default()
    };
    let r =
        e.ai.generate_ext(
            AiRequest {
                profile_id: e.profile_id.clone(),
                prompt: prompt(200),
            },
            extras,
            &CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap();
    assert_eq!(
        (r.stop, r.tool_calls.len(), r.tool_calls[0].name.as_str()),
        (StopKind::ToolUse, 1, "get_weather")
    );
    assert_eq!(e.srv.last().json()["max_tokens"], 2048, "인자 JSON이 잘리지 않게 출력 한도의 하한을 둔다");
    assert_eq!(usage_rows(&e.db).await, vec![("chat".into(), "gpt-4o".into(), 20, 7, false, true)]);
    // 도구가 없는 평범한 호출은 한도를 건드리지 않는다
    let e2 = env(
        "openai",
        "gpt-4o",
        "",
        vec![Reply::ok(json!({ "choices": [{ "message": { "content": "안녕" }, "finish_reason": "stop" }] }))],
    )
    .await;
    e2.ai
        .generate_once(AiRequest {
            profile_id: e2.profile_id.clone(),
            prompt: prompt(200),
        })
        .await
        .unwrap();
    assert_eq!(e2.srv.last().json()["max_tokens"], 200);
}

#[tokio::test]
async fn bad_tool_requests_are_refused_before_any_request() {
    let cancel = CancellationToken::new();
    let c2 = env("claude", "claude-2.1", "/v1", vec![]).await;
    let req = |e: &Env| AiRequest {
        profile_id: e.profile_id.clone(),
        prompt: prompt(200),
    };
    let e = c2
        .ai
        .generate_ext(
            req(&c2),
            ChatExtras {
                tools: vec![weather()],
                ..Default::default()
            },
            &cancel,
            |_| {},
        )
        .await
        .unwrap_err()
        .to_response();
    assert_eq!(e.code, ErrorCode::AiFeatureUnsupported, "Claude 2는 도구가 없음");

    let o = env("openai", "gpt-4o", "", vec![]).await;
    let mut bad = weather();
    bad.parameters = json!({ "type": "object", "properties": { "a": { "$ref": "#/x" } } });
    assert!(o
        .ai
        .generate_ext(
            req(&o),
            ChatExtras {
                tools: vec![bad],
                ..Default::default()
            },
            &cancel,
            |_| {}
        )
        .await
        .is_err());
    assert!(o
        .ai
        .generate_ext(
            req(&o),
            ChatExtras {
                tool_choice: ToolChoice::Required,
                ..Default::default()
            },
            &cancel,
            |_| {}
        )
        .await
        .is_err());
    assert_eq!((c2.srv.requests().len(), o.srv.requests().len(), usage_rows(&o.db).await.len()), (0, 0, 0));
}

#[tokio::test]
async fn a_tool_call_cut_off_by_the_output_limit_is_not_run() {
    let e = env(
        "openai",
        "gpt-4o",
        "",
        vec![Reply::ok(json!({
            "choices": [{ "finish_reason": "length", "message": { "content": null, "tool_calls": [{ "id": "c", "function": { "name": "get_weather", "arguments": "{\"city\": \"서" } }] } }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 2048 }
        }))],
    )
    .await;
    let err =
        e.ai.generate_ext(
            AiRequest {
                profile_id: e.profile_id.clone(),
                prompt: prompt(200),
            },
            ChatExtras {
                tools: vec![weather()],
                ..Default::default()
            },
            &CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap_err()
        .to_response();
    assert_eq!(err.code, ErrorCode::AiOutputTruncated);
    let rows = usage_rows(&e.db).await;
    assert_eq!((rows.len(), rows[0].3), (1, 2048), "이미 쓴 토큰은 기록");
}

fn mood() -> ResponseSchema {
    ResponseSchema {
        name: "mood".into(),
        description: None,
        schema: json!({ "type": "object", "properties": { "mood": { "enum": ["happy", "sad"] } }, "required": ["mood"], "additionalProperties": false }),
    }
}

#[tokio::test]
async fn structured_output_native_on_claude_and_gemini() {
    let cancel = CancellationToken::new();
    let c = env(
        "claude",
        "claude-sonnet-5-5",
        "/v1",
        vec![Reply::ok(
            json!({ "content": [{ "type": "tool_use", "id": "t", "name": "mood", "input": { "mood": "sad" } }], "stop_reason": "tool_use" }),
        )],
    )
    .await;
    let r =
        c.ai.generate_structured(
            StructuredRequest {
                profile_id: c.profile_id.clone(),
                prompt: prompt(100),
                schema: mood(),
            },
            &cancel,
        )
        .await
        .unwrap();
    assert_eq!((r.value, r.native), (json!({ "mood": "sad" }), true));
    let sent = c.srv.last().json();
    assert_eq!(
        (sent["tool_choice"].clone(), sent["stream"].clone()),
        (json!({ "type": "tool", "name": "mood" }), json!(false))
    );

    let g = env(
        "gemini",
        "gemini-2.5-flash",
        "",
        vec![Reply::ok(
            json!({ "candidates": [{ "finishReason": "STOP", "content": { "parts": [{ "text": "{\"mood\": \"happy\"}" }] } }] }),
        )],
    )
    .await;
    let r =
        g.ai.generate_structured(
            StructuredRequest {
                profile_id: g.profile_id.clone(),
                prompt: prompt(100),
                schema: mood(),
            },
            &cancel,
        )
        .await
        .unwrap();
    assert_eq!((r.value, r.native), (json!({ "mood": "happy" }), true));
    assert_eq!(g.srv.last().json()["generationConfig"]["responseMimeType"], "application/json");
}

#[tokio::test]
async fn structured_output_falls_back_to_instruction_plus_validation() {
    let cancel = CancellationToken::new();
    // 호환 서버(알 수 없음): 네이티브 response_format을 보내지 않고 지시를 덧붙인다
    let reply = Reply::ok(json!({ "choices": [{ "finish_reason": "stop", "message": { "content": "네!\n```json\n{\"mood\": \"happy\"}\n```" } }] }));
    let e = env("openai", "my-local-model", "/v1", vec![reply]).await;
    let r =
        e.ai.generate_structured(
            StructuredRequest {
                profile_id: e.profile_id.clone(),
                prompt: prompt(100),
                schema: mood(),
            },
            &cancel,
        )
        .await
        .unwrap();
    assert_eq!((r.value, r.native), (json!({ "mood": "happy" }), false));
    let sent = e.srv.last().json();
    assert!(sent.get("response_format").is_none());
    let last = sent["messages"].as_array().unwrap().last().unwrap().clone();
    assert!(last["role"] == "system" && last["content"].as_str().unwrap().contains("JSON Schema"));
    assert_eq!(sent["max_tokens"], 2048);

    // 스키마에 안 맞으면 오류 — 그리고 다시 부르지 않는다 (요청 1번)
    let bad = Reply::ok(
        json!({ "choices": [{ "finish_reason": "stop", "message": { "content": "{\"mood\": \"angry\"}" } }], "usage": { "prompt_tokens": 9, "completion_tokens": 5 } }),
    );
    let e = env(
        "openai",
        "my-local-model",
        "",
        vec![bad, Reply::ok(json!({ "choices": [{ "message": { "content": "{\"mood\": \"sad\"}" } }] }))],
    )
    .await;
    let err =
        e.ai.generate_structured(
            StructuredRequest {
                profile_id: e.profile_id.clone(),
                prompt: prompt(100),
                schema: mood(),
            },
            &cancel,
        )
        .await
        .unwrap_err()
        .to_response();
    assert_eq!(err.code, ErrorCode::AiStructuredOutputInvalid);
    assert_eq!(e.srv.requests().len(), 1, "자동 재시도 없음");
    assert_eq!(usage_rows(&e.db).await.len(), 1, "쓴 토큰은 기록");

    // 쓸 수 없는 스키마·글 생성이 안 되는 모델은 호출 전에 거절
    let mut weird = mood();
    weird.schema = json!({ "type": "object", "oneOf": [] });
    assert!(e
        .ai
        .generate_structured(
            StructuredRequest {
                profile_id: e.profile_id.clone(),
                prompt: prompt(100),
                schema: weird
            },
            &cancel
        )
        .await
        .is_err());
    let emb = env("gemini", "gemini-embedding-001", "", vec![]).await;
    let err = emb
        .ai
        .generate_structured(
            StructuredRequest {
                profile_id: emb.profile_id.clone(),
                prompt: prompt(100),
                schema: mood(),
            },
            &cancel,
        )
        .await
        .unwrap_err()
        .to_response();
    assert_eq!(err.code, ErrorCode::AiFeatureUnsupported);
    assert!(emb.srv.requests().is_empty());
}

#[test]
fn capabilities_of_a_profile_are_available_without_a_request() {
    let p = crate::models::ai_service::ApiProfiles {
        id: "x".into(),
        name: "n".into(),
        provider: "claude".into(),
        format_type: "claude".into(),
        base_url: "https://api.anthropic.com/v1".into(),
        model: "claude-sonnet-5-5".into(),
        created_at: 0,
        updated_at: 0,
    };
    let c = AiService::capabilities_of(&p);
    assert!(!c.embeddings.is_supported() && c.tools.is_supported() && c.structured.is_supported());
}
