//! 데이터 관리 커맨드: 조회 / 캐릭터 팩 / 전체 백업·복원
use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::{
    commands::character_image::app_data_dir,
    error::Result,
    service::data::{
        backup::{self, BackupManifest, BackupSummary, RestorePreview},
        overview::{self, DataOverview},
        pack::{self, PackExportSummary, PackImportSummary},
    },
    state::AppState,
};

/// 캐릭터 → 채팅 트리, 개수, 저장 공간
#[tauri::command]
#[specta::specta]
pub async fn get_data_overview(app: AppHandle, state: State<'_, AppState>) -> Result<DataOverview> {
    overview::data_overview(&state.db, &app_data_dir(&app)?).await
}

/// 여러 대화를 한 번에 삭제. 지운 개수를 돌려준다.
#[tauri::command]
#[specta::specta]
pub async fn delete_conversations(state: State<'_, AppState>, ids: Vec<String>) -> Result<u64> {
    overview::delete_conversations(&state.db, &ids).await
}

/// 캐릭터 팩 내보내기. `path`는 프론트의 저장 대화상자에서 고른 경로 (.scpack).
/// `conversation_ids`를 주면 그 대화만, 생략하면 고른 캐릭터의 모든 대화.
#[tauri::command]
#[specta::specta]
pub async fn export_character_pack(
    app: AppHandle,
    state: State<'_, AppState>,
    character_ids: Vec<String>,
    conversation_ids: Option<Vec<String>>,
    path: String,
) -> Result<PackExportSummary> {
    pack::export_pack(
        &state.db,
        &app_data_dir(&app)?,
        &character_ids,
        conversation_ids.as_deref(),
        &PathBuf::from(path),
    )
    .await
}

/// 캐릭터 팩 가져오기 (기존 데이터와 병합, id는 새로 발급)
#[tauri::command]
#[specta::specta]
pub async fn import_character_pack(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<PackImportSummary> {
    pack::import_pack(&state.db, &app_data_dir(&app)?, &PathBuf::from(path)).await
}

/// 전체 백업 (.scbackup)
#[tauri::command]
#[specta::specta]
pub async fn export_backup(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<BackupSummary> {
    backup::export_backup(&state.db, &app_data_dir(&app)?, &PathBuf::from(path)).await
}

/// 복원 준비: 백업을 검사해서 스테이징에 풀어 둔다. 실제 교체는 `restart_app` 후 시작할 때.
#[tauri::command]
#[specta::specta]
pub async fn prepare_restore(app: AppHandle, path: String) -> Result<RestorePreview> {
    backup::prepare_restore(&app_data_dir(&app)?, &PathBuf::from(path)).await
}

/// 준비해 둔 복원 취소
#[tauri::command]
#[specta::specta]
pub fn cancel_restore(app: AppHandle) -> Result<()> {
    backup::cancel_restore(&app_data_dir(&app)?);
    Ok(())
}

/// 재시작을 기다리는 복원 정보 (없으면 null)
#[tauri::command]
#[specta::specta]
pub fn get_pending_restore(app: AppHandle) -> Result<Option<BackupManifest>> {
    Ok(backup::pending_restore(&app_data_dir(&app)?))
}

/// 앱 재시작 (복원 적용용)
#[tauri::command]
#[specta::specta]
pub fn restart_app(app: AppHandle) {
    app.restart();
}
