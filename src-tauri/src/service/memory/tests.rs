//! 메모리 색인·검색 (가짜 공급자 서버 + 실제 DB·memory.db). 실제 공급자를 부르지 않는다.
use serde_json::json;
use tokio_util::sync::CancellationToken;

use super::{
    chunker,
    index::{self, BATCH},
    search::{self, SearchRequest},
    store::MemoryStore,
    *,
};
use crate::{
    adapter::testutil::{serve, FakeProvider, Reply},
    db::conn::DbPool,
    models::ai_service::ApiProfilesInput,
    service::{
        ai::AiService,
        secret_store::{api_key_name, SecretStore},
    },
};

struct Env {
    ai: AiService,
    db: DbPool,
    store: MemoryStore,
    profile: ApiProfiles,
    cfg: MemorySetting,
    srv: FakeProvider,
    _dir: tempfile::TempDir,
}

async fn env(replies: Vec<Reply>) -> Env {
    let dir = tempfile::tempdir().unwrap();
    let db = crate::db::conn::init_pool(dir.path().join("a")).await.unwrap();
    let srv = serve(replies).await;
    let profile = crate::db::ai_service::create_api_profile(
        &db,
        ApiProfilesInput {
            name: "임베딩".into(),
            provider: "openai".into(),
            format_type: "openai".into(),
            base_url: format!("{}/v1", srv.base),
            model: "chat".into(),
        },
    )
    .await
    .unwrap();
    SecretStore::set(&api_key_name(&profile.id), "test-key").unwrap();
    let cfg = MemorySetting {
        api_profile_id: Some(profile.id.clone()),
        model: "emb-1".into(),
        ..Default::default()
    };
    Env {
        ai: AiService::new(db.clone(), reqwest::Client::new()),
        store: MemoryStore::new(dir.path().join("memory.db")),
        db,
        profile,
        cfg,
        srv,
        _dir: dir,
    }
}

/// 응답을 새로 정해 서버를 다시 띄우고 프로필 주소를 맞춘다
async fn set_replies(e: &mut Env, replies: Vec<Reply>) {
    e.srv = serve(replies).await;
    e.profile.base_url = format!("{}/v1", e.srv.base);
    sqlx::query("UPDATE api_profiles SET base_url = ? WHERE id = ?")
        .bind(&e.profile.base_url)
        .bind(&e.profile.id)
        .execute(&e.db)
        .await
        .unwrap();
}

/// 글에서 뜻을 흉내 낸 3차원 벡터: [고양이, 강아지, 1]
fn fake_vec(text: &str) -> Vec<f32> {
    vec![text.matches("고양이").count() as f32, text.matches("강아지").count() as f32, 1.0]
}

fn embed_reply(texts: &[String]) -> Reply {
    let data: Vec<_> = texts.iter().enumerate().map(|(i, t)| json!({ "index": i, "embedding": fake_vec(t) })).collect();
    Reply::ok(json!({ "data": data, "usage": { "prompt_tokens": texts.len() * 10 } }))
}

async fn add_conv(db: &DbPool, id: &str, meta: Option<&str>) {
    sqlx::query("INSERT INTO conversations (id, title, meta) VALUES (?, ?, ?)")
        .bind(id)
        .bind(format!("대화 {id}"))
        .bind(meta)
        .execute(db)
        .await
        .unwrap();
}

async fn add_msg(db: &DbPool, conv: &str, seq: i64, role: &str, content: &str, meta: Option<&str>) -> String {
    let id = format!("{conv}-m{seq}");
    sqlx::query("INSERT INTO messages (id, conversation_id, seq, role, content, meta) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(conv)
        .bind(seq)
        .bind(role)
        .bind(content)
        .bind(meta)
        .execute(db)
        .await
        .unwrap();
    id
}

fn body(word: &str) -> String {
    format!("{word} {}", "가".repeat(390))
}

/// 6개 메시지(400자씩): 앞 3개는 고양이, 뒤 3개는 강아지 이야기 → chunk 2개
async fn pets_conv(db: &DbPool, id: &str) {
    add_conv(db, id, None).await;
    for i in 0..6 {
        let word = if i < 3 { "고양이" } else { "강아지" };
        add_msg(db, id, i, if i % 2 == 0 { "user" } else { "assistant" }, &body(word), None).await;
    }
}

async fn planned_texts(db: &DbPool, conv: &str) -> Vec<String> {
    chunker::plan(&load_messages(db, conv).await.unwrap(), &[])
        .into_iter()
        .map(|p| p.text)
        .collect()
}

fn scope(id: &str) -> MemoryScope {
    MemoryScope {
        conversation_id: Some(id.into()),
    }
}

fn query_req(q: &str, scope: MemoryScope) -> SearchRequest {
    SearchRequest {
        query: q.into(),
        scope,
        limit: None,
    }
}

async fn run_index(e: &Env, s: &MemoryScope, allow: bool) -> Result<index::IndexResult> {
    index::run(&e.db, &e.store, &e.ai, &e.profile, &e.cfg, s, allow, &CancellationToken::new()).await
}

#[tokio::test]
async fn index_then_search_finds_by_meaning_and_a_second_run_costs_nothing() {
    let mut e = env(vec![]).await;
    pets_conv(&e.db, "c1").await;
    let texts = planned_texts(&e.db, "c1").await;
    assert_eq!(texts.len(), 2);
    // 서버 응답: 색인 1번(2개 입력), 질의 1번
    set_replies(&mut e, vec![embed_reply(&texts), embed_reply(&["고양이".to_string()])]).await;

    // 미리보기: 요금 없음, 파일도 만들지 않음
    let pv = index::preview(&e.db, &e.store, &e.profile, &e.cfg, &scope("c1")).await.unwrap();
    assert_eq!((pv.conversations, pv.chunks, pv.cached, pv.to_embed, pv.over_cap), (1, 2, 0, 2, false));
    assert!(pv.est_tokens > 0 && e.srv.requests().is_empty() && !e.store.exists());

    let r = run_index(&e, &scope("c1"), false).await.unwrap();
    assert_eq!((r.chunks, r.embedded, r.cached, r.tokens), (2, 2, 0, 20));
    assert_eq!(e.srv.requests().len(), 1);
    let sent = e.srv.last().json();
    assert_eq!((sent["model"].as_str(), sent["input"].as_array().unwrap().len()), (Some("emb-1"), 2));

    // 다시 돌려도 공급자를 부르지 않는다
    let again = index::preview(&e.db, &e.store, &e.profile, &e.cfg, &scope("c1")).await.unwrap();
    assert_eq!((again.cached, again.to_embed, again.est_tokens), (2, 0, 0));
    let r2 = run_index(&e, &scope("c1"), false).await.unwrap();
    assert_eq!((r2.embedded, r2.cached), (0, 2));
    assert_eq!(e.srv.requests().len(), 1);

    // 뜻으로 찾기: "고양이"는 첫 chunk가 위
    let found = search::search(
        &e.db,
        &e.store,
        &e.ai,
        &e.profile,
        &e.cfg,
        &query_req("고양이", scope("c1")),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(e.srv.requests().len(), 2);
    assert_eq!((found.hits.len(), found.scanned, found.truncated, found.other_model_chunks), (2, 2, false, 0));
    assert_eq!(found.hits[0].first_message_id, "c1-m0");
    assert!(found.hits[0].score > found.hits[1].score && !found.hits[0].stale);
    assert!(found.hits[0].snippet.contains("고양이") && found.hits[0].conversation_title == "대화 c1");
    // 같은 질의는 다시 임베딩하지 않음
    search::search(
        &e.db,
        &e.store,
        &e.ai,
        &e.profile,
        &e.cfg,
        &query_req("고양이", scope("c1")),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(e.srv.requests().len(), 2, "질의 캐시");
}

#[tokio::test]
async fn editing_re_embeds_only_that_chunk_and_search_marks_it_stale_until_then() {
    let mut e = env(vec![]).await;
    pets_conv(&e.db, "c1").await;
    let texts = planned_texts(&e.db, "c1").await;
    // 고친 뒤 새 글: m4를 바꾸면 둘째 chunk만 달라진다
    sqlx::query("UPDATE messages SET content = ? WHERE id = 'c1-m4'")
        .bind(body("고양이"))
        .execute(&e.db)
        .await
        .unwrap();
    let after = planned_texts(&e.db, "c1").await;
    assert_eq!((after[0] == texts[0], after[1] == texts[1]), (true, false));
    sqlx::query("UPDATE messages SET content = ? WHERE id = 'c1-m4'")
        .bind(body("강아지"))
        .execute(&e.db)
        .await
        .unwrap();

    set_replies(
        &mut e,
        vec![embed_reply(&texts), embed_reply(&["강아지".to_string()]), embed_reply(&[after[1].clone()])],
    )
    .await;
    run_index(&e, &scope("c1"), false).await.unwrap();

    sqlx::query("UPDATE messages SET content = ? WHERE id = 'c1-m4'")
        .bind(body("고양이"))
        .execute(&e.db)
        .await
        .unwrap();
    let found = search::search(
        &e.db,
        &e.store,
        &e.ai,
        &e.profile,
        &e.cfg,
        &query_req("강아지", scope("c1")),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(found.hits.iter().any(|h| h.stale), "색인한 뒤 바뀐 chunk는 오래됨 표시");
    assert_eq!(found.hits.iter().filter(|h| !h.stale).count(), 1);

    let before = e.srv.requests().len();
    let r = run_index(&e, &scope("c1"), false).await.unwrap();
    assert_eq!((r.embedded, r.cached), (1, 1), "고친 chunk 하나만 다시 임베딩");
    assert_eq!(e.srv.requests().len(), before + 1);
    assert_eq!(e.srv.last().json()["input"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_branch_copy_costs_zero_provider_calls() {
    let mut e = env(vec![]).await;
    pets_conv(&e.db, "c1").await;
    let texts = planned_texts(&e.db, "c1").await;
    set_replies(&mut e, vec![embed_reply(&texts)]).await;
    run_index(&e, &scope("c1"), false).await.unwrap();
    // 분기 = 새 id의 같은 글
    pets_conv(&e.db, "c2").await;
    let pv = index::preview(&e.db, &e.store, &e.profile, &e.cfg, &MemoryScope::default()).await.unwrap();
    assert_eq!((pv.conversations, pv.chunks, pv.to_embed), (2, 4, 0));
    let r = run_index(&e, &MemoryScope::default(), false).await.unwrap();
    assert_eq!((r.embedded, r.cached), (0, 4));
    assert_eq!(e.srv.requests().len(), 1, "공급자 호출은 처음 한 번뿐");
}

#[tokio::test]
async fn token_cap_needs_a_second_confirmation_and_nothing_is_sent_before() {
    let mut e = env(vec![]).await;
    pets_conv(&e.db, "c1").await;
    let texts = planned_texts(&e.db, "c1").await;
    set_replies(&mut e, vec![embed_reply(&texts)]).await;
    e.cfg.token_cap = 10;
    let pv = index::preview(&e.db, &e.store, &e.profile, &e.cfg, &scope("c1")).await.unwrap();
    assert!(pv.over_cap);
    let err = run_index(&e, &scope("c1"), false).await.unwrap_err().to_string();
    assert!(err.contains("상한"), "{err}");
    assert!(e.srv.requests().is_empty() && !e.store.exists(), "확인 전에는 공급자도 파일도 건드리지 않음");
    assert_eq!(run_index(&e, &scope("c1"), true).await.unwrap().embedded, 2);
    // 설정이 비면 거절
    e.cfg.model = " ".into();
    assert!(run_index(&e, &scope("c1"), true).await.is_err());
}

#[tokio::test]
async fn a_failed_batch_keeps_earlier_batches_and_the_next_run_resumes_without_retry() {
    let mut e = env(vec![]).await;
    add_conv(&e.db, "big", None).await;
    for i in 0..210 {
        add_msg(
            &e.db,
            "big",
            i,
            if i % 2 == 0 { "user" } else { "assistant" },
            &body(&format!("고양이{i}")),
            None,
        )
        .await;
    }
    let texts = planned_texts(&e.db, "big").await;
    assert_eq!(texts.len(), 70);
    // 첫 배치 성공, 둘째 배치는 서버 오류
    // (주소가 모델 키에 들어가므로 서버는 하나로 — 이어서 돌릴 때 쓸 응답도 미리 줌)
    set_replies(
        &mut e,
        vec![
            embed_reply(&texts[..BATCH]),
            Reply::Json(500, json!({ "error": { "message": "boom" } })),
            embed_reply(&texts[BATCH..]),
        ],
    )
    .await;
    assert!(run_index(&e, &scope("big"), false).await.is_err());
    assert_eq!(e.srv.requests().len(), 2, "실패해도 다시 시도하지 않음");
    let st = status(&e.store, Some(&model_key(&e.profile, &e.cfg))).await.unwrap();
    assert_eq!((st.chunks_current, st.vectors), (BATCH as u64, BATCH as u64), "쓴 배치는 남음");

    // 이어서: 남은 6개만
    let r = run_index(&e, &scope("big"), false).await.unwrap();
    assert_eq!((r.embedded, r.cached), (6, 64));
    assert_eq!(e.srv.last().json()["input"].as_array().unwrap().len(), 6);

    // 취소 신호가 켜져 있으면 공급자를 부르지 않음
    let c = CancellationToken::new();
    c.cancel();
    sqlx::query("UPDATE messages SET content = ? WHERE id = 'big-m1'")
        .bind(body("강아지"))
        .execute(&e.db)
        .await
        .unwrap();
    let before = e.srv.requests().len();
    let err = index::run(&e.db, &e.store, &e.ai, &e.profile, &e.cfg, &scope("big"), false, &c)
        .await
        .unwrap_err()
        .to_response();
    assert_eq!(err.code, crate::error::ErrorCode::AiRequestCancelled);
    assert_eq!(e.srv.requests().len(), before);
}

#[tokio::test]
async fn excluded_conversations_and_messages_are_never_indexed() {
    let e = env(vec![]).await;
    add_conv(&e.db, "off", Some(r#"{"memory":"off"}"#)).await;
    add_conv(&e.db, "peek", Some(r#"{"phone":{"x":1}}"#)).await;
    for c in ["off", "peek"] {
        add_msg(&e.db, c, 0, "user", &body("고양이"), None).await;
    }
    add_conv(&e.db, "ok", None).await;
    add_msg(&e.db, "ok", 0, "user", &body("고양이"), Some(r#"{"hidden":true}"#)).await;
    add_msg(&e.db, "ok", 1, "system", &body("고양이"), None).await;
    let pv = index::preview(&e.db, &e.store, &e.profile, &e.cfg, &MemoryScope::default()).await.unwrap();
    assert_eq!((pv.conversations, pv.chunks), (1, 0), "숨김·시스템 메시지만 있는 대화는 chunk가 없음");
    // 제외한 대화를 직접 지정하면 거절
    assert!(index::preview(&e.db, &e.store, &e.profile, &e.cfg, &scope("off")).await.is_err());
}

#[tokio::test]
async fn other_models_are_not_mixed_and_purge_cleans_up() {
    let mut e = env(vec![]).await;
    pets_conv(&e.db, "c1").await;
    let texts = planned_texts(&e.db, "c1").await;
    set_replies(&mut e, vec![embed_reply(&texts), embed_reply(&["고양이".to_string()])]).await;
    run_index(&e, &scope("c1"), false).await.unwrap();

    // 모델(접두어)이 바뀌면 키가 달라 기존 chunk는 검색되지 않는다
    e.cfg.query_prefix = "query: ".into();
    let found = search::search(
        &e.db,
        &e.store,
        &e.ai,
        &e.profile,
        &e.cfg,
        &query_req("고양이", scope("c1")),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!((found.hits.len(), found.other_model_chunks), (0, 2));
    let st = status(&e.store, Some(&model_key(&e.profile, &e.cfg))).await.unwrap();
    assert_eq!((st.chunks_current, st.chunks_other), (0, 2));
    let removed = purge(&e.store, &e.db, Some(&model_key(&e.profile, &e.cfg)), true).await.unwrap();
    assert_eq!(removed, 2);
    assert_eq!(status(&e.store, None).await.unwrap().vectors, 0, "가리키는 chunk가 없는 벡터도 지움");
}

#[tokio::test]
async fn deleted_conversations_drop_out_of_search_and_purge_removes_them() {
    let mut e = env(vec![]).await;
    pets_conv(&e.db, "c1").await;
    pets_conv(&e.db, "c2").await;
    let texts = planned_texts(&e.db, "c1").await;
    set_replies(&mut e, vec![embed_reply(&texts), embed_reply(&["고양이".to_string()])]).await;
    run_index(&e, &MemoryScope::default(), false).await.unwrap();
    sqlx::query("DELETE FROM conversations WHERE id = 'c2'").execute(&e.db).await.unwrap();
    let found = search::search(
        &e.db,
        &e.store,
        &e.ai,
        &e.profile,
        &e.cfg,
        &query_req("고양이", MemoryScope::default()),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert!(
        found.hits.iter().all(|h| h.conversation_id == "c1") && found.hits.len() == 2,
        "c2의 중복 chunk는 사라진 대화라 건너뜀"
    );
    assert!(purge(&e.store, &e.db, None, false).await.unwrap() >= 2);
}

#[test]
fn top_k_keeps_the_best_and_dedupes_by_text() {
    let hit = |h: &str, s: f32| search::RawHit {
        chunk_id: 0,
        conversation_id: "c".into(),
        first_message_id: h.into(),
        last_message_id: h.into(),
        text_hash: h.into(),
        score: s,
    };
    let mut top = vec![];
    for (h, s) in [("a", 0.1), ("b", 0.9), ("c", 0.5), ("a", 0.7), ("d", 0.2), ("b", 0.3)] {
        search::offer(&mut top, 3, hit(h, s));
    }
    let got: Vec<(String, f32)> = top.iter().map(|h| (h.text_hash.clone(), h.score)).collect();
    assert_eq!(got, vec![("b".to_string(), 0.9), ("a".to_string(), 0.7), ("c".to_string(), 0.5)]);
}

// ---------------------------------------------------------------------------
// 자동 회상 (3-C-5)
// ---------------------------------------------------------------------------
use super::recall::{self, RecallParams};
use crate::task::{NoopEventSink, TaskManager};
use std::collections::HashSet;
use std::sync::Arc;

/// 30개 chunk(메시지 90개) 중 11번째만 고양이 이야기, 나머지는 강아지 수가 다른 잡음
async fn big_conv(db: &DbPool, id: &str) {
    add_conv(db, id, None).await;
    for i in 0..90i64 {
        let c = i / 3;
        let word = if c == 10 {
            "고양이 고양이 고양이".to_string()
        } else {
            "강아지 ".repeat(1 + (c % 3) as usize).trim().to_string()
        };
        add_msg(
            db,
            id,
            i,
            if i % 2 == 0 { "user" } else { "assistant" },
            &format!("{word} {c}-{i} {}", "가".repeat(380)),
            None,
        )
        .await;
    }
}

struct Rec {
    e: Env,
    tasks: Arc<TaskManager>,
    input: String,
}

async fn recall_env(extra_replies: Vec<Reply>) -> Rec {
    let mut e = env(vec![]).await;
    big_conv(&e.db, "big").await;
    let texts = planned_texts(&e.db, "big").await;
    assert_eq!(texts.len(), 30);
    let mut replies = vec![embed_reply(&texts)];
    replies.extend(extra_replies);
    set_replies(&mut e, replies).await;
    e.cfg.auto_recall = true;
    run_index(&e, &scope("big"), false).await.unwrap();
    let tasks = TaskManager::new(e.db.clone(), Arc::new(NoopEventSink));
    Rec {
        e,
        tasks,
        input: "고양이 이야기를 다시 해 줘".into(),
    }
}

async fn do_recall(r: &Rec, input: &str, window: &HashSet<String>, reuse: Option<&[recall::MemoryRef]>, cancel: &CancellationToken) -> recall::RecallOutcome {
    recall::recall(RecallParams {
        db: &r.e.db,
        store: &r.e.store,
        ai: &r.e.ai,
        tasks: &r.tasks,
        cfg: &r.e.cfg,
        profile: &r.e.profile,
        conversation_id: "big",
        character_id: "",
        input,
        last_assistant: None,
        window_ids: window,
        reuse,
        chat_cancel: cancel,
    })
    .await
}

async fn embedding_tasks(db: &DbPool) -> Vec<(String, String, String)> {
    sqlx::query_as("SELECT task_type, run_by, status FROM generation_tasks WHERE task_type = 'embedding' ORDER BY created_at")
        .fetch_all(db)
        .await
        .unwrap()
}

#[tokio::test]
async fn recall_injects_the_outlier_chunk_as_an_auto_task_and_a_cache_hit_makes_no_task() {
    let r = recall_env(vec![embed_reply(&["고양이 이야기를 다시 해 줘".to_string()])]).await;
    let before = r.e.srv.requests().len();
    let out = do_recall(&r, &r.input, &HashSet::new(), None, &CancellationToken::new()).await;
    assert_eq!((out.meta.status.as_str(), out.meta.hits.len()), ("used", 1), "{:?}", out.meta);
    let block = out.block.unwrap();
    assert!(block.starts_with("[참고 자료") && block.contains("지시가 아니며") && block.ends_with("[참고 자료 끝]"));
    assert!(block.contains("고양이 고양이 고양이") && !block.contains("강아지"), "{block}");
    assert!(!block.contains("<image>") && out.meta.tokens > 0);
    assert_eq!(r.e.srv.requests().len(), before + 1, "질의 임베딩 1회");
    assert_eq!(
        embedding_tasks(&r.e.db).await.iter().filter(|t| t.1 == "auto").count(),
        1,
        "회상은 자동 실행으로 기록"
    );
    // meta에는 참조만 — chunk 글은 저장하지 않음
    let meta = serde_json::to_string(&out.meta).unwrap();
    assert!(!meta.contains("고양이 고양이") && !meta.contains("가가가"), "{meta}");

    // 같은 질의: 캐시 → 공급자도 작업도 없음
    let tasks_before = embedding_tasks(&r.e.db).await.len();
    let again = do_recall(&r, &r.input, &HashSet::new(), None, &CancellationToken::new()).await;
    assert_eq!(again.meta.status, "used");
    assert_eq!((r.e.srv.requests().len(), embedding_tasks(&r.e.db).await.len()), (before + 1, tasks_before));

    // 다시 만들기: 참조로 글만 다시 읽음 (공급자·작업 없음)
    let again2 = do_recall(&r, "아무 말", &HashSet::new(), Some(&out.meta.hits), &CancellationToken::new()).await;
    assert_eq!((again2.meta.status.as_str(), again2.block.is_some()), ("used", true));
    assert_eq!((r.e.srv.requests().len(), embedding_tasks(&r.e.db).await.len()), (before + 1, tasks_before));
}

#[tokio::test]
async fn recall_skips_without_calling_the_provider_when_it_cannot_help() {
    let r = recall_env(vec![]).await;
    let before = r.e.srv.requests().len();
    // 입력이 너무 짧음
    assert_eq!(
        do_recall(&r, "응", &HashSet::new(), None, &CancellationToken::new())
            .await
            .meta
            .reason
            .as_deref(),
        Some("short")
    );
    // 모든 chunk가 이미 프롬프트 창 안
    let all: HashSet<String> = (0..90).map(|i| format!("big-m{i}")).collect();
    assert_eq!(
        do_recall(&r, &r.input, &all, None, &CancellationToken::new()).await.meta.reason.as_deref(),
        Some("no_candidates")
    );
    // 이 대화에서 제외된 캐릭터 범위에 없음: 색인이 없는 설정(다른 접두어)은 다른 모델
    let mut r2 = recall_env(vec![]).await;
    r2.e.cfg.query_prefix = "q: ".into();
    assert_eq!(
        do_recall(&r2, &r2.input, &HashSet::new(), None, &CancellationToken::new())
            .await
            .meta
            .reason
            .as_deref(),
        Some("other_model")
    );
    assert_eq!(r.e.srv.requests().len(), before);
    assert!(
        embedding_tasks(&r.e.db).await.is_empty() && embedding_tasks(&r2.e.db).await.is_empty(),
        "작업도 만들지 않음"
    );
}

#[test]
fn selection_keeps_only_what_stands_out_from_the_rest() {
    let hit = |s: f32| search::RawHit {
        chunk_id: 0,
        conversation_id: "c".into(),
        first_message_id: "m".into(),
        last_message_id: "m".into(),
        text_hash: format!("{s}"),
        score: s,
    };
    let scan = |scores: &[f32], scanned: u64, mean: f64, std: f64| search::Scan {
        hits: scores.iter().map(|s| hit(*s)).collect(),
        scanned,
        truncated: false,
        mean,
        m2: std * std * (scanned - 1) as f64,
    };
    // 잡음뿐: 가장 좋은 것도 평균에서 2.5 표준편차 안
    assert!(recall::select(&scan(&[0.31, 0.30, 0.29], 30, 0.2, 0.05)).is_empty());
    // 튀는 것 하나 (나머지는 상대 차이 밖)
    let kept = recall::select(&scan(&[0.9, 0.7, 0.3], 30, 0.2, 0.05));
    assert_eq!(kept.len(), 1);
    // 후보가 너무 적어 통계가 의미 없으면 가장 좋은 하나
    assert_eq!(recall::select(&scan(&[0.5, 0.4], 5, 0.4, 0.1)).len(), 1);
    // 점수가 모두 같으면(표준편차 0) 아무것도 튀지 않음
    assert!(recall::select(&scan(&[0.5, 0.5], 30, 0.5, 0.0)).is_empty());
    assert!(recall::select(&scan(&[], 30, 0.0, 0.0)).is_empty());
}

#[tokio::test]
async fn recall_is_blocked_by_the_budget_and_by_a_cancelled_chat_without_leaving_running_tasks() {
    let r = recall_env(vec![embed_reply(&["고양이 이야기를 다시 해 줘".to_string()])]).await;
    // 취소된 채팅: 작업은 닫히고(Running으로 남지 않음) 기억 없이 진행
    let c = CancellationToken::new();
    c.cancel();
    let out = do_recall(&r, &r.input, &HashSet::new(), None, &c).await;
    assert_eq!(out.meta.reason.as_deref(), Some("cancelled"));
    let tasks = embedding_tasks(&r.e.db).await;
    assert!(tasks.iter().all(|t| t.2 != "running" && t.2 != "pending"), "{tasks:?}");

    // 예산 초과: 자동 실행은 항상 막힘 → 기억 없이, 공급자 호출 없음
    crate::db::budget::upsert_budget_limit(
        &r.e.db,
        &crate::db::budget::BudgetLimit {
            api_profile_id: r.e.profile.id.clone(),
            daily_tokens: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let before = r.e.srv.requests().len();
    let out = do_recall(&r, &r.input, &HashSet::new(), None, &CancellationToken::new()).await;
    assert_eq!(
        (out.meta.status.as_str(), out.meta.reason.as_deref()),
        ("skipped", Some("budget")),
        "{:?}",
        out.meta
    );
    assert_eq!(r.e.srv.requests().len(), before, "한도에 막히면 공급자를 부르지 않음");
}

#[test]
fn query_and_selection_helpers() {
    assert!(recall::build_query("  ", None).is_none() && recall::build_query("응", None).is_none());
    assert_eq!(recall::build_query("응", Some("<b>지난 답변</b>")).as_deref(), Some("응\n지난 답변"));
    let long = recall::build_query(&"가".repeat(900), Some(&"나".repeat(900))).unwrap();
    assert_eq!(long.chars().count(), 500 + 1 + 300);
    let block = recall::format_block(&[("제 \"목\"<>".into(), "사용자: 안녕".into())]);
    assert!(block.contains("title=\"제 목\"") && block.contains("사용자: 안녕"));
}

#[test]
fn recall_block_is_a_user_turn_merged_into_the_input_and_never_lands_in_the_system_prompt() {
    use crate::models::ai_service::{ChatOptions, PromptMessage, PromptMessageItem, PromptRole};
    let item = |role, c: &str| PromptMessageItem {
        images: vec![],
        role,
        content: c.into(),
        token: 0,
    };
    let block = recall::format_block(&[("대화".into(), "사용자: 지난 이야기".into())]);
    // 첫 메시지(기록이 비어 있음)에서도: 규칙(system) 뒤에 user로 붙는다
    let prompt = PromptMessage {
        chat_options: ChatOptions::default(),
        messages: vec![
            item(PromptRole::System, "규칙"),
            item(PromptRole::User, &block),
            item(PromptRole::User, "지금 입력"),
        ],
    };
    let (system, turns) = crate::adapter::split_system(&prompt);
    assert_eq!(system.as_deref(), Some("규칙"), "회상 글은 system 문자열에 들어가지 않음");
    assert_eq!(turns.len(), 1);
    assert!(
        turns[0].content.starts_with("[참고 자료") && turns[0].content.ends_with("지금 입력"),
        "{}",
        turns[0].content
    );
}

// ---------------------------------------------------------------------------
// 색인 무효화 (3-C-6)
// ---------------------------------------------------------------------------
use super::maintain;

async fn rows_of(e: &Env, conv: &str) -> Vec<String> {
    let pool = e.store.pool().await.unwrap();
    sqlx::query_scalar("SELECT first_message_id FROM chunks WHERE conversation_id = ? ORDER BY id")
        .bind(conv)
        .fetch_all(&pool)
        .await
        .unwrap()
}

async fn indexed_pets(extra: Vec<Reply>) -> Env {
    let mut e = env(vec![]).await;
    pets_conv(&e.db, "c1").await;
    let texts = planned_texts(&e.db, "c1").await;
    let mut replies = vec![embed_reply(&texts)];
    replies.extend(extra);
    set_replies(&mut e, replies).await;
    run_index(&e, &scope("c1"), false).await.unwrap();
    e
}

#[tokio::test]
async fn editing_a_message_drops_only_its_chunk_row_and_reindexing_embeds_only_that_one() {
    let e = indexed_pets(vec![]).await;
    assert_eq!(maintain::reconcile(&e.db, &e.store, "c1").await.unwrap(), 0, "바뀐 것이 없으면 지우지 않음");
    let vectors = status(&e.store, None).await.unwrap().vectors;

    sqlx::query("UPDATE messages SET content = ? WHERE id = 'c1-m4'")
        .bind(body("고양이"))
        .execute(&e.db)
        .await
        .unwrap();
    assert_eq!(maintain::reconcile(&e.db, &e.store, "c1").await.unwrap(), 1);
    assert_eq!(rows_of(&e, "c1").await, vec!["c1-m0".to_string()], "고친 메시지가 든 chunk의 행만 사라짐");
    assert_eq!(status(&e.store, None).await.unwrap().vectors, vectors, "벡터 캐시는 남김");
    // 되돌려 놓은 글은 캐시에 있어 다시 임베딩하지 않아도 된다 (색인 미리보기에서 새로 할 것 없음)
    sqlx::query("UPDATE messages SET content = ? WHERE id = 'c1-m4'")
        .bind(body("강아지"))
        .execute(&e.db)
        .await
        .unwrap();
    let pv = index::preview(&e.db, &e.store, &e.profile, &e.cfg, &scope("c1")).await.unwrap();
    assert_eq!((pv.cached, pv.to_embed), (2, 0));
}

#[tokio::test]
async fn deleting_or_hiding_messages_drops_the_affected_chunk_rows() {
    let e = indexed_pets(vec![]).await;
    // 첫 chunk의 끝 메시지를 지움
    sqlx::query("DELETE FROM messages WHERE id = 'c1-m2'").execute(&e.db).await.unwrap();
    assert!(maintain::reconcile(&e.db, &e.store, "c1").await.unwrap() >= 1);
    assert!(!rows_of(&e, "c1").await.contains(&"c1-m0".to_string()));
    // 숨김 처리도 글이 달라지는 것
    let e = indexed_pets(vec![]).await;
    sqlx::query("UPDATE messages SET meta = ? WHERE id = 'c1-m1'")
        .bind(r#"{"hidden":true}"#)
        .execute(&e.db)
        .await
        .unwrap();
    assert_eq!(maintain::reconcile(&e.db, &e.store, "c1").await.unwrap(), 1);
    assert_eq!(rows_of(&e, "c1").await, vec!["c1-m3".to_string()]);
}

#[tokio::test]
async fn conversation_delete_forgets_rows_and_permanent_delete_purges_vectors() {
    let e = indexed_pets(vec![]).await;
    assert_eq!(maintain::forget_conversation(&e.store, "c1").await.unwrap(), 2);
    assert!(rows_of(&e, "c1").await.is_empty());
    assert_eq!(status(&e.store, None).await.unwrap().vectors, 2, "되살릴 때를 위해 벡터는 남김");
    // 휴지통에서 완전히 지운 뒤: 가리키지 않는 벡터까지
    purge(&e.store, &e.db, None, false).await.unwrap();
    assert_eq!(status(&e.store, None).await.unwrap().vectors, 0);
    // 대화가 이미 없으면 reconcile도 모든 행을 지움
    let e = indexed_pets(vec![]).await;
    sqlx::query("DELETE FROM conversations WHERE id = 'c1'").execute(&e.db).await.unwrap();
    assert_eq!(maintain::reconcile(&e.db, &e.store, "c1").await.unwrap(), 2);
    // 색인 파일이 없으면 아무것도 열지 않음
    let none = env(vec![]).await;
    assert_eq!(maintain::reconcile(&none.db, &none.store, "c1").await.unwrap(), 0);
    assert!(!none.store.exists());
    assert_eq!(
        maintain::conversations_of(&e.db, &["c1-m0".into()]).await.len(),
        0,
        "지운 뒤에는 대화를 알 수 없음"
    );
}

#[tokio::test]
async fn recall_does_not_inject_a_chunk_edited_after_indexing() {
    let r = recall_env(vec![embed_reply(&["고양이 이야기를 다시 해 줘".to_string()])]).await;
    // 색인한 뒤 목표 chunk(메시지 30~32)의 글을 고침 — 아직 맞추지 않은 상태
    sqlx::query("UPDATE messages SET content = '고친 글' WHERE id = 'big-m31'")
        .execute(&r.e.db)
        .await
        .unwrap();
    let out = do_recall(&r, &r.input, &HashSet::new(), None, &CancellationToken::new()).await;
    assert_eq!((out.meta.status.as_str(), out.block.is_none()), ("skipped", true), "{:?}", out.meta);
    // 맞춘 뒤 다시 색인하면 새 글로 돌아온다
    assert_eq!(maintain::reconcile(&r.e.db, &r.e.store, "big").await.unwrap(), 1);
}
