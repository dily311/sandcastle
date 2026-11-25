use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

use crate::{
    db::ai_service as db,
    models::ai_service::{PromptBlocksInput, PromptPresetsInput},
};

fn block(id: Option<&str>, identifier: &str) -> PromptBlocksInput {
    PromptBlocksInput {
        id: id.map(str::to_string),
        preset_id: String::new(),
        name: identifier.into(),
        identifier: identifier.into(),
        block_type: "text".into(),
        role: "system".into(),
        content: format!("{identifier} content"),
        enabled: true,
        position: "relative".into(),
        depth: 0,
        sort_order: 99,
        triggers_json: json!(["continue"]),
        extra_json: None,
    }
}

#[tokio::test]
async fn replace_blocks_keeps_order_ids_and_triggers() {
    let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
    let id = db::upsert_prompt_preset(
        &pool,
        PromptPresetsInput {
            id: None,
            name: "p".into(),
            description: None,
            preset_type: "chat_completion".into(),
            source_format: "native".into(),
            config_json: json!({}),
            settings_json: None,
            compatibility_json: None,
        },
    )
    .await
    .unwrap();

    let mut conn = pool.acquire().await.unwrap();
    db::replace_prompt_blocks(
        &mut conn,
        &id,
        vec![block(Some("a"), "main"), block(Some("b"), "chatHistory"), block(Some("c"), "jailbreak")],
    )
    .await
    .unwrap();
    // c 삭제, 순서 바꿈, 새 블록 추가
    db::replace_prompt_blocks(
        &mut conn,
        &id,
        vec![block(Some("b"), "chatHistory"), block(None, "note"), block(Some("a"), "main")],
    )
    .await
    .unwrap();
    drop(conn);

    let blocks = db::get_prompt_blocks_by_preset(&pool, &id).await.unwrap();
    let ids: Vec<_> = blocks.iter().map(|b| b.identifier.as_str()).collect();
    assert_eq!(ids, vec!["chatHistory", "note", "main"]);
    assert_eq!(blocks[0].id, "b");
    assert_eq!(blocks[2].id, "a");
    assert_eq!(blocks[1].triggers_json, json!(["continue"]), "트리거가 이중 인코딩 없이 저장됨");
    assert!(blocks[1].extra_json.is_none());
    let preset = db::get_prompt_preset(&pool, &id).await.unwrap();
    assert!(preset.settings_json.is_none(), "빈 설정은 NULL");
}

#[tokio::test]
async fn default_preset_is_seeded_and_builds() {
    use crate::service::prompt::prompt_manager::PromptManager;
    let pool = sqlx::sqlite::SqlitePoolOptions::new().connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
    let p = PromptManager::load_preset(&pool, Some("default")).await.unwrap().expect("기본 프리셋");
    assert_eq!(p.preset.name, "Default");
    assert_eq!(p.blocks.len(), 8);
    assert!(p.blocks.iter().any(|b| b.identifier == "main" && b.content.contains("{{char}}'s next reply")));
    // 유틸 지시문 자리 (5-F-7-1, ADR-030 추가): 대화 기록 바로 앞
    let order = |id: &str| p.blocks.iter().find(|b| b.identifier == id).map(|b| b.sort_order).unwrap();
    assert!(p
        .blocks
        .iter()
        .any(|b| b.identifier == "utils" && b.content.contains("{{vars}}") && b.content.contains("{{img_inprompt}}")));
    assert_eq!((order("utils") + 1, order("chatHistory") + 1), (order("chatHistory"), order("postHistory")));
}
