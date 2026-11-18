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
    search::clear_cache();
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
