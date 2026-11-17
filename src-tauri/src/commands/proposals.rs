//! 고치기 제안 커맨드 (CLAUDE.md 6-D-3, ADR-012). 사용자만 적용·거절·되돌리기를 한다(플러그인 API에는 없음).
use tauri::{AppHandle, Emitter, State};

use crate::{
    error::Result,
    service::proposals::{self, ChangeProposal},
    state::AppState,
};

/// 제안이 생기거나 골랐을 때 — 화면이 목록·알림을 새로 고친다
pub const CHANGED_EVENT: &str = "change-proposals-changed";

pub fn changed(app: &AppHandle) {
    let _ = app.emit(CHANGED_EVENT, ());
}

/// 제안 목록 (새것부터). status: pending / applied / rejected / reverted, 비우면 전부
#[tauri::command]
#[specta::specta]
pub async fn list_change_proposals(state: State<'_, AppState>, status: Option<String>) -> Result<Vec<ChangeProposal>> {
    proposals::list(&state.db, status.as_deref()).await
}

/// 기다리는 제안 수
#[tauri::command]
#[specta::specta]
pub async fn count_pending_proposals(state: State<'_, AppState>) -> Result<i64> {
    proposals::pending_count(&state.db).await
}

/// 적용 (그 사이 원본이 바뀌었으면 거절)
#[tauri::command]
#[specta::specta]
pub async fn apply_change_proposal(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<ChangeProposal> {
    let p = proposals::apply(&state.db, &id).await?;
    changed(&app);
    Ok(p)
}

#[tauri::command]
#[specta::specta]
pub async fn reject_change_proposal(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<ChangeProposal> {
    let p = proposals::reject(&state.db, &id).await?;
    changed(&app);
    Ok(p)
}

/// 적용한 것을 되돌림 (그 뒤 원본이 또 바뀌었으면 거절)
#[tauri::command]
#[specta::specta]
pub async fn revert_change_proposal(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<ChangeProposal> {
    let p = proposals::revert(&state.db, &id).await?;
    changed(&app);
    Ok(p)
}
