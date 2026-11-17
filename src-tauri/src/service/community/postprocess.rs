//! 메시지 후처리 결과 저장 (CLAUDE.md 6-D-4, ADR-012).
//!
//! 순서: 응답 저장 → 정규식(백엔드) → 플러그인(화면의 호스트가 설치 순서로 부름) → 여기서 저장.
//! - 글은 "자동 고침"으로 고른 플러그인만 바꿀 수 있다. "표시만"인 플러그인은 메모(notes)만 남긴다.
//! - 원문은 meta.pluginProcess.original에 남긴다(여러 번 고쳐도 처음 원문). "원문 보기 / 되돌리기"가 이것을 쓴다.
//! - 플러그인이 일하는 사이 사용자가 메시지를 고쳤으면(지금 글 ≠ 시작할 때 글) 저장하지 않는다.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;

use super::runtime;
use crate::error::{AppError, Result};

const MAX_CONTENT: usize = 200_000;
const MAX_NOTES: usize = 20;
const MAX_NOTE: usize = 500;

pub const MODE_NOTES: &str = "notes";
pub const MODE_FIX: &str = "fix";

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginNote {
    /// info / warn
    pub level: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PostProcessResult {
    pub plugin_id: String,
    /// 이 플러그인이 글을 바꿨나
    pub changed: bool,
    pub notes: Vec<PluginNote>,
}

async fn modes(db: &SqlitePool) -> Result<std::collections::HashMap<String, String>> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT id, process_mode FROM user_plugins").fetch_all(db).await?;
    Ok(rows.into_iter().collect())
}

pub async fn set_mode(db: &SqlitePool, id: &str, mode: &str) -> Result<()> {
    if mode != MODE_NOTES && mode != MODE_FIX {
        return Err(AppError::validation(format!("후처리 방식은 notes / fix: {mode}"), None));
    }
    let n = sqlx::query("UPDATE user_plugins SET process_mode = ? WHERE id = ?")
        .bind(mode)
        .bind(id)
        .execute(db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(AppError::not_found(format!("설치된 플러그인이 아님: {id}")));
    }
    Ok(())
}

fn clean_notes(notes: &[PluginNote]) -> Vec<PluginNote> {
    notes
        .iter()
        .filter(|n| !n.text.trim().is_empty())
        .take(MAX_NOTES)
        .map(|n| PluginNote {
            level: if n.level == "warn" { "warn".into() } else { "info".into() },
            text: n.text.trim().chars().take(MAX_NOTE).collect(),
        })
        .collect()
}

/// 메시지 meta. 비었으면 {}, 읽을 수 없으면 거절 (덮어써서 스와이프 등을 잃지 않게)
fn parse_meta(raw: Option<String>) -> Result<Value> {
    match raw.as_deref().map(str::trim) {
        None | Some("") | Some("null") => Ok(json!({})),
        Some(r) => serde_json::from_str::<Value>(r)
            .ok()
            .filter(Value::is_object)
            .ok_or_else(|| AppError::validation("메시지 meta를 읽을 수 없어 후처리를 저장하지 않음", None)),
    }
}

/// 후처리 결과 저장. `expected`는 후처리를 시작할 때의 글, `content`는 끝난 글
pub async fn apply(db: &SqlitePool, message_id: &str, expected: &str, content: &str, results: &[PostProcessResult]) -> Result<()> {
    if content.chars().count() > MAX_CONTENT {
        return Err(AppError::validation("후처리한 글이 너무 김", None));
    }
    let modes = modes(db).await?;
    let mut changed_by = vec![];
    let mut notes = serde_json::Map::new();
    for r in results {
        // 도는 중이고 messages.process 권한이 있는 플러그인의 결과만 받는다
        if !runtime::is_running(&r.plugin_id) || !runtime::has_permission(&r.plugin_id, "messages.process") {
            return Err(AppError::validation(format!("후처리할 수 없는 플러그인: {}", r.plugin_id), None));
        }
        if r.changed {
            if modes.get(&r.plugin_id).map(String::as_str) != Some(MODE_FIX) {
                return Err(AppError::validation(
                    format!("'{}'은(는) 표시만 하도록 되어 있어 글을 바꿀 수 없음", r.plugin_id),
                    None,
                ));
            }
            changed_by.push(r.plugin_id.clone());
        }
        let n = clean_notes(&r.notes);
        if !n.is_empty() {
            notes.insert(r.plugin_id.clone(), serde_json::to_value(n).unwrap_or_default());
        }
    }
    if content != expected && changed_by.is_empty() {
        return Err(AppError::validation("글을 바꾼 플러그인이 없는데 글이 다름", None));
    }

    let mut tx = db.begin().await?;
    let row: Option<(String, String, Option<String>)> = sqlx::query_as("SELECT role, content, meta FROM messages WHERE id = ?")
        .bind(message_id)
        .fetch_optional(&mut *tx)
        .await?;
    let Some((role, now_content, meta_raw)) = row else {
        return Err(AppError::not_found(format!("메시지가 없음: {message_id}")));
    };
    if role != "assistant" {
        return Err(AppError::validation("AI 응답만 후처리함", None));
    }
    if now_content != expected {
        return Err(AppError::validation("후처리하는 사이 메시지가 바뀌어 저장하지 않음", None));
    }
    let mut meta = parse_meta(meta_raw)?;
    // 처음 원문을 지킨다 (이미 고친 메시지를 다시 처리해도)
    let original = meta
        .pointer("/pluginProcess/original")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| expected.to_string());
    if changed_by.is_empty() && notes.is_empty() {
        return Ok(());
    }
    meta["pluginProcess"] = json!({
        "original": original,
        "content": content,
        "changedBy": changed_by,
        "notes": notes,
        "at": chrono::Utc::now().timestamp_millis(),
    });
    // 지금 보이는 스와이프의 글도 같이 (스와이프를 넘겼다 돌아와도 고친 글이 보이게)
    if content != expected {
        let sid = meta.get("swipeId").and_then(Value::as_u64).unwrap_or(0) as usize;
        if let Some(s) = meta.get_mut("swipes").and_then(Value::as_array_mut).and_then(|a| a.get_mut(sid)) {
            if s.get("content").and_then(Value::as_str) == Some(expected) {
                s["content"] = Value::String(content.to_string());
            }
        }
    }
    sqlx::query("UPDATE messages SET content = ?, meta = ?, updated_at = ? WHERE id = ?")
        .bind(content)
        .bind(meta.to_string())
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(message_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// 되돌리기: 후처리한 글이 그대로일 때만 원문으로. 메모도 지운다
pub async fn revert(db: &SqlitePool, message_id: &str) -> Result<()> {
    let mut tx = db.begin().await?;
    let row: Option<(String, Option<String>)> = sqlx::query_as("SELECT content, meta FROM messages WHERE id = ?")
        .bind(message_id)
        .fetch_optional(&mut *tx)
        .await?;
    let Some((content, meta_raw)) = row else {
        return Err(AppError::not_found(format!("메시지가 없음: {message_id}")));
    };
    let mut meta = parse_meta(meta_raw)?;
    let Some(pp) = meta.get("pluginProcess").cloned() else {
        return Err(AppError::validation("플러그인이 고친 메시지가 아님", None));
    };
    let original = pp.get("original").and_then(Value::as_str).unwrap_or(&content).to_string();
    let processed = pp.get("content").and_then(Value::as_str).unwrap_or("");
    if content != processed {
        return Err(AppError::validation("플러그인이 고친 뒤 메시지를 또 고쳐서 되돌리지 않음", None));
    }
    if let Some(m) = meta.as_object_mut() {
        m.remove("pluginProcess");
    }
    let sid = meta.get("swipeId").and_then(Value::as_u64).unwrap_or(0) as usize;
    if let Some(s) = meta.get_mut("swipes").and_then(Value::as_array_mut).and_then(|a| a.get_mut(sid)) {
        if s.get("content").and_then(Value::as_str) == Some(content.as_str()) {
            s["content"] = Value::String(original.clone());
        }
    }
    sqlx::query("UPDATE messages SET content = ?, meta = ?, updated_at = ? WHERE id = ?")
        .bind(&original)
        .bind(meta.to_string())
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(message_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
