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
    import_file_with(
        &state.db,
        &base,
        &PathBuf::from(path),
        ImportOptions {
            name,
            character_id,
            preset_id,
            ..Default::default()
        },
    )
    .await
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

/// 프리셋 하나를 SillyTavern Chat Completion 프리셋(.json)으로 (프리셋에 묶인 정규식 포함)
#[tauri::command]
#[specta::specta]
pub async fn export_preset_sillytavern(state: State<'_, AppState>, id: String, path: String) -> Result<()> {
    let (_, json) = crate::service::import::sillytavern::preset::preset_st_json(&state.db, &id).await?;
    let text = serde_json::to_string_pretty(&json).map_err(|e| crate::error::AppError::internal(e.to_string()))?;
    std::fs::write(PathBuf::from(path), text).map_err(|e| crate::error::AppError::io(format!("파일을 쓸 수 없습니다: {e}")))
}

/// SillyTavern 빠른 답장 파일 읽기 (세트 .json 또는 버튼 하나 .qr.json, 여러 개).
/// 세트 파일은 각각 세트로, 버튼 파일들은 모아서 세트 하나("가져온 빠른 답장")로. 저장은 화면이 upsert_quick_reply_set으로 한다
#[tauri::command]
#[specta::specta]
pub fn read_quick_reply_files(paths: Vec<String>) -> Result<(Vec<crate::models::quick_reply::QuickReplySetInput>, Vec<String>)> {
    use crate::service::import::sillytavern::quick_reply as qr;
    let mut sets = vec![];
    let mut singles = vec![];
    let mut warnings = vec![];
    for path in paths.iter().map(PathBuf::from) {
        let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let read = || -> Result<serde_json::Value> {
            let meta = std::fs::metadata(&path).map_err(|e| crate::error::AppError::io(format!("파일을 열 수 없습니다: {e}")))?;
            if meta.len() > 10 * 1024 * 1024 {
                return Err(crate::error::AppError::validation("파일이 너무 큽니다 (10MB 이하)", None));
            }
            serde_json::from_slice(&std::fs::read(&path)?).map_err(|e| crate::error::AppError::validation(format!("JSON 파일을 읽을 수 없습니다: {e}"), None))
        };
        let json = match read() {
            Ok(v) => v,
            Err(e) => {
                warnings.push(format!("{name}: {e}"));
                continue;
            }
        };
        if qr::is_single_qr(&json) {
            if let Some(q) = qr::map_qr_item(&json, &mut warnings) {
                singles.push(q);
            }
            continue;
        }
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "빠른 답장".into());
        match qr::map_qr_set(&json, &stem, &mut warnings) {
            Ok(set) => sets.push(set),
            Err(e) => warnings.push(format!("{name}: {e}")),
        }
    }
    if !singles.is_empty() {
        sets.push(crate::models::quick_reply::QuickReplySetInput {
            id: None,
            name: "가져온 빠른 답장".into(),
            scope: "global".into(),
            scope_id: None,
            enabled: true,
            auto_send: false,
            items: singles,
            sort_order: 0,
        });
    }
    if sets.is_empty() && !warnings.is_empty() {
        return Err(crate::error::AppError::validation(warnings.join(" · "), None));
    }
    Ok((sets, warnings))
}
