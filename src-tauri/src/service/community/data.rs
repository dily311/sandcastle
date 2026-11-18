//! 플러그인의 데이터 읽기 (`data.read:<종류>`, CLAUDE.md 6-D-3, ADR-012).
//!
//! 플러그인에 주는 것은 글 칸만: 파일 경로(아바타·커버), meta, 설정 연결(프리셋·API), 다른 내부 값은 주지 않는다.
//! 한 번에 주는 양에 상한을 둔다.
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::error::{AppError, Result};

const MAX_LIST: i64 = 500;
const MAX_MESSAGES: i64 = 200;

fn arr(raw: Option<String>) -> Value {
    raw.and_then(|r| serde_json::from_str::<Value>(&r).ok())
        .filter(Value::is_array)
        .unwrap_or_else(|| json!([]))
}

fn need_id(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| AppError::validation(format!("{key}가 필요함"), None))
}

fn not_found(what: &str, id: &str) -> AppError {
    AppError::not_found(format!("{what}이(가) 없음: {id}"))
}

pub async fn character_list(db: &SqlitePool) -> Result<Value> {
    let rows = sqlx::query("SELECT id, name, tags, category FROM characters ORDER BY name LIMIT ?")
        .bind(MAX_LIST)
        .fetch_all(db)
        .await?;
    Ok(rows
        .iter()
        .map(|r| json!({ "id": r.get::<String, _>("id"), "name": r.get::<String, _>("name"), "tags": arr(r.get("tags")), "category": r.get::<Option<String>, _>("category") }))
        .collect())
}

pub async fn character_get(db: &SqlitePool, args: &Value) -> Result<Value> {
    let id = need_id(args, "id")?;
    let r = sqlx::query(
        "SELECT id, name, description, first_message, example_dialogue, creator_notes, system_prompt_override, tags, category FROM characters WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| not_found("캐릭터", &id))?;
    Ok(json!({
        "id": r.get::<String, _>("id"),
        "name": r.get::<String, _>("name"),
        "description": r.get::<Option<String>, _>("description").unwrap_or_default(),
        "firstMessage": r.get::<Option<String>, _>("first_message").unwrap_or_default(),
        "exampleDialogue": r.get::<Option<String>, _>("example_dialogue").unwrap_or_default(),
        "creatorNotes": r.get::<Option<String>, _>("creator_notes").unwrap_or_default(),
        "systemPromptOverride": r.get::<Option<String>, _>("system_prompt_override").unwrap_or_default(),
        "tags": arr(r.get("tags")),
        "category": r.get::<Option<String>, _>("category"),
    }))
}

pub async fn persona_list(db: &SqlitePool) -> Result<Value> {
    let rows = sqlx::query("SELECT id, name, tags FROM personas ORDER BY name LIMIT ?")
        .bind(MAX_LIST)
        .fetch_all(db)
        .await?;
    Ok(rows
        .iter()
        .map(|r| json!({ "id": r.get::<String, _>("id"), "name": r.get::<String, _>("name"), "tags": arr(r.get("tags")) }))
        .collect())
}

pub async fn persona_get(db: &SqlitePool, args: &Value) -> Result<Value> {
    let id = need_id(args, "id")?;
    let r = sqlx::query("SELECT id, name, description, tags FROM personas WHERE id = ?")
        .bind(&id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| not_found("페르소나", &id))?;
    Ok(json!({
        "id": r.get::<String, _>("id"),
        "name": r.get::<String, _>("name"),
        "description": r.get::<Option<String>, _>("description").unwrap_or_default(),
        "tags": arr(r.get("tags")),
    }))
}

pub async fn lorebook_list(db: &SqlitePool) -> Result<Value> {
    let rows = sqlx::query(
        "SELECT b.id, b.name, b.description, (SELECT COUNT(*) FROM worldinfo_entries e WHERE e.book_id = b.id) AS n FROM worldinfo_books b ORDER BY b.name LIMIT ?",
    )
    .bind(MAX_LIST)
    .fetch_all(db)
    .await?;
    Ok(rows
        .iter()
        .map(|r| {
            json!({ "id": r.get::<String, _>("id"), "name": r.get::<String, _>("name"), "description": r.get::<Option<String>, _>("description").unwrap_or_default(), "entries": r.get::<i64, _>("n") })
        })
        .collect())
}

pub async fn lorebook_get(db: &SqlitePool, args: &Value) -> Result<Value> {
    let id = need_id(args, "id")?;
    let b = sqlx::query("SELECT id, name, description FROM worldinfo_books WHERE id = ?")
        .bind(&id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| not_found("로어북", &id))?;
    let entries = sqlx::query(
        "SELECT id, keys, secondary_keys, content, comment, enabled, priority, constant FROM worldinfo_entries WHERE book_id = ? ORDER BY priority DESC, id LIMIT 2000",
    )
    .bind(&id)
    .fetch_all(db)
    .await?;
    Ok(json!({
        "id": b.get::<String, _>("id"),
        "name": b.get::<String, _>("name"),
        "description": b.get::<Option<String>, _>("description").unwrap_or_default(),
        "entries": entries.iter().map(|e| json!({
            "id": e.get::<String, _>("id"),
            "keys": arr(e.get("keys")),
            "secondaryKeys": arr(e.get("secondary_keys")),
            "content": e.get::<Option<String>, _>("content").unwrap_or_default(),
            "comment": e.get::<Option<String>, _>("comment").unwrap_or_default(),
            "enabled": e.get::<Option<i64>, _>("enabled").unwrap_or(1) != 0,
            "priority": e.get::<Option<i64>, _>("priority").unwrap_or(0),
            "constant": e.get::<Option<i64>, _>("constant").unwrap_or(0) != 0,
        })).collect::<Vec<_>>(),
    }))
}

/// 대화 목록 (`characterId`로 거르기)
pub async fn chat_list(db: &SqlitePool, args: &Value) -> Result<Value> {
    let character = args.get("characterId").and_then(Value::as_str);
    let rows = sqlx::query(
        "SELECT id, title, character_id, persona_id, updated_at FROM conversations WHERE (?1 IS NULL OR character_id = ?1) ORDER BY updated_at DESC LIMIT ?2",
    )
    .bind(character)
    .bind(MAX_LIST)
    .fetch_all(db)
    .await?;
    Ok(rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<String, _>("id"),
                "title": r.get::<Option<String>, _>("title").unwrap_or_default(),
                "characterId": r.get::<Option<String>, _>("character_id"),
                "personaId": r.get::<Option<String>, _>("persona_id"),
                "updatedAt": r.get::<Option<i64>, _>("updated_at"),
            })
        })
        .collect())
}

/// 대화의 메시지 (오래된 것부터). `limit`(최대 200)개, `beforeSeq`를 주면 그 앞의 것
pub async fn chat_messages(db: &SqlitePool, args: &Value) -> Result<Value> {
    let id = need_id(args, "conversationId")?;
    let limit = args.get("limit").and_then(Value::as_i64).unwrap_or(50).clamp(1, MAX_MESSAGES);
    let before = args.get("beforeSeq").and_then(Value::as_i64);
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM conversations WHERE id = ?")
        .bind(&id)
        .fetch_optional(db)
        .await?;
    if exists.is_none() {
        return Err(not_found("대화", &id));
    }
    let mut rows = sqlx::query(
        "SELECT id, seq, role, name, content, created_at FROM messages WHERE conversation_id = ?1 AND (?2 IS NULL OR seq < ?2) ORDER BY seq DESC LIMIT ?3",
    )
    .bind(&id)
    .bind(before)
    .bind(limit)
    .fetch_all(db)
    .await?;
    rows.reverse();
    Ok(rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<String, _>("id"),
                "seq": r.get::<i64, _>("seq"),
                "role": r.get::<String, _>("role"),
                "name": r.get::<Option<String>, _>("name"),
                "content": r.get::<Option<String>, _>("content").unwrap_or_default(),
                "createdAt": r.get::<Option<i64>, _>("created_at"),
            })
        })
        .collect())
}
