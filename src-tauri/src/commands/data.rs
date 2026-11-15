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

/// 끌어다 놓은 파일을 디스크에 저장하고 경로를 돌려준다 (웹뷰의 드롭은 경로 없이 내용만 줌).
/// - `keep = true`: 메시지 첨부 이미지. 앱 데이터의 attachments 폴더에 계속 둔다
/// - `keep = false`: 가져오기용 임시 파일. 원래 이름을 살려 임시 폴더에 두고, 하루 지난 것은 지운다
#[tauri::command]
#[specta::specta]
pub async fn store_dropped_file(app: AppHandle, name: String, data_base64: String, keep: bool) -> Result<String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.split_once(',').map(|(_, b)| b).unwrap_or(&data_base64))
        .map_err(|e| crate::error::AppError::validation(format!("파일 내용을 읽을 수 없습니다: {e}"), None))?;
    let file_name = safe_file_name(&name);
    let path = if keep {
        let ext = std::path::Path::new(&file_name).extension().and_then(|e| e.to_str()).unwrap_or("bin").to_ascii_lowercase();
        app_data_dir(&app)?.join("attachments").join(format!("{}.{ext}", uuid::Uuid::new_v4()))
    } else {
        let root = std::env::temp_dir().join("sandcastle-drop");
        clean_old(&root);
        root.join(uuid::Uuid::new_v4().to_string()).join(&file_name)
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| crate::error::AppError::io(format!("폴더를 만들 수 없습니다: {e}")))?;
    }
    std::fs::write(&path, bytes).map_err(|e| crate::error::AppError::io(format!("파일을 쓸 수 없습니다: {e}")))?;
    Ok(path.to_string_lossy().into_owned())
}

/// 경로 구분자·예약 문자를 뺀 파일 이름
fn safe_file_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let cleaned: String = base
        .chars()
        .map(|c| if matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') || c.is_control() { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_string();
    if cleaned.is_empty() { "file".into() } else { cleaned }
}

/// 임시 드롭 폴더에서 하루 지난 것 지우기
fn clean_old(root: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    let day = std::time::Duration::from_secs(24 * 60 * 60);
    for e in entries.flatten() {
        let old = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|d| d > day);
        if old {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

#[cfg(test)]
mod drop_tests {
    use super::safe_file_name;

    #[test]
    fn file_names_are_made_safe() {
        assert_eq!(safe_file_name("../../evil.png"), "evil.png");
        assert_eq!(safe_file_name(r"C:\x\카드 v2.png"), "카드 v2.png");
        assert_eq!(safe_file_name("a:b?.json"), "a_b_.json");
        assert_eq!(safe_file_name(".."), "file");
    }
}


/// 캐릭터를 SillyTavern 카드로 내보내기. format: "json" | "png" (아바타에 카드를 넣은 그림)
#[tauri::command]
#[specta::specta]
pub async fn export_character_card(state: State<'_, AppState>, id: String, format: String, path: String) -> Result<()> {
    crate::service::data::card::export_card(&state.db, &id, &format, &PathBuf::from(path)).await
}
