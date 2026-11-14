//! 가져오기 커맨드
use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::{
    commands::character_image::app_data_dir,
    error::Result,
    service::import::sillytavern::{import_file_with, ImportOptions, SillyTavernImport},
    state::AppState,
};

/// SillyTavern 파일 가져오기 (캐릭터 카드 PNG/JSON, 프리셋, 정규식, 월드인포, 채팅 .jsonl 자동 판별).
/// - `name`: 프리셋/로어북/대화 이름. 비우면 파일 이름을 쓴다.
/// - `character_id`: 채팅을 붙일 캐릭터. 비우면 메시지의 캐릭터 이름으로 찾고,
///   못 찾으면 오류의 details.speakers에 화자 이름 목록을 담아 돌려준다 (프론트가 캐릭터를 고르게 하면 됨).
#[tauri::command]
pub async fn import_sillytavern(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    name: Option<String>,
    character_id: Option<String>,
) -> Result<SillyTavernImport> {
    let base = app_data_dir(&app)?;
    import_file_with(&state.db, &base, &PathBuf::from(path), ImportOptions { name, character_id }).await
}
