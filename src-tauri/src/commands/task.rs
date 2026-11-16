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
