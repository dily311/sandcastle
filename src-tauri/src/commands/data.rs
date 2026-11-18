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
        trash::{self, TrashItem, TrashKind},
    },
    state::AppState,
};

/// 캐릭터 → 채팅 트리, 개수, 저장 공간
#[tauri::command]
#[specta::specta]
pub async fn get_data_overview(app: AppHandle, state: State<'_, AppState>) -> Result<DataOverview> {
    overview::data_overview(&state.db, &app_data_dir(&app)?).await
}

/// 고른 파일(Android content:// 주소)을 앱 임시 폴더로 복사해 경로로 (1-C-2). 경로면 그대로.
/// extensions: 고르기 필터의 확장자 (내용으로 확장자를 정할 때 이 안에서)
#[tauri::command]
#[specta::specta]
pub async fn stage_picked_file(app: AppHandle, uri: String, extensions: Vec<String>) -> Result<String> {
    let base = app_data_dir(&app)?;
    tokio::task::spawn_blocking(move || crate::service::picked_file::stage_picked(&app, &base, &uri, &extensions))
        .await
        .map_err(|e| crate::error::AppError::internal(e.to_string()))?
}

/// 저장 위치가 content:// 주소일 때 먼저 쓸 임시 경로
#[tauri::command]
#[specta::specta]
pub async fn prepare_save_target(app: AppHandle, file_name: String) -> Result<String> {
    crate::service::picked_file::save_target(&app_data_dir(&app)?, &file_name)
}

/// 임시 경로에 쓴 파일을 고른 content:// 주소로 옮긴다
#[tauri::command]
#[specta::specta]
pub async fn deliver_saved_file(app: AppHandle, tmp: String, uri: String) -> Result<()> {
    let base = app_data_dir(&app)?;
    tokio::task::spawn_blocking(move || crate::service::picked_file::deliver_saved(&app, &base, &tmp, &uri))
        .await
        .map_err(|e| crate::error::AppError::internal(e.to_string()))?
}

/// 새 버전 확인 (GitHub Releases). 내려받지 않고 릴리스 페이지 주소만 돌려준다 (1-E-2)
#[tauri::command]
#[specta::specta]
pub async fn check_for_update(state: State<'_, AppState>) -> Result<crate::service::update::UpdateInfo> {
    crate::service::update::check(&state.http_client).await
}

/// 지난 panic 기록 (최근 것부터, 앱 데이터 폴더의 logs/panic.log)
#[tauri::command]
#[specta::specta]
pub async fn get_panic_log(app: AppHandle) -> Result<Vec<crate::sys::panic_log::PanicEntry>> {
    Ok(crate::sys::panic_log::read(&crate::sys::panic_log::log_path(&app_data_dir(&app)?), 100))
}

#[tauri::command]
#[specta::specta]
pub async fn clear_panic_log(app: AppHandle) -> Result<()> {
    Ok(crate::sys::panic_log::clear(&crate::sys::panic_log::log_path(&app_data_dir(&app)?))?)
}

/// DB 점검: 무결성(PRAGMA integrity_check)과 아무 데도 쓰이지 않는 파일 목록. 지우지 않는다
#[tauri::command]
#[specta::specta]
pub async fn check_data(app: AppHandle, state: State<'_, AppState>) -> Result<crate::service::data::check::DataCheck> {
    crate::service::data::check::check_data(&state.db, &app_data_dir(&app)?).await
}

/// 여러 대화를 한 번에 휴지통으로 (대화마다 휴지통 항목 하나). 옮긴 개수를 돌려준다.
#[tauri::command]
#[specta::specta]
pub async fn delete_conversations(app: AppHandle, state: State<'_, AppState>, ids: Vec<String>) -> Result<u64> {
    let mut n = 0;
    for id in &ids {
        match move_to_trash(&app, &state, TrashKind::Conversation, id).await {
            Ok(_) => n += 1,
            Err(crate::error::AppError::NotFound { .. }) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

// ---------------------------------------------------------------------------
// 휴지통 (2-A-3)
// ---------------------------------------------------------------------------

fn trash_retention(app: &AppHandle) -> Option<u32> {
    use tauri::Manager;
    app.state::<crate::sys::settings::SettingsState<crate::models::settings::AppSetting>>()
        .0
        .lock()
        .ok()
        .and_then(|s| s.trash_retention_days)
}

/// 지우기 커맨드들이 부른다: 휴지통으로 옮기고, 보존 기간이 지난 것을 정리한다
pub async fn move_to_trash(app: &AppHandle, state: &AppState, kind: TrashKind, id: &str) -> Result<TrashItem> {
    let base = app_data_dir(app)?;
    let item = trash::move_to_trash(&state.db, &base, kind, id).await?;
    // 지운 대화의 메모리 색인 행도 지운다 (벡터 캐시는 남겨 되살리면 다시 임베딩하지 않게)
    if kind == TrashKind::Conversation {
        crate::service::memory::maintain::after_conversation_deleted(app, id);
    }
    match trash::purge_expired(&state.db, &base, trash_retention(app)).await {
        Ok(n) if n > 0 => crate::service::memory::maintain::after_permanent_delete(app, &state.db),
        Ok(_) => {}
        Err(e) => eprintln!("[trash] 기간 지난 항목 정리 실패: {e}"),
    }
    Ok(item)
}

/// 휴지통 목록 (열 때 보존 기간이 지난 것을 먼저 정리)
#[tauri::command]
#[specta::specta]
pub async fn list_trash(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<TrashItem>> {
    let base = app_data_dir(&app)?;
    if trash::purge_expired(&state.db, &base, trash_retention(&app)).await? > 0 {
        crate::service::memory::maintain::after_permanent_delete(&app, &state.db);
    }
    trash::list(&state.db).await
}

/// 되살리기
#[tauri::command]
#[specta::specta]
pub async fn restore_trash_item(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<TrashItem> {
    trash::restore(&state.db, &app_data_dir(&app)?, &id).await
}

/// 완전히 지우기 (되돌릴 수 없음 — 화면에서 확인을 받는다)
#[tauri::command]
#[specta::specta]
pub async fn purge_trash_item(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<bool> {
    let removed = trash::purge(&state.db, &app_data_dir(&app)?, &id).await?;
    if removed {
        crate::service::memory::maintain::after_permanent_delete(&app, &state.db);
    }
    Ok(removed)
}

/// 휴지통 비우기 (되돌릴 수 없음 — 화면에서 확인을 받는다). 지운 개수
#[tauri::command]
#[specta::specta]
pub async fn empty_trash(app: AppHandle, state: State<'_, AppState>) -> Result<u32> {
    let n = trash::purge_all(&state.db, &app_data_dir(&app)?).await?;
    if n > 0 {
        crate::service::memory::maintain::after_permanent_delete(&app, &state.db);
    }
    Ok(n)
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
        let ext = std::path::Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
            .to_ascii_lowercase();
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
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_string();
    if cleaned.is_empty() {
        "file".into()
    } else {
        cleaned
    }
}

/// 임시 드롭 폴더에서 하루 지난 것 지우기
fn clean_old(root: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    let day = std::time::Duration::from_secs(24 * 60 * 60);
    for e in entries.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|d| d > day);
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

/// 화면에서 만든 png(이미지 카드 등)를 사용자가 저장 대화상자로 고른 곳에 쓴다.
/// .png 경로와 png 내용만 받는다 (다른 파일을 쓰는 데 쓰이지 않게)
#[tauri::command]
#[specta::specta]
pub fn save_png_file(path: String, data_base64: String) -> Result<()> {
    use base64::Engine;
    let path = PathBuf::from(path);
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
        return Err(crate::error::AppError::validation("png 파일로만 저장할 수 있습니다", None));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.as_bytes())
        .map_err(|e| crate::error::AppError::validation(format!("이미지 내용이 올바르지 않습니다: {e}"), None))?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(crate::error::AppError::validation("png 이미지가 아닙니다", None));
    }
    std::fs::write(&path, bytes)?;
    Ok(())
}
