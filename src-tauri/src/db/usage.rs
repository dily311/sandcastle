//! 사용량 기록 (usage_log) — ResourceManager 1차.
//!
//! 공급자 호출 한 번마다 한 줄. 토큰은 공급자가 알려주면 그 값(estimated=0), 아니면 앱이 추정한 값(estimated=1).
//! 지금 실행 중인 작업(task_context)이 있으면 그 작업 id·기능을 함께 남긴다.
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Sqlite};

use crate::task::current_task;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageKind {
    Chat,
    Tts,
    Image,
    /// 임베딩 (입력 토큰만 센다, CLAUDE.md 3-A-1)
    Embedding,
}

impl UsageKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Tts => "tts",
            Self::Image => "image",
            Self::Embedding => "embedding",
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
    /// 그때의 단가로 계산한 금액(달러). 글 호출(채팅·임베딩)만, 단가를 모르면 None
    pub cost_usd: Option<f64>,
    /// 단가를 몰라 금액을 계산하지 못한 글 호출
    pub unpriced: bool,
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
            cost_usd: None,
            unpriced: false,
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
        "INSERT INTO usage_log (task_id, feature, api_profile_id, model, kind, input_tokens, output_tokens, characters, images, estimated, ok, error, duration_ms, created_at, cost_usd, unpriced)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(entry.cost_usd)
    .bind(entry.unpriced)
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
    pub model: Option<String>,
    /// 단가표로 계산한 추정 금액 (단가표에 이 모델이 없으면 None)
    pub estimated_cost: Option<f64>,
    pub currency: Option<String>,
}

pub async fn summary<'e, E>(executor: E, since: i64) -> Result<Vec<UsageSummary>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    // 상세(usage_log) + 정리된 일별 합계(usage_daily)
    sqlx::query_as::<_, UsageSummary>(
        "WITH u AS (
             SELECT api_profile_id, model, kind, 1 AS calls, CASE WHEN ok = 0 THEN 1 ELSE 0 END AS failed,
                    input_tokens, output_tokens, characters, images, estimated AS estimated_calls, cost_usd
             FROM usage_log WHERE created_at >= ?1
             UNION ALL
             SELECT api_profile_id, model, kind, calls, failed, input_tokens, output_tokens, characters, images, estimated_calls, cost_usd
             FROM usage_daily WHERE day_start >= ?1
         )
         SELECT u.api_profile_id, p.name AS profile_name, u.kind,
                SUM(u.calls) AS calls,
                SUM(u.failed) AS failed,
                SUM(u.input_tokens) AS input_tokens,
                SUM(u.output_tokens) AS output_tokens,
                SUM(u.characters) AS characters,
                SUM(u.images) AS images,
                SUM(u.estimated_calls) AS estimated_calls,
                u.model,
                -- 호출할 때 계산해 둔 금액(달러)이 있으면 그것, 없으면(음성·그림·옛 기록) 단가표로 계산
                CASE WHEN SUM(u.cost_usd) IS NOT NULL THEN SUM(u.cost_usd)
                     WHEN mp.model IS NULL THEN NULL ELSE
                    SUM(u.input_tokens) * COALESCE(mp.input_per_mtok, 0) / 1000000.0
                  + SUM(u.output_tokens) * COALESCE(mp.output_per_mtok, 0) / 1000000.0
                  + SUM(u.characters) * COALESCE(mp.per_1k_chars, 0) / 1000.0
                  + SUM(u.images) * COALESCE(mp.per_image, 0)
                END AS estimated_cost,
                CASE WHEN SUM(u.cost_usd) IS NOT NULL THEN '$' ELSE mp.currency END AS currency
         FROM u
         LEFT JOIN api_profiles p ON p.id = u.api_profile_id
         LEFT JOIN model_prices mp ON mp.model = u.model
         GROUP BY u.api_profile_id, u.kind, u.model
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

// ---------------------------------------------------------
// 단가표 (1-A-3): 사용자가 적는 모델별 단가. 앱은 가격을 넣어 두지 않는다
// ---------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelPrice {
    pub model: String,
    pub currency: String,
    /// 100만 토큰당
    pub input_per_mtok: Option<f64>,
    pub output_per_mtok: Option<f64>,
    /// 음성 1000자당
    pub per_1k_chars: Option<f64>,
    /// 그림 1장당
    pub per_image: Option<f64>,
}

pub async fn get_all_model_prices<'e, E>(executor: E) -> Result<Vec<ModelPrice>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, ModelPrice>("SELECT model, currency, input_per_mtok, output_per_mtok, per_1k_chars, per_image FROM model_prices ORDER BY model")
        .fetch_all(executor)
        .await
}

pub async fn upsert_model_price<'e, E>(executor: E, price: &ModelPrice) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    // 음수는 비운 칸으로
    let ok = |v: Option<f64>| v.filter(|n| n.is_finite() && *n >= 0.0);
    sqlx::query(
        "INSERT INTO model_prices (model, currency, input_per_mtok, output_per_mtok, per_1k_chars, per_image, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(model) DO UPDATE SET currency = excluded.currency, input_per_mtok = excluded.input_per_mtok,
           output_per_mtok = excluded.output_per_mtok, per_1k_chars = excluded.per_1k_chars, per_image = excluded.per_image,
           updated_at = excluded.updated_at",
    )
    .bind(price.model.trim())
    .bind(if price.currency.trim().is_empty() { "$" } else { price.currency.trim() })
    .bind(ok(price.input_per_mtok))
    .bind(ok(price.output_per_mtok))
    .bind(ok(price.per_1k_chars))
    .bind(ok(price.per_image))
    .bind(chrono::Utc::now().timestamp_millis())
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn delete_model_price<'e, E>(executor: E, model: &str) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("DELETE FROM model_prices WHERE model = ?").bind(model).execute(executor).await?;
    Ok(())
}

/// 사용량 기록 비우기 (작업 기록은 남긴다). 정리된 일별 합계도 지운다.
/// 금액 한도가 걸린 프로필이 있으면 이번 달 글 호출 기록은 남긴다 — 지우면 한도가 0으로 돌아가 막힌 호출이 풀리기 때문
pub async fn clear(pool: &sqlx::SqlitePool) -> Result<u64, sqlx::Error> {
    let month_start = super::budget::month_start(&chrono::Local::now());
    let mut tx = pool.begin().await?;
    let n = sqlx::query(
        "DELETE FROM usage_log WHERE NOT (created_at >= ?1 AND kind IN ('chat', 'embedding') AND EXISTS (SELECT 1 FROM budget_limits WHERE monthly_usd IS NOT NULL))",
    )
    .bind(month_start)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    sqlx::query(
        "DELETE FROM usage_daily WHERE NOT (day_start >= ?1 AND kind IN ('chat', 'embedding') AND EXISTS (SELECT 1 FROM budget_limits WHERE monthly_usd IS NOT NULL))",
    )
    .bind(month_start)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(n)
}

// ---------------------------------------------------------
// 기록 정리 (1-A-4): 오래된 상세는 일별 합계로 남기고 지운다
// ---------------------------------------------------------

/// 보존 기간으로 고를 수 있는 날 수 (30일보다 짧으면 실행 기록 화면의 "30일" 칸이 비므로 30일부터)
pub const RETENTION_CHOICES: [u32; 4] = [30, 90, 180, 365];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CompactResult {
    /// 일별 합계로 옮기고 지운 사용량 상세 줄 수
    pub usage_rows: u32,
    /// 지운 작업 수 (시도 기록은 함께 지워짐)
    pub tasks: u32,
}

/// 보존 기간의 시작: (오늘 − days)일 0시 (현지, ms). 하루 단위로 자르므로 일별 합계와 기간 계산이 맞는다
pub fn retention_cutoff<Tz: chrono::TimeZone>(now: &chrono::DateTime<Tz>, days: u32) -> i64 {
    let back = now.clone() - chrono::Duration::days(i64::from(days));
    super::budget::period_starts(&back).0
}

/// cutoff보다 오래된 사용량 상세를 일별 합계로 옮기고, 끝난 작업을 지운다.
/// - 지우는 작업: 완료·실패·멈춤만. 끊긴 작업(다시 실행 가능)과 다른 기록이 가리키는 작업(대화 제출·에이전트 도구 호출)은 남긴다
pub async fn compact_before(pool: &sqlx::SqlitePool, cutoff: i64) -> Result<CompactResult, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO usage_daily (day, day_start, api_profile_id, model, kind, feature, calls, failed, input_tokens, output_tokens, characters, images, estimated_calls, cost_usd, unpriced_calls)
         SELECT date(created_at / 1000, 'unixepoch', 'localtime') AS d,
                CAST(strftime('%s', date(created_at / 1000, 'unixepoch', 'localtime'), 'utc') AS INTEGER) * 1000,
                api_profile_id, model, kind, feature,
                COUNT(*), SUM(CASE WHEN ok = 0 THEN 1 ELSE 0 END),
                SUM(input_tokens), SUM(output_tokens), SUM(characters), SUM(images), SUM(estimated), SUM(cost_usd), SUM(unpriced)
         FROM usage_log WHERE created_at < ?
         GROUP BY d, api_profile_id, model, kind, feature",
    )
    .bind(cutoff)
    .execute(&mut *tx)
    .await?;
    let usage_rows = sqlx::query("DELETE FROM usage_log WHERE created_at < ?")
        .bind(cutoff)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    let tasks = sqlx::query(
        "DELETE FROM generation_tasks
         WHERE created_at < ? AND status IN ('completed', 'failed', 'cancelled')
           AND id NOT IN (SELECT task_id FROM ai_submissions)
           AND id NOT IN (SELECT task_id FROM agent_tool_calls)",
    )
    .bind(cutoff)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    let n = |v: u64| u32::try_from(v).unwrap_or(u32::MAX);
    Ok(CompactResult {
        usage_rows: n(usage_rows),
        tasks: n(tasks),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
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
        assert_eq!(
            (chat.calls, chat.failed, chat.input_tokens, chat.output_tokens, chat.estimated_calls),
            (2, 1, 200, 40, 1)
        );
        let tts = rows.iter().find(|r| r.kind == "tts").unwrap();
        assert_eq!(tts.characters, 42);
        assert!(task_history(&db, 10).await.unwrap().is_empty());
        assert!(rows.iter().all(|r| r.estimated_cost.is_none()));
    }

    #[tokio::test]
    async fn compact_keeps_totals_and_removes_details() {
        let db = pool().await;
        let day = 86_400_000_i64;
        let now = chrono::Local::now();
        let at = |d: i64| now.timestamp_millis() - d * day;
        for (days_ago, tokens, ok) in [(60, 100, true), (45, 50, false), (45, 7, true), (2, 30, true)] {
            let mut e = UsageEntry::new(UsageKind::Chat, "p1", "m");
            (e.input_tokens, e.output_tokens, e.ok, e.estimated) = (tokens, 1, ok, true);
            record(&db, e).await;
            sqlx::query("UPDATE usage_log SET created_at = ? WHERE id = (SELECT MAX(id) FROM usage_log)")
                .bind(at(days_ago))
                .execute(&db)
                .await
                .unwrap();
        }
        // 오래된 끝난 작업 / 끊긴 작업 / 최근 작업
        for (id, status, days_ago) in [("old", "completed", 50), ("cut", "interrupted", 50), ("new", "completed", 1)] {
            sqlx::query("INSERT INTO generation_tasks (id, task_type, status, created_at, updated_at) VALUES (?, 'chat_stream', ?, ?, ?)")
                .bind(id)
                .bind(status)
                .bind(at(days_ago))
                .bind(at(days_ago))
                .execute(&db)
                .await
                .unwrap();
        }
        let before = summary(&db, 0).await.unwrap();
        let month_start = crate::db::budget::period_starts(&now).1;
        let all_before = crate::db::budget::cost_since(&db, "p1", 0).await.unwrap();
        let month_before = crate::db::budget::cost_since(&db, "p1", month_start).await.unwrap();

        let r = compact_before(&db, retention_cutoff(&now, 30)).await.unwrap();
        assert_eq!(r, CompactResult { usage_rows: 3, tasks: 1 });

        let pick = |v: &[UsageSummary]| {
            v.iter()
                .map(|r| (r.calls, r.failed, r.input_tokens, r.output_tokens, r.estimated_calls))
                .collect::<Vec<_>>()
        };
        let after = summary(&db, 0).await.unwrap();
        assert_eq!(pick(&before), pick(&after));
        assert_eq!(pick(&after), vec![(4, 1, 187, 4, 4)]);
        assert_eq!(crate::db::budget::cost_since(&db, "p1", 0).await.unwrap(), all_before);
        assert_eq!(crate::db::budget::cost_since(&db, "p1", month_start).await.unwrap(), month_before);
        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usage_log").fetch_one(&db).await.unwrap();
        assert_eq!(left, 1);
        let tasks: Vec<String> = sqlx::query_scalar("SELECT id FROM generation_tasks ORDER BY id").fetch_all(&db).await.unwrap();
        assert_eq!(tasks, vec!["cut".to_string(), "new".to_string()]);
        // 다시 해도 바뀌지 않음
        assert_eq!(compact_before(&db, retention_cutoff(&now, 30)).await.unwrap(), CompactResult::default());
        assert_eq!(pick(&summary(&db, 0).await.unwrap()), pick(&before));
    }

    #[tokio::test]
    async fn estimated_cost_uses_user_prices() {
        let db = pool().await;
        let mut a = UsageEntry::new(UsageKind::Chat, "p1", "gpt-x");
        (a.input_tokens, a.output_tokens) = (2_000_000, 500_000);
        record(&db, a).await;
        let mut i = UsageEntry::new(UsageKind::Image, "p2", "img");
        i.images = 3;
        record(&db, i).await;
        upsert_model_price(
            &db,
            &ModelPrice {
                model: "gpt-x".into(),
                currency: "".into(),
                input_per_mtok: Some(1.5),
                output_per_mtok: Some(4.0),
                per_1k_chars: None,
                per_image: Some(-1.0),
            },
        )
        .await
        .unwrap();
        let rows = summary(&db, 0).await.unwrap();
        let chat = rows.iter().find(|r| r.kind == "chat").unwrap();
        assert!((chat.estimated_cost.unwrap() - 5.0).abs() < 1e-9); // 2 × 1.5 + 0.5 × 4
        assert_eq!(chat.currency.as_deref(), Some("$"));
        // 단가표에 없는 모델은 금액 없음
        assert!(rows.iter().find(|r| r.kind == "image").unwrap().estimated_cost.is_none());
        let saved = get_all_model_prices(&db).await.unwrap();
        assert_eq!(saved[0].per_image, None);
        delete_model_price(&db, "gpt-x").await.unwrap();
        assert!(get_all_model_prices(&db).await.unwrap().is_empty());
    }
}
