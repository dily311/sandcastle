//! 자동 재생성 테스트 (결정·지시·묶음 확인·가장 나은 스와이프 고르기). 사슬 전체(작업 제출·취소)는 앱 핸들이 있어야 해서 실제 앱에서 확인한다.
use serde_json::json;

use super::*;

fn cfg(max: u32, banned: bool, stuck: bool) -> RewriteSetting {
    RewriteSetting {
        max_attempts: max,
        banned,
        stuck,
    }
}

fn finding(rule: &str, fixed: bool, detail: &str) -> Value {
    json!({ "rule": rule, "level": "warn", "text": "x", "fixed": fixed, "detail": detail })
}

fn lint(findings: Vec<Value>) -> Value {
    json!({ "content": "글", "findings": findings, "fixedRules": [], "at": 1 })
}

#[test]
fn only_unfixed_banned_and_stuck_findings_the_user_enabled_can_trigger() {
    let l = lint(vec![finding("banned", false, "알량한\u{1f}헤드라이트 사슴"), finding("stuck", false, "그래서")]);
    let p = decide(&cfg(1, true, true), Some(&l), 0).unwrap();
    assert_eq!((p.attempt, p.max, p.rules.clone()), (1, 1, vec!["banned".to_string(), "stuck".to_string()]));
    assert!(
        p.instruction.contains("「알량한」, 「헤드라이트 사슴」") && p.instruction.contains("「그래서」"),
        "{}",
        p.instruction
    );
    assert!(!p.instruction.contains("원문"), "결함이 있는 글은 넣지 않음");
    // 규칙별로 사용자가 고른다
    assert_eq!(decide(&cfg(1, true, false), Some(&l), 0).unwrap().rules, vec!["banned".to_string()]);
    assert_eq!(decide(&cfg(1, false, true), Some(&l), 0).unwrap().rules, vec!["stuck".to_string()]);
    assert!(decide(&cfg(1, false, false), Some(&l), 0).is_none());
    // 무료 자동 고침이 이미 고친 것, 다른 규칙(반복 문단·대사 대신 쓰기·글자 수·닫힘)은 방아쇠가 아니다
    let other = lint(vec![
        finding("banned", true, "알량한"),
        finding("repeat", false, ""),
        finding("impersonate", false, ""),
        finding("length", false, ""),
        finding("unclosed", false, ""),
    ]);
    assert!(decide(&cfg(3, true, true), Some(&other), 0).is_none());
    assert!(decide(&cfg(1, true, true), None, 0).is_none() && decide(&cfg(1, true, true), Some(&lint(vec![])), 0).is_none());
}

#[test]
fn attempts_are_capped_by_the_setting_and_by_the_hard_limit() {
    let l = lint(vec![finding("stuck", false, "그래서")]);
    // 기본 1번: 이미 1번 했으면 더 하지 않음
    assert!(decide(&cfg(1, true, true), Some(&l), 0).is_some() && decide(&cfg(1, true, true), Some(&l), 1).is_none());
    assert_eq!(decide(&cfg(2, true, true), Some(&l), 1).unwrap().attempt, 2);
    assert!(decide(&cfg(2, true, true), Some(&l), 2).is_none());
    // 설정이 아무리 커도 3번까지 (한 번마다 응답 하나의 요금), 0이나 비정상 값도 1번으로
    assert_eq!(max_attempts(&cfg(99, true, true)), HARD_CAP);
    assert!(decide(&cfg(99, true, true), Some(&l), HARD_CAP).is_none());
    assert_eq!(max_attempts(&cfg(0, true, true)), 1);
    assert_eq!(RewriteSetting::default().max_attempts, 1, "기본은 1번");
}

#[test]
fn a_chain_is_bound_to_one_message_and_swipe() {
    let ctx = AutoCtx {
        message_id: "m1".into(),
        swipe_id: 1,
        attempt: 1,
        max: 1,
        rules: vec![],
    };
    let msg = |id: &str, meta: Option<Value>| Message {
        id: id.into(),
        conversation_id: "c".into(),
        seq: 1,
        role: "assistant".into(),
        name: None,
        content: String::new(),
        character_id: None,
        meta,
        created_at: 0,
        updated_at: 0,
    };
    assert!(ctx.matches(&msg("m1", Some(json!({ "swipeId": 1 })))));
    assert!(!ctx.matches(&msg("m2", Some(json!({ "swipeId": 1 })))), "다른 메시지");
    assert!(!ctx.matches(&msg("m1", Some(json!({ "swipeId": 2 })))), "사용자가 스와이프를 넘김");
    assert!(!ctx.matches(&msg("m1", None)), "스와이프 0");
    // 작업 payload는 왕복하고 시도 횟수는 payload에만 있다
    let v = serde_json::to_value(&ctx).unwrap();
    assert_eq!(serde_json::from_value::<AutoCtx>(v).unwrap(), ctx);
}

async fn db_with_message(meta: Value) -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db = crate::db::conn::init_pool(dir.path().join("a")).await.unwrap();
    sqlx::query("INSERT INTO conversations (id, title) VALUES ('c', 't')")
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO messages (id, conversation_id, seq, role, content, meta) VALUES ('m', 'c', 1, 'assistant', '마지막 글', ?)")
        .bind(meta.to_string())
        .execute(&db)
        .await
        .unwrap();
    (db, dir)
}

#[tokio::test]
async fn when_attempts_run_out_the_swipe_with_the_fewest_hits_wins_and_ties_go_to_the_latest() {
    let bad = |n: usize| lint((0..n).map(|_| finding("banned", false, "알량한")).collect());
    let swipe = |content: &str, hits: usize| json!({ "content": content, "meta": { "info": { "model": content }, "images": [], "lint": bad(hits) } });
    // 원본(2건) → 다시 쓰기 1(1건) → 다시 쓰기 2(1건, 선택되어 있음): 가장 나은 것 중 나중 것이 남는다
    let meta = json!({ "swipeId": 2, "swipes": [swipe("원본", 2), swipe("고침1", 1), swipe("고침2", 1)], "lint": bad(1), "info": { "model": "고침2" } });
    let (db, _d) = db_with_message(meta).await;
    assert_eq!(finish_exhausted(&db, "m", &cfg(2, true, true), 2).await.unwrap(), Some(2));
    let (content, raw): (String, String) = sqlx::query_as("SELECT content, meta FROM messages WHERE id = 'm'")
        .fetch_one(&db)
        .await
        .unwrap();
    let m: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        (content.as_str(), m["swipeId"].as_i64(), m["rewriteNote"]["status"].as_str()),
        ("마지막 글", Some(2), Some("exhausted")),
        "{m}"
    );

    // 마지막 시도가 원본보다 나쁘면(3건) 가장 나은 스와이프로 되돌아가고 그 스와이프의 info·lint가 위로 올라온다
    let meta = json!({ "swipeId": 1, "swipes": [swipe("원본", 1), swipe("더 나쁨", 3)], "lint": bad(3), "info": { "model": "더 나쁨" } });
    let (db, _d) = db_with_message(meta).await;
    assert_eq!(finish_exhausted(&db, "m", &cfg(1, true, true), 1).await.unwrap(), Some(0));
    let (content, raw): (String, String) = sqlx::query_as("SELECT content, meta FROM messages WHERE id = 'm'")
        .fetch_one(&db)
        .await
        .unwrap();
    let m: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        (content.as_str(), m["swipeId"].as_i64(), m["info"]["model"].as_str()),
        ("원본", Some(0), Some("원본"))
    );
    assert_eq!(m["lint"]["findings"].as_array().unwrap().len(), 1);
    // 스와이프가 없으면(방금 만든 한 줄) 메모만 남긴다
    let (db, _d) = db_with_message(json!({ "lint": bad(1) })).await;
    assert_eq!(finish_exhausted(&db, "m", &cfg(1, true, true), 1).await.unwrap(), Some(0));
}

#[test]
fn pending_signal_is_read_from_the_response_info() {
    let p = Pending {
        attempt: 1,
        max: 2,
        rules: vec!["stuck".into()],
        instruction: "x".into(),
    };
    let mut m = Message {
        id: "m".into(),
        conversation_id: "c".into(),
        seq: 1,
        role: "assistant".into(),
        name: None,
        content: String::new(),
        character_id: None,
        meta: Some(json!({ "info": { "rewritePending": p } })),
        created_at: 0,
        updated_at: 0,
    };
    assert_eq!(pending_of(&m), Some(p));
    m.meta = Some(json!({ "info": { "rewritePending": null } }));
    assert!(pending_of(&m).is_none());
    m.meta = None;
    assert!(pending_of(&m).is_none());
}
