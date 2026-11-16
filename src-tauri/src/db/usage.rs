//! 사용량 기록 (usage_log) — ResourceManager 1차.
//!
//! 공급자 호출 한 번마다 한 줄. 토큰은 공급자가 알려주면 그 값(estimated=0), 아니면 앱이 추정한 값(estimated=1).
//! 지금 실행 중인 작업(task_context)이 있으면 그 작업 id·기능을 함께 남긴다.
use serde::Serialize;
use sqlx::{Executor, Sqlite};

use crate::task::current_task;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageKind {
    Chat,
    Tts,
    Image,
}

impl UsageKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Tts => "tts",
            Self::Image => "image",
        }
    }
}

/// 호출 한 번의 사용량
#[derive(Debug, Clone)]
pub struct UsageEntry {
    pub kind: UsageKind,
    pub api_profile_id: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub characters: u64,
    pub images: u64,
    pub estimated: bool,
    pub ok: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl UsageEntry {
    pub fn new(kind: UsageKind, api_profile_id: &str, model: &str) -> Self {
        Self {
            kind,
            api_profile_id: api_profile_id.to_string(),
            model: model.to_string(),
            input_tokens: 0,
            output_tokens: 0,
            characters: 0,
            images: 0,
            estimated: false,
            ok: true,
            error: None,
            duration_ms: 0,
        }
    }
}

fn to_i64(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

pub async fn insert<'e, E>(executor: E, entry: &UsageEntry) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let task = current_task();
    sqlx::query(
        "INSERT INTO usage_log (task_id, feature, api_profile_id, model, kind, input_tokens, output_tokens, characters, images, estimated, ok, error, duration_ms, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(task.as_ref().map(|t| t.task_id.clone()))
    .bind(task.and_then(|t| t.feature))
    .bind(&entry.api_profile_id)
    .bind(&entry.model)
    .bind(entry.kind.as_str())
    .bind(to_i64(entry.input_tokens))
    .bind(to_i64(entry.output_tokens))
    .bind(to_i64(entry.characters))
    .bind(to_i64(entry.images))
    .bind(entry.estimated)
    .bind(entry.ok)
    .bind(&entry.error)
    .bind(to_i64(entry.duration_ms))
    .bind(chrono::Utc::now().timestamp_millis())
    .execute(executor)
    .await?;
    Ok(())
}

/// 기록 실패가 원래 호출을 망치지 않게: 실패하면 로그만 남긴다
pub async fn record<'e, E>(executor: E, entry: UsageEntry)
where
    E: Executor<'e, Database = Sqlite>,
{
    if let Err(e) = insert(executor, &entry).await {
        eprintln!("[usage] 사용량 기록 실패: {e}");
    }
}

/// API 프로필·종류별 합계 (설정 > 실행 기록 화면)
#[derive(Debug, Clone, Serialize, sqlx::FromRow, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub api_profile_id: Option<String>,
    /// 지워진 프로필이면 None
    pub profile_name: Option<String>,
    pub kind: String,
    pub calls: i64,
    pub failed: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub characters: i64,
    pub images: i64,
    /// 토큰을 앱이 추정한 호출 수
    pub estimated_calls: i64,
}

pub async fn summary<'e, E>(executor: E, since: i64) -> Result<Vec<UsageSummary>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, UsageSummary>(
        "SELECT u.api_profile_id, p.name AS profile_name, u.kind,
                COUNT(*) AS calls,
                SUM(CASE WHEN u.ok = 0 THEN 1 ELSE 0 END) AS failed,
                SUM(u.input_tokens) AS input_tokens,
                SUM(u.output_tokens) AS output_tokens,
                SUM(u.characters) AS characters,
                SUM(u.images) AS images,
                SUM(u.estimated) AS estimated_calls
         FROM usage_log u LEFT JOIN api_profiles p ON p.id = u.api_profile_id
         WHERE u.created_at >= ?
         GROUP BY u.api_profile_id, u.kind
         ORDER BY calls DESC",
    )
    .bind(since)
    .fetch_all(executor)
    .await
}

/// 최근 실행 내역 한 줄 (작업 + 그 작업의 사용량 합계)
#[derive(Debug, Clone, Serialize, sqlx::FromRow, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaskHistoryItem {
    pub id: String,
    pub task_type: String,
    pub feature: Option<String>,
    /// user / auto
    pub run_by: String,
    pub label: Option<String>,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub attempt_count: i64,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub api_profile_id: Option<String>,
    pub profile_name: Option<String>,
    pub model: Option<String>,
    pub calls: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub characters: i64,
    pub images: i64,
    pub estimated: bool,
}

pub async fn task_history<'e, E>(executor: E, limit: i64) -> Result<Vec<TaskHistoryItem>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, TaskHistoryItem>(
        "SELECT t.id, t.task_type, t.feature, t.run_by, t.label, t.status, t.error_code, t.error_message,
                t.attempt_count, t.created_at, t.started_at, t.finished_at,
                u.api_profile_id, p.name AS profile_name, u.model,
                COALESCE(u.calls, 0) AS calls,
                COALESCE(u.input_tokens, 0) AS input_tokens,
                COALESCE(u.output_tokens, 0) AS output_tokens,
                COALESCE(u.characters, 0) AS characters,
                COALESCE(u.images, 0) AS images,
                COALESCE(u.estimated, 0) AS estimated
         FROM generation_tasks t
         LEFT JOIN (
             SELECT task_id, MAX(api_profile_id) AS api_profile_id, MAX(model) AS model, COUNT(*) AS calls,
                    SUM(input_tokens) AS input_tokens, SUM(output_tokens) AS output_tokens,
                    SUM(characters) AS characters, SUM(images) AS images, MAX(estimated) AS estimated
             FROM usage_log WHERE task_id IS NOT NULL GROUP BY task_id
         ) u ON u.task_id = t.id
         LEFT JOIN api_profiles p ON p.id = u.api_profile_id
         ORDER BY t.created_at DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(executor)
    .await
}

/// 사용량 기록 비우기 (작업 기록은 남긴다)
pub async fn clear<'e, E>(executor: E) -> Result<u64, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    Ok(sqlx::query("DELETE FROM usage_log").execute(executor).await?.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn records_and_sums_by_profile_and_kind() {
        let db = pool().await;
        let mut a = UsageEntry::new(UsageKind::Chat, "p1", "m");
        a.input_tokens = 100;
        a.output_tokens = 20;
        a.estimated = true;
        record(&db, a.clone()).await;
        a.ok = false;
        a.estimated = false;
        record(&db, a).await;
        let mut t = UsageEntry::new(UsageKind::Tts, "p1", "v");
        t.characters = 42;
        record(&db, t).await;

        let rows = summary(&db, 0).await.unwrap();
        let chat = rows.iter().find(|r| r.kind == "chat").unwrap();
        assert_eq!((chat.calls, chat.failed, chat.input_tokens, chat.output_tokens, chat.estimated_calls), (2, 1, 200, 40, 1));
        let tts = rows.iter().find(|r| r.kind == "tts").unwrap();
        assert_eq!(tts.characters, 42);
        assert!(task_history(&db, 10).await.unwrap().is_empty());
    }
}
