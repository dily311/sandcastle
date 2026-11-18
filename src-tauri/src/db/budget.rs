//! 예산 한도 (budget_limits) — ResourceManager 2차 (CLAUDE.md 1-A-1, ADR-007).
//!
//! API 프로필별 하루/한 달 상한을 저장하고, 기간별 사용량 대비 남은 양을 계산한다.
//! 값이 없으면(NULL·행 없음) 한도 없음. 하루·한 달은 기기의 현지 시각 기준.
use chrono::{DateTime, Datelike, Local, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Sqlite};

use super::usage::UsageKind;

/// 한 프로필의 한도. None = 한도 없음
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetLimit {
    pub api_profile_id: String,
    pub daily_tokens: Option<i64>,
    pub monthly_tokens: Option<i64>,
    pub daily_characters: Option<i64>,
    pub monthly_characters: Option<i64>,
    pub daily_images: Option<i64>,
    pub monthly_images: Option<i64>,
}

impl BudgetLimit {
    /// 모든 칸이 비었는지 (= 한도 없음, 행을 지워도 됨)
    pub fn is_empty(&self) -> bool {
        [
            self.daily_tokens,
            self.monthly_tokens,
            self.daily_characters,
            self.monthly_characters,
            self.daily_images,
            self.monthly_images,
        ]
        .iter()
        .all(Option::is_none)
    }
}

pub async fn get_budget_limit<'e, E>(executor: E, api_profile_id: &str) -> Result<Option<BudgetLimit>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, BudgetLimit>(
        "SELECT api_profile_id, daily_tokens, monthly_tokens, daily_characters, monthly_characters, daily_images, monthly_images
         FROM budget_limits WHERE api_profile_id = ?",
    )
    .bind(api_profile_id)
    .fetch_optional(executor)
    .await
}

pub async fn get_all_budget_limits<'e, E>(executor: E) -> Result<Vec<BudgetLimit>, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, BudgetLimit>(
        "SELECT api_profile_id, daily_tokens, monthly_tokens, daily_characters, monthly_characters, daily_images, monthly_images
         FROM budget_limits ORDER BY api_profile_id",
    )
    .fetch_all(executor)
    .await
}

/// 저장. 0 이하 값은 "한도 없음"으로 본다 (실수로 0을 넣어 모든 호출이 막히지 않게 — 막으려면 1)
pub async fn upsert_budget_limit<'e, E>(executor: E, limit: &BudgetLimit) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let pos = |v: Option<i64>| v.filter(|n| *n > 0);
    sqlx::query(
        "INSERT INTO budget_limits (api_profile_id, daily_tokens, monthly_tokens, daily_characters, monthly_characters, daily_images, monthly_images, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(api_profile_id) DO UPDATE SET
           daily_tokens = excluded.daily_tokens, monthly_tokens = excluded.monthly_tokens,
           daily_characters = excluded.daily_characters, monthly_characters = excluded.monthly_characters,
           daily_images = excluded.daily_images, monthly_images = excluded.monthly_images,
           updated_at = excluded.updated_at",
    )
    .bind(&limit.api_profile_id)
    .bind(pos(limit.daily_tokens))
    .bind(pos(limit.monthly_tokens))
    .bind(pos(limit.daily_characters))
    .bind(pos(limit.monthly_characters))
    .bind(pos(limit.daily_images))
    .bind(pos(limit.monthly_images))
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
// 기간별 사용량 대비 남은 양
// ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BudgetMetric {
    Tokens,
    Characters,
    Images,
}

impl BudgetMetric {
    /// 이 호출 종류가 쓰는 단위
    pub fn for_kind(kind: UsageKind) -> Self {
        match kind {
            UsageKind::Chat => Self::Tokens,
            UsageKind::Tts => Self::Characters,
            UsageKind::Image => Self::Images,
            // 임베딩은 토큰 한도에 같이 센다
            UsageKind::Embedding => Self::Tokens,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BudgetPeriod {
    Day,
    Month,
}

/// 한도가 걸린 칸 하나 (한도 없는 칸은 만들지 않는다)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetMeter {
    pub metric: BudgetMetric,
    pub period: BudgetPeriod,
    pub used: i64,
    pub limit: i64,
    /// 0 이상 (넘었으면 0)
    pub remaining: i64,
}

impl BudgetMeter {
    pub fn exceeded(&self) -> bool {
        self.used >= self.limit
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BudgetStatus {
    pub api_profile_id: String,
    pub meters: Vec<BudgetMeter>,
}

impl BudgetStatus {
    /// 이 호출 종류를 막는 칸 (가장 먼저 넘은 것: 하루 → 한 달)
    pub fn blocking(&self, kind: UsageKind) -> Option<&BudgetMeter> {
        let metric = BudgetMetric::for_kind(kind);
        self.meters.iter().find(|m| m.metric == metric && m.exceeded())
    }
}

/// 기간별 사용량 (토큰 = 입력+출력)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PeriodUsage {
    pub tokens: i64,
    pub characters: i64,
    pub images: i64,
}

/// 오늘 0시와 이번 달 1일 0시 (현지 시각, ms)
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

/// since 이후 이 프로필의 사용량. 실패한 호출도 센다(요금이 나갈 수 있음).
/// 정리된 일별 합계(usage_daily)도 더한다 — 정리는 하루 단위로 자르므로 기간 시작(0시)과 맞는다
pub async fn usage_since<'e, E>(executor: E, api_profile_id: &str, since: i64) -> Result<PeriodUsage, sqlx::Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let row: (i64, i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(t), 0), COALESCE(SUM(c), 0), COALESCE(SUM(i), 0) FROM (
             SELECT input_tokens + output_tokens AS t, characters AS c, images AS i FROM usage_log WHERE api_profile_id = ?1 AND created_at >= ?2
             UNION ALL
             SELECT input_tokens + output_tokens, characters, images FROM usage_daily WHERE api_profile_id = ?1 AND day_start >= ?2
         )",
    )
    .bind(api_profile_id)
    .bind(since)
    .fetch_one(executor)
    .await?;
    Ok(PeriodUsage {
        tokens: row.0,
        characters: row.1,
        images: row.2,
    })
}

/// 한도와 두 기간의 사용량으로 칸 목록을 만든다 (순수 함수)
pub fn meters(limit: &BudgetLimit, day: PeriodUsage, month: PeriodUsage) -> Vec<BudgetMeter> {
    let cells = [
        (BudgetMetric::Tokens, BudgetPeriod::Day, limit.daily_tokens, day.tokens),
        (BudgetMetric::Tokens, BudgetPeriod::Month, limit.monthly_tokens, month.tokens),
        (BudgetMetric::Characters, BudgetPeriod::Day, limit.daily_characters, day.characters),
        (BudgetMetric::Characters, BudgetPeriod::Month, limit.monthly_characters, month.characters),
        (BudgetMetric::Images, BudgetPeriod::Day, limit.daily_images, day.images),
        (BudgetMetric::Images, BudgetPeriod::Month, limit.monthly_images, month.images),
    ];
    cells
        .into_iter()
        .filter_map(|(metric, period, limit, used)| {
            let limit = limit.filter(|n| *n > 0)?;
            Some(BudgetMeter {
                metric,
                period,
                used,
                limit,
                remaining: (limit - used).max(0),
            })
        })
        .collect()
}

/// 이 프로필의 지금 상태 (한도가 없으면 meters가 빈 목록)
pub async fn budget_status<Tz: TimeZone>(pool: &sqlx::SqlitePool, api_profile_id: &str, now: &DateTime<Tz>) -> Result<BudgetStatus, sqlx::Error> {
    let Some(limit) = get_budget_limit(pool, api_profile_id).await?.filter(|l| !l.is_empty()) else {
        return Ok(BudgetStatus {
            api_profile_id: api_profile_id.to_string(),
            meters: vec![],
        });
    };
    let (day_start, month_start) = period_starts(now);
    let (day, month) = futures::try_join!(usage_since(pool, api_profile_id, day_start), usage_since(pool, api_profile_id, month_start))?;
    Ok(BudgetStatus {
        api_profile_id: api_profile_id.to_string(),
        meters: meters(&limit, day, month),
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

    async fn usage_at(db: &sqlx::SqlitePool, kind: UsageKind, at: i64, tokens: u64, chars: u64, images: u64) {
        let mut e = UsageEntry::new(kind, "p1", "m");
        (e.input_tokens, e.output_tokens, e.characters, e.images) = (tokens, 0, chars, images);
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

    #[test]
    fn period_starts_are_local_midnight_and_month_first() {
        let now = kst(2026, 11, 16, 15);
        let (day, month) = period_starts(&now);
        assert_eq!(day, kst(2026, 11, 16, 0).timestamp_millis());
        assert_eq!(month, kst(2026, 11, 1, 0).timestamp_millis());
    }

    #[test]
    fn meters_skip_empty_and_non_positive_limits() {
        let limit = BudgetLimit {
            api_profile_id: "p1".into(),
            daily_tokens: Some(100),
            monthly_images: Some(0),
            ..Default::default()
        };
        let m = meters(
            &limit,
            PeriodUsage {
                tokens: 130,
                ..Default::default()
            },
            PeriodUsage::default(),
        );
        assert_eq!(m.len(), 1);
        assert_eq!((m[0].used, m[0].limit, m[0].remaining, m[0].exceeded()), (130, 100, 0, true));
    }

    #[tokio::test]
    async fn status_counts_day_and_month_separately() {
        let db = pool().await;
        let now = kst(2026, 11, 16, 15);
        usage_at(&db, UsageKind::Chat, kst(2026, 10, 31, 23).timestamp_millis(), 1000, 0, 0).await; // 지난달 — 안 셈
        usage_at(&db, UsageKind::Chat, kst(2026, 11, 3, 12).timestamp_millis(), 300, 0, 0).await; // 이번 달
        usage_at(&db, UsageKind::Chat, kst(2026, 11, 16, 9).timestamp_millis(), 50, 0, 0).await; // 오늘
        usage_at(&db, UsageKind::Image, kst(2026, 11, 16, 10).timestamp_millis(), 0, 0, 2).await;

        // 한도 없음 → 칸 없음
        assert!(budget_status(&db, "p1", &now).await.unwrap().meters.is_empty());

        upsert_budget_limit(
            &db,
            &BudgetLimit {
                api_profile_id: "p1".into(),
                daily_tokens: Some(100),
                monthly_tokens: Some(300),
                daily_images: Some(5),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let s = budget_status(&db, "p1", &now).await.unwrap();
        let get = |metric, period| s.meters.iter().find(|m| m.metric == metric && m.period == period).unwrap().clone();
        assert_eq!(get(BudgetMetric::Tokens, BudgetPeriod::Day).used, 50);
        assert_eq!(get(BudgetMetric::Tokens, BudgetPeriod::Day).remaining, 50);
        assert_eq!(get(BudgetMetric::Tokens, BudgetPeriod::Month).used, 350);
        assert_eq!(get(BudgetMetric::Images, BudgetPeriod::Day).remaining, 3);
        // 채팅은 한 달 토큰으로 막히고, 그림은 막히지 않는다
        assert_eq!(s.blocking(UsageKind::Chat).map(|m| m.period), Some(BudgetPeriod::Month));
        assert!(s.blocking(UsageKind::Image).is_none());
        assert!(s.blocking(UsageKind::Tts).is_none());
    }

    #[tokio::test]
    async fn crud_and_cascade_on_profile_delete() {
        let db = pool().await;
        let l = BudgetLimit {
            api_profile_id: "p1".into(),
            monthly_characters: Some(5000),
            ..Default::default()
        };
        upsert_budget_limit(&db, &l).await.unwrap();
        assert_eq!(get_budget_limit(&db, "p1").await.unwrap(), Some(l));
        assert_eq!(get_all_budget_limits(&db).await.unwrap().len(), 1);
        sqlx::query("PRAGMA foreign_keys = ON").execute(&db).await.unwrap();
        sqlx::query("DELETE FROM api_profiles WHERE id = 'p1'").execute(&db).await.unwrap();
        assert!(get_budget_limit(&db, "p1").await.unwrap().is_none());
        delete_budget_limit(&db, "p1").await.unwrap();
    }
}
