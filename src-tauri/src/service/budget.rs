//! 예산 한도 확인 (CLAUDE.md 1-A-2, ADR-007).
//!
//! 공급자를 실제로 부르기 직전(AiService::generate, 음성·이미지)에 `ensure`를 부른다.
//! - 한도를 넘었으면 `BUDGET_EXCEEDED`로 거절 (재시도 안 함, 공급자는 부르지 않음)
//! - 자동 실행(run_by = auto)은 항상 막는다
//! - 사용자 실행(user)은 "이번만 계속"(`allow_once`)을 누른 뒤 2분 안의 작업 하나만 통과
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use serde_json::json;

use crate::{
    db::{
        budget::{self, BudgetMeter, BudgetMetric, BudgetPeriod},
        usage::UsageKind,
    },
    error::{AppError, ErrorCode, Result},
    task::{current_task, RunBy},
};

/// "이번만 계속"을 누르고 다시 실행하기까지 기다리는 시간
const ALLOW_WINDOW: Duration = Duration::from_secs(120);

#[derive(Debug)]
struct Allowance {
    until: Instant,
    /// 허용을 처음 쓴 작업 (같은 작업의 다음 호출도 통과)
    task_id: Option<String>,
}

static ALLOW: Mutex<Option<Allowance>> = Mutex::new(None);

fn lock() -> std::sync::MutexGuard<'static, Option<Allowance>> {
    ALLOW.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// 사용자가 "이번만 계속"을 누름: 2분 안에 사용자가 실행하는 작업 하나를 한도 밖으로
pub fn allow_once() {
    *lock() = Some(Allowance { until: Instant::now() + ALLOW_WINDOW, task_id: None });
}

/// 이 작업이 허용을 쓸 수 있는지 (처음 쓰는 작업이 허용을 차지한다)
fn take_allowance(task_id: Option<&str>) -> bool {
    let mut slot = lock();
    let Some(a) = slot.as_mut() else { return false };
    if Instant::now() > a.until {
        *slot = None;
        return false;
    }
    match (&a.task_id, task_id) {
        (None, Some(id)) => {
            a.task_id = Some(id.to_string());
            true
        }
        (Some(owner), Some(id)) => owner == id,
        // 작업 밖의 호출: 한 번 쓰고 끝
        (_, None) => {
            *slot = None;
            true
        }
    }
}

/// 이 호출을 해도 되는지. 넘었으면 BUDGET_EXCEEDED
pub async fn ensure(db: &sqlx::SqlitePool, api_profile_id: &str, kind: UsageKind) -> Result<()> {
    let status = budget::budget_status_now(db, api_profile_id).await?;
    let Some(meter) = status.blocking(kind) else { return Ok(()) };
    let task = current_task();
    let run_by = task.as_ref().map_or(RunBy::User, |t| t.run_by);
    if run_by == RunBy::User && take_allowance(task.as_ref().map(|t| t.task_id.as_str())) {
        return Ok(());
    }
    Err(exceeded(api_profile_id, meter, run_by))
}

fn exceeded(api_profile_id: &str, m: &BudgetMeter, run_by: RunBy) -> AppError {
    let what = match m.metric {
        BudgetMetric::Tokens => "토큰",
        BudgetMetric::Characters => "음성 글자",
        BudgetMetric::Images => "그림",
    };
    let when = match m.period {
        BudgetPeriod::Day => "오늘",
        BudgetPeriod::Month => "이번 달",
    };
    let tail = if run_by == RunBy::Auto { " 자동 실행은 실행하지 않았습니다." } else { "" };
    AppError::Validation {
        code: ErrorCode::BudgetExceeded,
        message: format!("{when} {what} 한도를 넘어 실행하지 않았습니다 ({} / {}).{tail}", m.used, m.limit),
        details: Some(json!({
            "apiProfileId": api_profile_id,
            "metric": m.metric,
            "period": m.period,
            "used": m.used,
            "limit": m.limit,
            "canOverride": run_by == RunBy::User,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::{budget::{upsert_budget_limit, BudgetLimit}, usage::{record, UsageEntry}},
        task::{task_context_scope_for_test as scope, CurrentTask},
    };

    async fn pool_over_limit() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        sqlx::query("INSERT INTO api_profiles (id, name, provider, format_type, base_url, model, created_at, updated_at) VALUES ('p1', 'P', 'openai', 'openai', '', 'm', 0, 0)")
            .execute(&pool)
            .await
            .unwrap();
        upsert_budget_limit(&pool, &BudgetLimit { api_profile_id: "p1".into(), daily_tokens: Some(10), ..Default::default() }).await.unwrap();
        let mut e = UsageEntry::new(UsageKind::Chat, "p1", "m");
        e.input_tokens = 10;
        record(&pool, e).await;
        pool
    }

    fn task(id: &str, run_by: RunBy) -> CurrentTask {
        CurrentTask { task_id: id.into(), feature: Some("chat".into()), run_by }
    }

    fn code(r: Result<()>) -> Option<ErrorCode> {
        r.err().map(|e| e.to_response().code)
    }

    // 전역 허용 슬롯을 같이 쓰므로 한 테스트 안에서 순서대로 확인한다
    #[tokio::test]
    async fn blocks_auto_always_and_user_until_allowed_once() {
        let db = pool_over_limit().await;
        *lock() = None;
        // 다른 단위(그림)는 막지 않는다
        assert!(scope(task("t0", RunBy::User), ensure(&db, "p1", UsageKind::Image)).await.is_ok());
        // 사용자 실행도 허용 전에는 막힘
        assert_eq!(code(scope(task("t1", RunBy::User), ensure(&db, "p1", UsageKind::Chat)).await), Some(ErrorCode::BudgetExceeded));

        allow_once();
        // 자동 실행은 허용이 있어도 막고, 허용을 써 버리지도 않는다
        assert_eq!(code(scope(task("a1", RunBy::Auto), ensure(&db, "p1", UsageKind::Chat)).await), Some(ErrorCode::BudgetExceeded));
        // 처음 실행한 사용자 작업이 허용을 차지 — 같은 작업 안의 다음 호출도 통과
        assert!(scope(task("t2", RunBy::User), ensure(&db, "p1", UsageKind::Chat)).await.is_ok());
        assert!(scope(task("t2", RunBy::User), ensure(&db, "p1", UsageKind::Chat)).await.is_ok());
        // 다른 작업은 다시 막힘
        assert_eq!(code(scope(task("t3", RunBy::User), ensure(&db, "p1", UsageKind::Chat)).await), Some(ErrorCode::BudgetExceeded));
        // 한도가 없는 프로필은 그대로 통과
        assert!(ensure(&db, "none", UsageKind::Chat).await.is_ok());
        *lock() = None;
    }
}
