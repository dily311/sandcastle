//! AI 호출 기록 (5-E-20-6-7): 보낸 프롬프트·받은 원문을 작업에 묶어 남긴다. 설정 > 실행 기록에서 본다.
//! 켜야만 남긴다 (app-setting aiCallLog, 기본 꺼짐 — 프롬프트 전체가 디스크에 남으므로). 최근 KEEP개만 둔다.
//! 비밀(API 키)은 프롬프트에 없고 여기 넣지 않는다.
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use serde_json::json;

use crate::{db::conn::DbPool, models::ai_service::PromptMessage};

static ENABLED: AtomicBool = AtomicBool::new(false);
pub const KEEP: i64 = 200;

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// 한 번의 호출 (보낸 것·받은 것)
#[derive(Debug, Clone, Serialize, sqlx::FromRow, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AiCallLog {
    pub id: i64,
    pub task_id: Option<String>,
    pub profile_id: String,
    pub model: String,
    pub request_json: String,
    pub response_text: String,
    pub finish_reason: Option<String>,
    pub error: Option<String>,
    pub created_at: i64,
}

pub struct CallRecord<'a> {
    pub task_id: Option<String>,
    pub profile_id: &'a str,
    pub model: &'a str,
    pub prompt: &'a PromptMessage,
    pub response: &'a str,
    pub finish_reason: Option<&'a str>,
    pub error: Option<&'a str>,
}

/// 남기고 오래된 것 지우기 (실패해도 호출에는 영향 없음)
pub async fn record(db: &DbPool, r: CallRecord<'_>) {
    let request = json!({
        "messages": r.prompt.messages.iter().map(|m| json!({ "role": m.role, "content": m.content, "images": m.images.len() })).collect::<Vec<_>>(),
        "options": r.prompt.chat_options,
    });
    let now = chrono::Utc::now().timestamp_millis();
    let res = sqlx::query(
        "INSERT INTO ai_call_logs (task_id, profile_id, model, request_json, response_text, finish_reason, error, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&r.task_id)
    .bind(r.profile_id)
    .bind(r.model)
    .bind(request.to_string())
    .bind(r.response)
    .bind(r.finish_reason)
    .bind(r.error)
    .bind(now)
    .execute(db)
    .await;
    if let Err(e) = res {
        eprintln!("[ai] 호출 기록 실패: {e}");
        return;
    }
    let _ = sqlx::query("DELETE FROM ai_call_logs WHERE id NOT IN (SELECT id FROM ai_call_logs ORDER BY id DESC LIMIT ?)")
        .bind(KEEP)
        .execute(db)
        .await;
}

pub async fn for_task(db: &DbPool, task_id: &str) -> Result<Vec<AiCallLog>, sqlx::Error> {
    sqlx::query_as::<_, AiCallLog>("SELECT * FROM ai_call_logs WHERE task_id = ? ORDER BY id")
        .bind(task_id)
        .fetch_all(db)
        .await
}

/// 기록이 있는 작업 id (실행 기록 목록에서 표시용)
pub async fn task_ids(db: &DbPool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT DISTINCT task_id FROM ai_call_logs WHERE task_id IS NOT NULL")
        .fetch_all(db)
        .await
}

pub async fn clear(db: &DbPool) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query("DELETE FROM ai_call_logs").execute(db).await?.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ai_service::{ChatOptions, PromptMessageItem, PromptRole};

    #[tokio::test]
    async fn records_request_and_response_and_keeps_only_the_latest() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::conn::init_pool(dir.path().to_path_buf()).await.unwrap();
        let prompt = PromptMessage {
            chat_options: ChatOptions::default(),
            messages: vec![PromptMessageItem {
                role: PromptRole::User,
                content: "지시".into(),
                token: 0,
                images: vec![],
            }],
        };
        for i in 0..(KEEP + 5) {
            record(
                &db,
                CallRecord {
                    task_id: Some(format!("t{i}")),
                    profile_id: "p",
                    model: "m",
                    prompt: &prompt,
                    response: "답",
                    finish_reason: Some("MAX_TOKENS"),
                    error: None,
                },
            )
            .await;
        }
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_call_logs").fetch_one(&db).await.unwrap();
        assert_eq!(n, KEEP, "최근 것만 남김");
        assert!(for_task(&db, "t0").await.unwrap().is_empty(), "오래된 것은 지워짐");
        let last = for_task(&db, &format!("t{}", KEEP + 4)).await.unwrap();
        assert_eq!(last.len(), 1);
        let req: serde_json::Value = serde_json::from_str(&last[0].request_json).unwrap();
        assert_eq!(req["messages"][0]["content"], "지시");
        assert_eq!(last[0].response_text, "답");
        assert_eq!(last[0].finish_reason.as_deref(), Some("MAX_TOKENS"));
        assert_eq!(clear(&db).await.unwrap(), KEEP as u64);
    }
}
