//! 가져오기 커맨드
use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::{
    commands::character_image::app_data_dir,
    error::Result,
    service::import::sillytavern::{import_file, SillyTavernImport},
    state::AppState,
};

/// SillyTavern 파일 가져오기 (캐릭터 카드 PNG/JSON, 프리셋, 정규식, 월드인포 자동 판별).
/// `name`: 프리셋/로어북 이름. 비우면 파일 이름을 쓴다.
#[tauri::command]
pub async fn import_sillytavern(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    name: Option<String>,
) -> Result<SillyTavernImport> {
    let base = app_data_dir(&app)?;
    import_file(&state.db, &base, &PathBuf::from(path), name).await
}
