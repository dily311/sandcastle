//! 작업 관리 커맨드. 모두 TaskManager에 "요청"만 하고, 상태 변경은 TaskManager가 한다.
use std::sync::Arc;

use tauri::State;

use crate::{
    error::Result,
    task::{TaskInfo, TaskManager},
};

#[tauri::command]
#[specta::specta]
pub async fn get_task(manager: State<'_, Arc<TaskManager>>, task_id: String) -> Result<TaskInfo> {
    manager.get_task(&task_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_active_tasks(manager: State<'_, Arc<TaskManager>>) -> Result<Vec<TaskInfo>> {
    manager.list_active().await
}

/// 앱 시작 후 프론트가 호출: 재개/포기를 물어볼 중단된 작업 목록
#[tauri::command]
#[specta::specta]
pub async fn get_recoverable_tasks(manager: State<'_, Arc<TaskManager>>) -> Result<Vec<TaskInfo>> {
    manager.list_recoverable().await
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_task(manager: State<'_, Arc<TaskManager>>, task_id: String) -> Result<()> {
    manager.cancel(&task_id).await
}

/// 재개 요청. 결과는 `task-event`로 전달되므로 접수된 작업 정보만 바로 돌려준다.
#[tauri::command]
#[specta::specta]
pub async fn resume_task(manager: State<'_, Arc<TaskManager>>, task_id: String) -> Result<TaskInfo> {
    let handle = manager.inner().resume_task(&task_id).await?;
    Ok(handle.task().clone())
}

#[tauri::command]
#[specta::specta]
pub async fn dismiss_task(manager: State<'_, Arc<TaskManager>>, task_id: String) -> Result<()> {
    manager.dismiss_task(&task_id).await
}

// ---------------------------------------------------------
// 실행 기록 · 사용량 (ResourceManager 1차)
// ---------------------------------------------------------

/// 최근 실행 내역 (최신 순). 작업마다 그 작업이 쓴 토큰·글자·그림 수 합계
#[tauri::command]
#[specta::specta]
pub async fn list_task_history(state: State<'_, crate::state::AppState>, limit: Option<u32>) -> Result<Vec<crate::db::usage::TaskHistoryItem>> {
    let limit = i64::from(limit.unwrap_or(100).clamp(1, 500));
    Ok(crate::db::usage::task_history(&state.db, limit).await?)
}

/// API 프로필·종류별 사용량 합계. since: 이 시각(ms) 이후만 (없으면 전부)
#[tauri::command]
#[specta::specta]
pub async fn usage_summary(state: State<'_, crate::state::AppState>, since: Option<f64>) -> Result<Vec<crate::db::usage::UsageSummary>> {
    Ok(crate::db::usage::summary(&state.db, since.map_or(0, |s| s as i64)).await?)
}

/// 사용량 기록 비우기 (작업 기록은 남는다)
#[tauri::command]
#[specta::specta]
pub async fn clear_usage(state: State<'_, crate::state::AppState>) -> Result<u32> {
    Ok(u32::try_from(crate::db::usage::clear(&state.db).await?).unwrap_or(u32::MAX))
}

/// 예산 한도로 막힌 사용자 실행을 "이번만 계속": 2분 안에 다시 실행하는 작업 하나만 한도 밖으로 (ADR-007).
/// 실행을 대신 시작하지 않는다 — 사용자가 다시 누른다
#[tauri::command]
#[specta::specta]
pub async fn allow_budget_once() -> Result<()> {
    crate::service::budget::allow_once();
    Ok(())
}

/// 보존 기간(설정)보다 오래된 작업·사용량 상세를 정리한다. 일별 합계는 남아 사용량 숫자는 그대로.
/// 보존 기간이 "계속 보관"이면 아무것도 하지 않는다
#[tauri::command]
#[specta::specta]
pub async fn compact_usage(
    state: State<'_, crate::state::AppState>,
    settings: State<'_, crate::sys::settings::SettingsState<crate::models::settings::AppSetting>>,
) -> Result<crate::db::usage::CompactResult> {
    let days = settings
        .0
        .lock()
        .map_err(|_| crate::error::AppError::Internal {
            message: "설정 잠금 실패".into(),
        })?
        .usage_retention_days;
    Ok(crate::service::budget::compact(&state.db, days).await?)
}

// ---------------------------------------------------------
// 단가표 (1-A-3)
// ---------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub async fn list_model_prices(state: State<'_, crate::state::AppState>) -> Result<Vec<crate::db::usage::ModelPrice>> {
    Ok(crate::db::usage::get_all_model_prices(&state.db).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn upsert_model_price(state: State<'_, crate::state::AppState>, price: crate::db::usage::ModelPrice) -> Result<()> {
    if price.model.trim().is_empty() {
        return Err(crate::error::AppError::validation("모델 이름이 비었습니다", None));
    }
    Ok(crate::db::usage::upsert_model_price(&state.db, &price).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_model_price(state: State<'_, crate::state::AppState>, model: String) -> Result<()> {
    Ok(crate::db::usage::delete_model_price(&state.db, &model).await?)
}

// ---------------------------------------------------------
// 예산 한도 (1-A-5)
// ---------------------------------------------------------

/// 한도가 있는 프로필과 지금 사용량 (한도 막대)
#[tauri::command]
#[specta::specta]
pub async fn list_budget_status(state: State<'_, crate::state::AppState>) -> Result<Vec<crate::service::budget::BudgetOverview>> {
    Ok(crate::service::budget::overview(&state.db).await?)
}

/// 이 모델의 단가 (사용자 단가표 → 앱 기본 단가). 없으면 null — 한도를 걸어도 금액을 셀 수 없다
#[tauri::command]
#[specta::specta]
pub async fn get_model_price(state: State<'_, crate::state::AppState>, model: String) -> Result<Option<crate::service::prices::ModelPriceInfo>> {
    Ok(crate::service::prices::info(&state.db, &model).await)
}

/// 한도 저장. 금액을 비우면 한도를 지운다
#[tauri::command]
#[specta::specta]
pub async fn upsert_budget_limit(state: State<'_, crate::state::AppState>, limit: crate::db::budget::BudgetLimit) -> Result<()> {
    use crate::db::budget;
    if limit.is_empty() {
        return Ok(budget::delete_budget_limit(&state.db, &limit.api_profile_id).await?);
    }
    Ok(budget::upsert_budget_limit(&state.db, &limit).await?)
}

/// 새로 80%에 닿은 한도 (작업이 끝날 때 프론트가 물어 앱 안 알림으로)
#[tauri::command]
#[specta::specta]
pub async fn take_budget_warnings(state: State<'_, crate::state::AppState>) -> Result<Vec<crate::service::budget::BudgetWarning>> {
    Ok(crate::service::budget::take_warnings(&state.db).await?)
}
