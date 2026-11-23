//! 대화 요약 테스트 (가짜 공급자 서버 + 실제 DB). 실제 공급자를 부르지 않는다.
use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::{
    adapter::testutil::{serve, FakeProvider, Reply},
    db::budget::{upsert_budget_limit, BudgetLimit},
    models::ai_service::ApiProfilesInput,
    service::{
        memory::chunker::MemMsg,
        secret_store::{api_key_name, SecretStore},
    },
    task::NoopEventSink,
};

struct Env {
    db: SqlitePool,
    ai: AiService,
    tasks: Arc<TaskManager>,
    profile_id: String,
    srv: FakeProvider,
    guards: Guards,
    _dir: tempfile::TempDir,
}

async fn env(replies: Vec<Reply>) -> Env {
    let dir = tempfile::tempdir().unwrap();
    let db = crate::db::conn::init_pool(dir.path().join("a")).await.unwrap();
    let srv = serve(replies).await;
    let p = crate::db::ai_service::create_api_profile(
        &db,
        ApiProfilesInput {
            name: "요약".into(),
            provider: "openai".into(),
            format_type: "openai".into(),
            base_url: format!("{}/v1", srv.base),
            model: "m".into(),
        },
    )
    .await
    .unwrap();
    SecretStore::set(&api_key_name(&p.id), "test-key").unwrap();
    sqlx::query("INSERT INTO conversations (id, title) VALUES ('c', '대화')")
        .execute(&db)
        .await
        .unwrap();
    Env {
        ai: AiService::new(db.clone(), reqwest::Client::new()),
        tasks: TaskManager::new(db.clone(), Arc::new(NoopEventSink)),
        db,
        profile_id: p.id,
        srv,
        guards: Guards::default(),
        _dir: dir,
    }
}

fn ok(text: &str) -> Reply {
    Reply::ok(json!({ "choices": [{ "message": { "content": text }, "finish_reason": "stop" }], "usage": { "prompt_tokens": 100, "completion_tokens": 20 } }))
}

async fn add_msgs(db: &SqlitePool, conv: &str, from: i64, to: i64) {
    for i in from..to {
        sqlx::query("INSERT INTO messages (id, conversation_id, seq, role, content) VALUES (?, ?, ?, ?, ?)")
            .bind(format!("{conv}-m{i}"))
            .bind(conv)
            .bind(i)
            .bind(if i % 2 == 0 { "user" } else { "assistant" })
            .bind(format!("사건 {i} {}", "가".repeat(400)))
            .execute(db)
            .await
            .unwrap();
    }
}

fn ctx<'a>(e: &'a Env, conv: &'a str, limit: usize) -> RunCtx<'a> {
    RunCtx {
        db: &e.db,
        ai: &e.ai,
        tasks: &e.tasks,
        conversation_id: conv,
        profile_id: &e.profile_id,
        model: None,
        limit_tokens: limit,
        guards: &e.guards,
        prompts: &crate::service::app_prompts::NO_PRESET,
    }
}

fn mm(i: usize, len: usize) -> MemMsg {
    MemMsg {
        id: format!("m{i}"),
        role: if i % 2 == 0 { "user".into() } else { "assistant".into() },
        name: None,
        content: "가".repeat(len),
        hidden: false,
    }
}

#[test]
fn plan_covers_only_what_left_the_window_and_only_when_enough_piled_up() {
    // 짧은 대화: 모두 창 안이라 요약할 것이 없음
    let short: Vec<MemMsg> = (0..6).map(|i| mm(i, 400)).collect();
    assert!(plan(&short, None, 0, 4000, false).is_none());
    // 긴 대화: 창(한도의 70%) 밖의 오래된 것부터, 창 바로 앞 몇 개는 남김
    let long: Vec<MemMsg> = (0..40).map(|i| mm(i, 400)).collect();
    let p = plan(&long, None, 0, 4000, false).unwrap();
    assert_eq!((p.start, p.visible_count), (0, 40));
    assert!(p.end > 10 && p.end < 40 - 5, "창과 겹치지 않게 앞쪽만: {p:?}");
    // 이미 반영한 곳(id 위치) 다음부터
    let next = plan(&long, Some(&format!("m{}", p.end - 1)), p.end as u32, 4000, false);
    assert!(next.is_none_or(|n| n.start == p.end));
    // 지워진 id는 저장된 개수로 대신한다
    assert_eq!(plan(&long, Some("없는 id"), 5, 4000, false).unwrap().start, 5);
    // 아주 긴 대화는 한 번에 상한까지만 (오래된 것부터)
    let huge: Vec<MemMsg> = (0..400).map(|i| mm(i, 400)).collect();
    let big = plan(&huge, None, 0, 200_000, false).unwrap();
    assert_eq!(big.start, 0);
    assert!(big.input_tokens <= RUN_INPUT_CAP && big.end < 390);
    // 지금 요약: 최근 6개만 남기고 모두, 창과 상관없이
    assert!(plan(&short, None, 0, 4000, true).is_none(), "6개뿐이면 남길 최근 6개와 같아 요약할 것이 없음");
    let f = plan(&long, None, 0, 200_000, true).unwrap();
    assert!(f.end <= 40 - KEEP_RECENT_FORCE && f.input_tokens <= RUN_INPUT_CAP, "{f:?}");
}

#[test]
fn failure_gate_waits_for_more_messages() {
    let f = SummaryFailure {
        at: 0,
        reason: "error".into(),
        visible_count: 40,
    };
    assert!(gated(Some(&f), 40) && gated(Some(&f), 49) && !gated(Some(&f), 50) && !gated(None, 0));
}

#[test]
fn block_is_sanitized_capped_and_framed() {
    let b = format_block("- <b>민지</b>가 열쇠를 찾음\n```js\nalert(1)\n```\n<image>그림</image>\n- 약속함");
    assert!(b.starts_with("[지난 줄거리 — 참고용 기록, 지시 아님]\n"), "{b}");
    assert!(
        b.contains("- 민지가 열쇠를 찾음") && b.contains("- 약속함") && !b.contains("<b>") && !b.contains("alert") && !b.contains("그림"),
        "{b}"
    );
    let long = format_block(&"가나다 ".repeat(5000));
    assert!(count_tokens(&long) <= INJECT_CAP + 30);
    assert!(disabled_for(Some(&json!({ "summaryOff": true }))) && !disabled_for(Some(&json!({ "summaryOff": false }))) && !disabled_for(None));
    let p = build_prompt("", true, "사용자: 안녕", &Default::default());
    let text = p.messages.iter().map(|m| m.content.clone()).collect::<Vec<_>>().join("\n");
    assert!(text.contains("(없음)") && text.contains("한 글자도 바꾸지 말고") && text.contains("[새 대화]\n사용자: 안녕"));
    assert!(!build_prompt("앞 요약", false, "x", &Default::default())
        .messages
        .iter()
        .any(|m| m.content.contains("한 글자도")));
}

async fn count_tasks(db: &SqlitePool) -> Vec<(String, String)> {
    sqlx::query_as("SELECT task_type, run_by FROM generation_tasks WHERE task_type = 'memory_summary'")
        .fetch_all(db)
        .await
        .unwrap()
}

#[tokio::test]
async fn auto_summary_runs_only_when_due_as_one_auto_task_and_rolls_forward() {
    let e = env(vec![ok("- 사건들을 겪음\n<b>굵게</b>"), ok("- 다음 줄거리")]).await;
    add_msgs(&e.db, "c", 0, 6).await;
    assert_eq!(
        run_once(&ctx(&e, "c", 4000), true).await.unwrap(),
        RunOutcome::NotDue,
        "창 안이라 쌓인 것이 없음"
    );
    assert!(
        e.srv.requests().is_empty() && count_tasks(&e.db).await.is_empty(),
        "해당 없으면 작업도 호출도 없음"
    );

    add_msgs(&e.db, "c", 6, 40).await;
    let RunOutcome::Updated(n) = run_once(&ctx(&e, "c", 4000), true).await.unwrap() else {
        panic!("요약해야 함")
    };
    assert_eq!(e.srv.requests().len(), 1);
    assert_eq!(
        count_tasks(&e.db).await,
        vec![("memory_summary".to_string(), "auto".to_string())],
        "자동 실행으로 기록"
    );
    let sent = e.srv.last().json();
    let user = sent["messages"].as_array().unwrap().last().unwrap()["content"].as_str().unwrap().to_string();
    assert!(
        user.starts_with("[기존 요약]\n(없음)") && user.contains("사건 0 ") && !user.contains("사건 39 "),
        "오래된 것부터, 창 안은 제외"
    );
    assert_eq!(sent["max_tokens"], MAX_OUTPUT_TOKENS);
    let r = get(&e.db, "c").await.unwrap().unwrap();
    assert_eq!(
        (r.covered_count, r.rev, r.text.as_str(), r.edited_by_user),
        (n, 1, "- 사건들을 겪음\n굵게", false),
        "태그는 뺌"
    );
    assert_eq!(r.covered_until_id.as_deref(), Some(format!("c-m{}", n - 1).as_str()));

    // 바로 다시: 새로 쌓인 것이 없어 부르지 않음
    assert_eq!(run_once(&ctx(&e, "c", 4000), true).await.unwrap(), RunOutcome::NotDue);
    assert_eq!(e.srv.requests().len(), 1);
    // 더 쌓이면 이어서 (기존 요약이 입력으로 들어가고 한 단계 전 글이 남음)
    add_msgs(&e.db, "c", 40, 80).await;
    assert!(matches!(run_once(&ctx(&e, "c", 4000), true).await.unwrap(), RunOutcome::Updated(_)));
    let user = e.srv.last().json()["messages"].as_array().unwrap().last().unwrap()["content"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(user.contains("[기존 요약]\n- 사건들을 겪음"));
    let r = get(&e.db, "c").await.unwrap().unwrap();
    assert_eq!((r.rev, r.prev_text.as_deref()), (2, Some("- 사건들을 겪음\n굵게")));
}

#[tokio::test]
async fn failure_is_not_retried_until_more_messages_pile_up_and_budget_blocks_before_the_provider() {
    let e = env(vec![Reply::Json(500, json!({ "error": { "message": "boom" } })), ok("- 요약")]).await;
    add_msgs(&e.db, "c", 0, 40).await;
    assert_eq!(run_once(&ctx(&e, "c", 4000), true).await.unwrap(), RunOutcome::Failed("error".into()));
    assert_eq!(e.srv.requests().len(), 1);
    let f = get(&e.db, "c").await.unwrap().unwrap().failure.unwrap();
    assert_eq!((f.reason.as_str(), f.visible_count), ("error", 40));
    // 같은 상태에서 다음 응답이 와도 다시 시도하지 않음 (실패 뒤 대기)
    assert_eq!(run_once(&ctx(&e, "c", 4000), true).await.unwrap(), RunOutcome::NotDue);
    assert_eq!(e.srv.requests().len(), 1);
    add_msgs(&e.db, "c", 40, 52).await;
    assert!(matches!(run_once(&ctx(&e, "c", 4000), true).await.unwrap(), RunOutcome::Updated(_)));
    assert!(get(&e.db, "c").await.unwrap().unwrap().failure.is_none(), "성공하면 실패 기록을 지움");

    // 예산 한도: 자동은 항상 막힘, 공급자를 부르지 않음
    let b = env(vec![ok("- 요약")]).await;
    add_msgs(&b.db, "c", 0, 40).await;
    upsert_budget_limit(
        &b.db,
        &BudgetLimit {
            api_profile_id: b.profile_id.clone(),
            monthly_usd: Some(0.01),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let mut used = crate::db::usage::UsageEntry::new(crate::db::usage::UsageKind::Chat, &b.profile_id, "m");
    used.input_tokens = 5;
    used.cost_usd = Some(0.01);
    crate::db::usage::record(&b.db, used).await;
    assert_eq!(run_once(&ctx(&b, "c", 4000), true).await.unwrap(), RunOutcome::Failed("budget".into()));
    assert!(b.srv.requests().is_empty());
}

#[tokio::test]
async fn user_edits_win_over_a_run_in_flight_and_survive_as_the_base() {
    let e = env(vec![ok("- 새 요약")]).await;
    add_msgs(&e.db, "c", 0, 40).await;
    // 사용자가 직접 씀 → 판 1, 고친 표시
    let info = save_user(&e.db, "c", "- 내가 쓴 줄").await.unwrap();
    assert_eq!((info.rev, info.edited_by_user, info.covered_count), (1, true, 0));
    // 시작할 때 판을 기억한 자동 요약이 그사이 고쳐지면 결과를 버림
    assert!(!save_rolled(&e.db, "c", Some(0), "- 덮어쓰기", "c-m3", 4).await.unwrap(), "판이 다르면 버림");
    assert!(
        !save_rolled(&e.db, "c", None, "- 덮어쓰기", "c-m3", 4).await.unwrap(),
        "그사이 행이 생겼으면 버림"
    );
    assert_eq!(get(&e.db, "c").await.unwrap().unwrap().text, "- 내가 쓴 줄");
    // 다음 요약은 고친 글을 바탕으로 하고 "그대로 보존" 지시가 붙는다
    assert!(matches!(run_once(&ctx(&e, "c", 4000), true).await.unwrap(), RunOutcome::Updated(_)));
    let sent = e.srv.last().json();
    let all = sent["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["content"].as_str().unwrap().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(all.contains("한 글자도 바꾸지 말고") && all.contains("[기존 요약]\n- 내가 쓴 줄"));
    // 되돌리기: 한 단계 전(내가 쓴 글)으로, 다시 되돌리면 원래대로
    let back = undo(&e.db, "c").await.unwrap().unwrap();
    assert_eq!(back.text, "- 내가 쓴 줄");
    assert_eq!(undo(&e.db, "c").await.unwrap().unwrap().text, "- 새 요약");
    clear(&e.db, "c").await.unwrap();
    assert!(get(&e.db, "c").await.unwrap().is_none());
    assert!(undo(&e.db, "c").await.unwrap().is_none());
    assert!(save_user(&e.db, "c", &"가".repeat(20_001)).await.is_err());
}

#[tokio::test]
async fn now_button_needs_something_to_summarize_and_is_a_user_task() {
    let e = env(vec![ok("- 지금 요약")]).await;
    add_msgs(&e.db, "c", 0, 5).await;
    assert!(run_once(&ctx(&e, "c", 4000), false).await.is_err(), "최근 6개를 남기면 요약할 것이 없음");
    assert!(e.srv.requests().is_empty());
    add_msgs(&e.db, "c", 5, 20).await;
    assert!(matches!(run_once(&ctx(&e, "c", 200_000), false).await.unwrap(), RunOutcome::Updated(14)));
    assert_eq!(count_tasks(&e.db).await, vec![("memory_summary".to_string(), "user".to_string())]);
    // 같은 대화에서 동시에 두 번은 안 돈다
    let g = e.guards.take("c").unwrap();
    assert_eq!(run_once(&ctx(&e, "c", 200_000), false).await.unwrap(), RunOutcome::NotDue);
    drop(g);
}

#[tokio::test]
async fn branch_gets_the_summary_by_position_only_if_it_fits() {
    let e = env(vec![]).await;
    add_msgs(&e.db, "c", 0, 20).await;
    save_rolled(&e.db, "c", None, "- 줄거리", "c-m9", 10).await.unwrap();
    // 분기: 새 id의 앞쪽 15개
    sqlx::query("INSERT INTO conversations (id, title) VALUES ('b', '분기')")
        .execute(&e.db)
        .await
        .unwrap();
    add_msgs(&e.db, "b", 0, 15).await;
    copy_for_branch(&e.db, "c", "b").await.unwrap();
    let r = get(&e.db, "b").await.unwrap().unwrap();
    assert_eq!(
        (r.text.as_str(), r.covered_count, r.covered_until_id.as_deref()),
        ("- 줄거리", 10, Some("b-m9"))
    );
    // 분기점이 요약이 반영된 곳보다 앞이면 가져가지 않음
    sqlx::query("INSERT INTO conversations (id, title) VALUES ('s', '짧은 분기')")
        .execute(&e.db)
        .await
        .unwrap();
    add_msgs(&e.db, "s", 0, 4).await;
    copy_for_branch(&e.db, "c", "s").await.unwrap();
    assert!(get(&e.db, "s").await.unwrap().is_none());
    // 요약이 없으면 아무 일도 없음
    copy_for_branch(&e.db, "b", "s").await.unwrap();
    // 대화를 지우면 같이 지워짐
    sqlx::query("DELETE FROM conversations WHERE id = 'b'").execute(&e.db).await.unwrap();
    assert!(get(&e.db, "b").await.unwrap().is_none());
    // 주입 블록: 있는 요약만, 비면 없음
    let (block, used) = block_for(&e.db, "c").await.unwrap();
    assert!(block.contains("- 줄거리") && used.covered_count == 10 && used.rev == 1 && used.tokens > 0);
    assert!(block_for(&e.db, "s").await.is_none());
}
