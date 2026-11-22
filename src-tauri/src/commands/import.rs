//! 가져오기 커맨드
use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::{
    commands::character_image::app_data_dir,
    error::Result,
    service::import::sillytavern::{detect_file, import_file_with, ImportOptions, SillyTavernImport, SillyTavernKind},
    state::AppState,
};

/// SillyTavern 파일 가져오기 (캐릭터 카드 PNG/JSON, 프리셋, 정규식, 월드인포, 채팅 .jsonl 자동 판별).
/// - `name`: 프리셋/로어북/대화 이름. 비우면 파일 이름을 쓴다.
/// - `character_id`: 채팅을 붙일 캐릭터. 비우면 메시지의 캐릭터 이름으로 찾고,
///   못 찾으면 오류의 details.speakers에 화자 이름 목록을 담아 돌려준다 (프론트가 캐릭터를 고르게 하면 됨).
/// - `preset_id`: 정규식 파일을 이 프리셋에 묶어 가져온다 (프리셋과 함께 고른 정규식). 비우면 전역.
#[tauri::command]
#[specta::specta]
pub async fn import_sillytavern(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    name: Option<String>,
    character_id: Option<String>,
    preset_id: Option<String>,
) -> Result<SillyTavernImport> {
    let base = app_data_dir(&app)?;
    import_file_with(&state.db, &base, &PathBuf::from(path), ImportOptions { name, character_id, preset_id }).await
}

/// SillyTavern 파일의 종류만 본다 (가져오지 않음)
#[tauri::command]
#[specta::specta]
pub fn detect_sillytavern(path: String) -> Result<SillyTavernKind> {
    detect_file(&PathBuf::from(path))
}

/// SillyTavern 전체 데이터 가져오기 (사용자 폴더 `data/default-user`를 압축한 zip).
/// 캐릭터·채팅·로어북·프리셋·정규식·페르소나를 새로 만들기만 한다 (지금 데이터는 그대로).
#[tauri::command]
#[specta::specta]
pub async fn import_sillytavern_data(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<crate::service::import::sillytavern::data::SillyTavernDataImport> {
    let base = app_data_dir(&app)?;
    crate::service::import::sillytavern::data::import_data_zip(&state.db, &base, &PathBuf::from(path)).await
}

/// 샌드캐슬 데이터를 SillyTavern 사용자 폴더 모양 zip으로 (캐릭터 카드·채팅·로어북·페르소나·전역 정규식)
#[tauri::command]
#[specta::specta]
pub async fn export_sillytavern_data(
    state: State<'_, AppState>,
    path: String,
) -> Result<crate::service::import::sillytavern::data_export::SillyTavernDataExport> {
    crate::service::import::sillytavern::data_export::export_data_zip(&state.db, &PathBuf::from(path)).await
}
