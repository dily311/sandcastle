//! 고치기 제안 테스트 (CLAUDE.md 6-D-3)
use serde_json::{json, Map, Value};
use sqlx::SqlitePool;

use super::*;
use crate::db;

async fn setup() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let db = db::conn::init_pool(dir.path().join("appdata")).await.unwrap();
    for sql in [
        r#"INSERT INTO characters (id, name, description, tags) VALUES ('c1', 'Mira', 'old desc', '["a"]')"#,
        "INSERT INTO personas (id, name, description) VALUES ('p1', 'Me', 'me')",
        "INSERT INTO worldinfo_books (id, name, description) VALUES ('b1', 'World', '')",
        r#"INSERT INTO worldinfo_entries (id, book_id, keys, secondary_keys, content, comment) VALUES ('e1', 'b1', '["forest"]', '[]', 'A forest.', 'forest')"#,
        "INSERT INTO conversations (id, title, character_id) VALUES ('v1', 'Chat', 'c1')",
        r#"INSERT INTO messages (id, conversation_id, seq, role, content) VALUES ('m1', 'v1', 1, 'assistant', 'Hello "there')"#,
    ] {
        sqlx::query(sql).execute(&db).await.unwrap();
    }
    (dir, db)
}

async fn prop(db: &SqlitePool, kind: ProposalKind, op: &str, target: Option<&str>, parent: Option<&str>, changes: Value) -> Result<ChangeProposal> {
    let changes: Map<String, Value> = changes.as_object().unwrap().clone();
    propose(
        db,
        NewProposal {
            source: "plugin:test",
            kind,
            op,
            target_id: target,
            parent_id: parent,
            changes: &changes,
            reason: "because",
        },
    )
    .await
}

async fn desc(db: &SqlitePool) -> String {
    sqlx::query_scalar("SELECT description FROM characters WHERE id = 'c1'")
        .fetch_one(db)
        .await
        .unwrap()
}

#[tokio::test]
async fn propose_does_not_touch_original_then_apply_and_revert() {
    let (_d, db) = setup().await;
    let p = prop(
        &db,
        ProposalKind::Character,
        "update",
        Some("c1"),
        None,
        json!({ "description": "new desc", "name": "Mira" }),
    )
    .await
    .unwrap();
    assert_eq!(p.status, "pending");
    assert_eq!(p.target_label, "Mira");
    assert_eq!(p.fields.len(), 1, "지금과 같은 칸(name)은 빠짐");
    assert_eq!(p.fields[0].before, json!("old desc"));
    assert_eq!(desc(&db).await, "old desc", "제안만으로는 원본이 바뀌지 않음");

    let applied = apply(&db, &p.id).await.unwrap();
    assert_eq!(applied.status, "applied");
    assert!(applied.can_revert);
    assert_eq!(desc(&db).await, "new desc");
    assert!(apply(&db, &p.id).await.is_err(), "두 번 적용하지 않음");

    let reverted = revert(&db, &p.id).await.unwrap();
    assert_eq!(reverted.status, "reverted");
    assert_eq!(desc(&db).await, "old desc");
}

#[tokio::test]
async fn same_pending_proposal_is_not_duplicated() {
    let (_d, db) = setup().await;
    let a = prop(&db, ProposalKind::Character, "update", Some("c1"), None, json!({ "description": "same" }))
        .await
        .unwrap();
    let b = prop(&db, ProposalKind::Character, "update", Some("c1"), None, json!({ "description": "same" }))
        .await
        .unwrap();
    assert_eq!(a.id, b.id, "같은 제안은 하나만");
    let c = prop(&db, ProposalKind::Character, "update", Some("c1"), None, json!({ "description": "other" }))
        .await
        .unwrap();
    assert_ne!(a.id, c.id);
    assert_eq!(list(&db, Some("pending")).await.unwrap().len(), 2);
}

#[tokio::test]
async fn refuses_when_original_changed() {
    let (_d, db) = setup().await;
    let p = prop(&db, ProposalKind::Character, "update", Some("c1"), None, json!({ "description": "plugin" }))
        .await
        .unwrap();
    sqlx::query("UPDATE characters SET description = 'user edit' WHERE id = 'c1'")
        .execute(&db)
        .await
        .unwrap();
    assert!(get(&db, &p.id).await.unwrap().outdated);
    let err = apply(&db, &p.id).await.unwrap_err().to_string();
    assert!(err.contains("바뀌어"), "{err}");
    assert_eq!(desc(&db).await, "user edit", "사용자가 고친 것을 덮어쓰지 않음");
    assert_eq!(reject(&db, &p.id).await.unwrap().status, "rejected");

    // 적용한 뒤 사용자가 또 고치면 되돌리지 않음
    let p = prop(&db, ProposalKind::Character, "update", Some("c1"), None, json!({ "description": "plugin2" }))
        .await
        .unwrap();
    apply(&db, &p.id).await.unwrap();
    sqlx::query("UPDATE characters SET description = 'user again' WHERE id = 'c1'")
        .execute(&db)
        .await
        .unwrap();
    assert!(!get(&db, &p.id).await.unwrap().can_revert);
    assert!(revert(&db, &p.id).await.is_err());
    assert_eq!(desc(&db).await, "user again");
}

#[tokio::test]
async fn validates_fields_and_kinds() {
    let (_d, db) = setup().await;
    // 정해 둔 칸만
    assert!(prop(&db, ProposalKind::Character, "update", Some("c1"), None, json!({ "avatar": "C:/x.png" }))
        .await
        .is_err());
    assert!(prop(&db, ProposalKind::Character, "update", Some("c1"), None, json!({ "id": "c2" }))
        .await
        .is_err());
    // 모양
    assert!(prop(&db, ProposalKind::Character, "update", Some("c1"), None, json!({ "tags": "a,b" }))
        .await
        .is_err());
    assert!(prop(&db, ProposalKind::LorebookEntry, "update", Some("e1"), None, json!({ "enabled": "yes" }))
        .await
        .is_err());
    assert!(prop(
        &db,
        ProposalKind::Character,
        "update",
        Some("c1"),
        None,
        json!({ "description": "x".repeat(MAX_TEXT + 1) })
    )
    .await
    .is_err());
    // 없는 항목, 같은 값만, 새로 만들 수 없는 종류, 모르는 op
    assert!(prop(&db, ProposalKind::Character, "update", Some("nope"), None, json!({ "name": "x" }))
        .await
        .is_err());
    assert!(prop(&db, ProposalKind::Character, "update", Some("c1"), None, json!({ "name": "Mira" }))
        .await
        .is_err());
    assert!(prop(&db, ProposalKind::Character, "create", None, None, json!({ "name": "x" })).await.is_err());
    assert!(prop(&db, ProposalKind::Message, "create", None, Some("v1"), json!({ "content": "x" }))
        .await
        .is_err());
    assert!(prop(&db, ProposalKind::LorebookEntry, "create", None, Some("nope"), json!({ "content": "x" }))
        .await
        .is_err());
    assert!(prop(&db, ProposalKind::Character, "delete", Some("c1"), None, json!({ "name": "x" }))
        .await
        .is_err());

    // 기다리는 제안 상한
    for i in 0..MAX_PENDING {
        prop(
            &db,
            ProposalKind::Persona,
            "update",
            Some("p1"),
            None,
            json!({ "description": format!("d{i}") }),
        )
        .await
        .unwrap();
    }
    let err = prop(&db, ProposalKind::Persona, "update", Some("p1"), None, json!({ "description": "over" }))
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("너무 많음"), "{err}");
}

#[tokio::test]
async fn lorebook_entry_create_and_message_edit() {
    let (_d, db) = setup().await;
    let p = prop(
        &db,
        ProposalKind::LorebookEntry,
        "create",
        None,
        Some("b1"),
        json!({ "keys": ["river"], "content": "A river.", "comment": "river" }),
    )
    .await
    .unwrap();
    assert_eq!(p.target_label, "river");
    let applied = apply(&db, &p.id).await.unwrap();
    let new_id = applied.target_id.clone().unwrap();
    let (keys, content, book): (String, String, String) = sqlx::query_as("SELECT keys, content, book_id FROM worldinfo_entries WHERE id = ?")
        .bind(&new_id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!((keys.as_str(), content.as_str(), book.as_str()), (r#"["river"]"#, "A river.", "b1"));
    revert(&db, &p.id).await.unwrap();
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM worldinfo_entries WHERE id = ?")
        .bind(&new_id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(left, 0, "되돌리면 만든 항목을 지움");

    // 항목 키워드·켜짐 고치기
    let p = prop(
        &db,
        ProposalKind::LorebookEntry,
        "update",
        Some("e1"),
        None,
        json!({ "keys": ["forest", "woods"], "enabled": false }),
    )
    .await
    .unwrap();
    apply(&db, &p.id).await.unwrap();
    let (keys, enabled): (String, i64) = sqlx::query_as("SELECT keys, enabled FROM worldinfo_entries WHERE id = 'e1'")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!((keys.as_str(), enabled), (r#"["forest","woods"]"#, 0));

    // 메시지 글 고치기
    let p = prop(&db, ProposalKind::Message, "update", Some("m1"), None, json!({ "content": "Hello \"there\"" }))
        .await
        .unwrap();
    assert_eq!(p.fields[0].before, json!("Hello \"there"));
    apply(&db, &p.id).await.unwrap();
    let c: String = sqlx::query_scalar("SELECT content FROM messages WHERE id = 'm1'").fetch_one(&db).await.unwrap();
    assert_eq!(c, "Hello \"there\"");

    assert!(list(&db, Some("pending")).await.unwrap().is_empty());
    assert_eq!(list(&db, None).await.unwrap().len(), 3);
}
