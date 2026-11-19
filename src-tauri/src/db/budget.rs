//! 예산 한도 (budget_limits) — API 프로필별 "한 달 금액(달러)" 하나 (ADR-007 추가).
//!
//! 글 호출(채팅·임베딩)의 금액(usage_log.cost_usd — 호출할 때 그때의 단가로 계산해 둔 값)을 이번 달 1일 0시(현지)부터 더해
//! 한도와 견준다. 값이 없으면(NULL·행 없음) 한도 없음. 음성·그림은 한도를 두지 않는다(구독형 공급자는 잔여만 보여줌).
//! 예전 칸(하루·한 달 토큰, 음성 글자, 그림 장 수)은 더 이상 쓰지 않는다. 한 달 토큰 한도가 있던 사람에게는 안내만 한다.
use chrono::{DateTime, Datelike, Local, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Sqlite};

use super::usage::UsageKind;

/// 한 프로필의 한도. None = 한도 없음
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetLimit {
    pub api_profile_id: String,
    /// 한 달 금액 한도 (달러)
    pub monthly_usd: Option<f64>,
    /// 예전 "한 달 토큰" 한도 (읽기 전용 안내용. 새 한도를 저장하면 지워진다)
    #[serde(default)]
    pub legacy_monthly_tokens: Option<i64>,
}

impl BudgetLimit {
    /// 한도가 없는지 (행을 지워도 됨)
    pub fn is_empty(&self) -> bool {
        self.monthly_usd.filter(|v| v.is_finite() && *v > 0.0).is_none()
    }
}

pub async fn get_budget_limit<'e, E>(executor: E, api_profile_id: &str) -> Result<Option<BudgetLimit>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, BudgetLimit>("SELECT api_profile_id, monthly_usd, monthly_tokens AS legacy_monthly_tokens FROM budget_limits WHERE api_profile_id = ?")
        .bind(api_profile_id)
        .fetch_optional(executor)
        .await
}

pub async fn get_all_budget_limits<'e, E>(executor: E) -> Result<Vec<BudgetLimit>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, BudgetLimit>("SELECT api_profile_id, monthly_usd, monthly_tokens AS legacy_monthly_tokens FROM budget_limits ORDER BY api_profile_id")
        .fetch_all(executor)
        .await
}

/// 저장. 0 이하·숫자가 아닌 값은 "한도 없음"으로 본다 (실수로 0을 넣어 모든 호출이 막히지 않게).
/// 예전 칸은 비운다 — 새 한도를 정하면 "이전 한도" 안내가 사라진다
pub async fn upsert_budget_limit<'e, E>(executor: E, limit: &BudgetLimit) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let usd = limit.monthly_usd.filter(|v| v.is_finite() && *v > 0.0);
    sqlx::query(
        "INSERT INTO budget_limits (api_profile_id, monthly_usd, updated_at) VALUES (?, ?, ?)
         ON CONFLICT(api_profile_id) DO UPDATE SET
           monthly_usd = excluded.monthly_usd,
           daily_tokens = NULL, monthly_tokens = NULL, daily_characters = NULL, monthly_characters = NULL,
           daily_images = NULL, monthly_images = NULL,
           updated_at = excluded.updated_at",
    )
    .bind(&limit.api_profile_id)
    .bind(usd)
    .bind(chrono::Utc::now().timestamp_millis())
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn delete_budget_limit<'e, E>(executor: E, api_profile_id: &str) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("DELETE FROM budget_limits WHERE api_profile_id = ?")
        .bind(api_profile_id)
        .execute(executor)
        .await?;
    Ok(())
}

// ---------------------------------------------------------
// 이번 달 사용 금액 대비 남은 금액
// ---------------------------------------------------------

/// 이 호출 종류가 한도에 걸리는가 (글 호출만)
pub fn is_limited(kind: UsageKind) -> bool {
    matches!(kind, UsageKind::Chat | UsageKind::Embedding)
}

/// 한도가 걸린 칸 (한도 없으면 만들지 않는다)
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetMeter {
    pub used_usd: f64,
    pub limit_usd: f64,
    /// 0 이상 (넘었으면 0)
    pub remaining_usd: f64,
    /// 이번 달, 단가를 몰라 금액에 못 넣은 글 호출 수 (0이면 빠짐없이 센 것)
    pub unpriced_calls: i64,
}

impl BudgetMeter {
    pub fn exceeded(&self) -> bool {
        self.used_usd >= self.limit_usd
    }
    /// 한도의 몇 %를 썼나
    pub fn percent(&self) -> f64 {
        self.used_usd * 100.0 / self.limit_usd
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetStatus {
    pub api_profile_id: String,
    pub meter: Option<BudgetMeter>,
}

impl BudgetStatus {
    /// 이 호출 종류를 막는 칸 (글 호출만, 한 달 금액을 넘었을 때)
    pub fn blocking(&self, kind: UsageKind) -> Option<&BudgetMeter> {
        if !is_limited(kind) {
            return None;
        }
        self.meter.as_ref().filter(|m| m.exceeded())
    }
}

/// 이번 달 1일 0시 (현지, ms)
pub fn month_start<Tz: TimeZone>(now: &DateTime<Tz>) -> i64 {
    period_starts(now).1
}

/// 오늘 0시와 이번 달 1일 0시 (현지 시각, ms). 오늘 0시는 기록 정리(보존 기간)가 쓴다
pub fn period_starts<Tz: TimeZone>(now: &DateTime<Tz>) -> (i64, i64) {
    let tz = now.timezone();
    let start = |d: chrono::NaiveDate| {
        let naive = d.and_time(NaiveTime::MIN);
        // 서머타임으로 0시가 없는 날은 가장 이른 시각
        tz.from_local_datetime(&naive)
            .earliest()
            .map_or_else(|| naive.and_utc().timestamp_millis(), |t| t.timestamp_millis())
    };
    let today = now.date_naive();
    let month_first = today.with_day(1).unwrap_or(today);
    (start(today), start(month_first))
}

/// since 이후 이 프로필의 글 호출 금액(달러)과 단가를 몰라 못 센 호출 수. 실패한 호출도 센다(요금이 나갈 수 있음).
/// 정리된 일별 합계(usage_daily)도 더한다 — 정리는 하루 단위로 자르므로 기간 시작(0시)과 맞는다
pub async fn cost_since<'e, E>(executor: E, api_profile_id: &str, since: i64) -> Result<(f64, i64), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as(
        "SELECT COALESCE(SUM(c), 0.0), COALESCE(SUM(u), 0) FROM (
             -- 금액이 없는데 토큰이 있는 줄(단가를 몰랐던 호출, 이 기능 이전의 옛 줄)은 금액에 못 넣은 호출로 센다
             SELECT cost_usd AS c, CASE WHEN cost_usd IS NULL AND (unpriced = 1 OR input_tokens + output_tokens > 0) THEN 1 ELSE 0 END AS u
             FROM usage_log WHERE api_profile_id = ?1 AND created_at >= ?2 AND kind IN ('chat', 'embedding')
             UNION ALL
             SELECT cost_usd, CASE WHEN cost_usd IS NULL AND input_tokens + output_tokens > 0 THEN calls ELSE unpriced_calls END
             FROM usage_daily WHERE api_profile_id = ?1 AND day_start >= ?2 AND kind IN ('chat', 'embedding')
         )",
    )
    .bind(api_profile_id)
    .bind(since)
    .fetch_one(executor)
    .await
}

/// 한도와 이번 달 사용량으로 칸을 만든다 (순수 함수). 한도가 없으면 None
pub fn meter(limit: &BudgetLimit, used_usd: f64, unpriced_calls: i64) -> Option<BudgetMeter> {
    let limit_usd = limit.monthly_usd.filter(|v| v.is_finite() && *v > 0.0)?;
    Some(BudgetMeter {
        used_usd,
        limit_usd,
        remaining_usd: (limit_usd - used_usd).max(0.0),
        unpriced_calls,
    })
}

/// 이 프로필의 지금 상태 (한도가 없으면 meter가 None)
pub async fn budget_status<Tz: TimeZone>(pool: &sqlx::SqlitePool, api_profile_id: &str, now: &DateTime<Tz>) -> Result<BudgetStatus, sqlx::Error> {
    let Some(limit) = get_budget_limit(pool, api_profile_id).await?.filter(|l| !l.is_empty()) else {
        return Ok(BudgetStatus {
            api_profile_id: api_profile_id.to_string(),
            meter: None,
        });
    };
    let (used, unpriced) = cost_since(pool, api_profile_id, month_start(now)).await?;
    Ok(BudgetStatus {
        api_profile_id: api_profile_id.to_string(),
        meter: meter(&limit, used, unpriced),
    })
}

/// 지금 시각(현지) 기준 상태
pub async fn budget_status_now(pool: &sqlx::SqlitePool, api_profile_id: &str) -> Result<BudgetStatus, sqlx::Error> {
    budget_status(pool, api_profile_id, &Local::now()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::usage::{record, UsageEntry};
    use chrono::FixedOffset;

    async fn pool() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        sqlx::query("INSERT INTO api_profiles (id, name, provider, format_type, base_url, model, created_at, updated_at) VALUES ('p1', 'P', 'openai', 'openai', '', 'm', 0, 0)")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    async fn usage_at(db: &sqlx::SqlitePool, kind: UsageKind, at: i64, cost: Option<f64>, unpriced: bool) {
        let mut e = UsageEntry::new(kind, "p1", "m");
        e.input_tokens = 1000;
        (e.cost_usd, e.unpriced) = (cost, unpriced);
        record(db, e).await;
        sqlx::query("UPDATE usage_log SET created_at = ? WHERE id = (SELECT MAX(id) FROM usage_log)")
            .bind(at)
            .execute(db)
            .await
            .unwrap();
    }

    fn kst(y: i32, m: u32, d: u32, h: u32) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(9 * 3600).unwrap().with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    fn limit(usd: f64) -> BudgetLimit {
        BudgetLimit {
            api_profile_id: "p1".into(),
            monthly_usd: Some(usd),
            ..Default::default()
        }
    }

    #[test]
    fn period_starts_are_local_midnight_and_month_first() {
        let now = kst(2026, 11, 16, 15);
        let (day, month) = period_starts(&now);
        assert_eq!(day, kst(2026, 11, 16, 0).timestamp_millis());
        assert_eq!(month, kst(2026, 11, 1, 0).timestamp_millis());
        assert_eq!(month_start(&now), month);
    }

    #[test]
    fn meter_needs_a_positive_limit_and_never_goes_negative() {
        assert!(meter(&BudgetLimit::default(), 5.0, 0).is_none());
        assert!(meter(&limit(0.0), 5.0, 0).is_none());
        let m = meter(&limit(10.0), 12.5, 3).unwrap();
        assert_eq!(
            (m.used_usd, m.limit_usd, m.remaining_usd, m.unpriced_calls, m.exceeded()),
            (12.5, 10.0, 0.0, 3, true)
        );
        assert!((meter(&limit(10.0), 8.0, 0).unwrap().percent() - 80.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn status_sums_this_months_text_cost_only() {
        let db = pool().await;
        let now = kst(2026, 11, 16, 15);
        usage_at(&db, UsageKind::Chat, kst(2026, 10, 31, 23).timestamp_millis(), Some(9.0), false).await; // 지난달 — 안 셈
        usage_at(&db, UsageKind::Chat, kst(2026, 11, 3, 12).timestamp_millis(), Some(0.5), false).await;
        usage_at(&db, UsageKind::Embedding, kst(2026, 11, 16, 9).timestamp_millis(), Some(0.25), false).await;
        usage_at(&db, UsageKind::Chat, kst(2026, 11, 16, 10).timestamp_millis(), None, true).await; // 단가 없음
        usage_at(&db, UsageKind::Image, kst(2026, 11, 16, 11).timestamp_millis(), Some(7.0), false).await; // 그림은 안 셈

        assert!(budget_status(&db, "p1", &now).await.unwrap().meter.is_none()); // 한도 없음
        upsert_budget_limit(&db, &limit(1.0)).await.unwrap();
        let s = budget_status(&db, "p1", &now).await.unwrap();
        let m = s.meter.clone().unwrap();
        assert!((m.used_usd - 0.75).abs() < 1e-9);
        assert_eq!(m.unpriced_calls, 1);
        assert!(s.blocking(UsageKind::Chat).is_none());
        // 한도를 넘기면 글 호출만 막힌다
        upsert_budget_limit(&db, &limit(0.7)).await.unwrap();
        let s = budget_status(&db, "p1", &now).await.unwrap();
        assert!(s.blocking(UsageKind::Chat).is_some());
        assert!(s.blocking(UsageKind::Embedding).is_some());
        assert!(s.blocking(UsageKind::Image).is_none());
        assert!(s.blocking(UsageKind::Tts).is_none());
    }

    #[tokio::test]
    async fn compacted_days_still_count_and_legacy_limit_is_cleared_on_save() {
        let db = pool().await;
        let now = kst(2026, 11, 16, 15);
        usage_at(&db, UsageKind::Chat, kst(2026, 11, 2, 12).timestamp_millis(), Some(2.0), false).await;
        crate::db::usage::compact_before(&db, kst(2026, 11, 10, 0).timestamp_millis()).await.unwrap();
        // 상세는 일별 합계로 옮겨졌지만 이번 달 금액은 그대로
        sqlx::query("INSERT INTO budget_limits (api_profile_id, monthly_tokens, updated_at) VALUES ('p1', 5000, 0)")
            .execute(&db)
            .await
            .unwrap();
        assert_eq!(get_budget_limit(&db, "p1").await.unwrap().unwrap().legacy_monthly_tokens, Some(5000));
        upsert_budget_limit(&db, &limit(3.0)).await.unwrap();
        let l = get_budget_limit(&db, "p1").await.unwrap().unwrap();
        assert_eq!((l.monthly_usd, l.legacy_monthly_tokens), (Some(3.0), None));
        let m = budget_status(&db, "p1", &now).await.unwrap().meter.unwrap();
        assert!((m.used_usd - 2.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn old_rows_without_cost_count_as_unpriced_and_clearing_keeps_this_months_text_calls() {
        let db = pool().await;
        let now = Local::now();
        // 이 기능 이전의 줄(금액 없음, 토큰 있음): 0원으로 세되 "못 넣은 호출"로 알린다
        usage_at(&db, UsageKind::Chat, now.timestamp_millis(), None, false).await;
        usage_at(&db, UsageKind::Chat, now.timestamp_millis(), Some(0.4), false).await;
        usage_at(&db, UsageKind::Image, now.timestamp_millis(), None, false).await;
        upsert_budget_limit(&db, &limit(1.0)).await.unwrap();
        let m = budget_status_now(&db, "p1").await.unwrap().meter.unwrap();
        assert!((m.used_usd - 0.4).abs() < 1e-9);
        assert_eq!(m.unpriced_calls, 1);
        // 사용량 비우기: 금액 한도가 있으면 이번 달 글 호출은 남아 한도가 0으로 돌아가지 않는다 (그림 줄은 지워짐)
        crate::db::usage::clear(&db).await.unwrap();
        let m = budget_status_now(&db, "p1").await.unwrap().meter.unwrap();
        assert!((m.used_usd - 0.4).abs() < 1e-9);
        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usage_log").fetch_one(&db).await.unwrap();
        assert_eq!(left, 2);
        // 한도를 지우면 다 지워짐
        delete_budget_limit(&db, "p1").await.unwrap();
        crate::db::usage::clear(&db).await.unwrap();
        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usage_log").fetch_one(&db).await.unwrap();
        assert_eq!(left, 0);
    }

    #[tokio::test]
    async fn crud_and_cascade_on_profile_delete() {
        let db = pool().await;
        let l = limit(5.0);
        upsert_budget_limit(&db, &l).await.unwrap();
        assert_eq!(get_budget_limit(&db, "p1").await.unwrap(), Some(l));
        assert_eq!(get_all_budget_limits(&db).await.unwrap().len(), 1);
        sqlx::query("PRAGMA foreign_keys = ON").execute(&db).await.unwrap();
        sqlx::query("DELETE FROM api_profiles WHERE id = 'p1'").execute(&db).await.unwrap();
        assert!(get_budget_limit(&db, "p1").await.unwrap().is_none());
        delete_budget_limit(&db, "p1").await.unwrap();
    }
}
