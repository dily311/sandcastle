use crate::{
    db, error::AppError, models::widget::{MusicMetaInput, Playlist, PlaylistInput, PlaylistItem, WidgetMusicSetting}, state::AppState, sys::{library::LibraryRoot, music::{self}, settings::merge_json}, widget::{InteractiveRegion, MouseTrackerState, WidgetInteractionState, manager::WidgetManager},
};
use std::path::Path;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_autostart::AutoLaunchManager;
use tauri_plugin_store::StoreExt;

// =======================================
// 위젯 윈도우 열기
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn open_widget(app: AppHandle, state: State<'_, MouseTrackerState>) -> Result<(), AppError> {
    // 하위 기능이 모두 꺼져 있으면 빈 창을 띄우지 않는다
    let wanted = app
        .try_state::<crate::sys::settings::SettingsState<crate::models::settings::FeatureSetting>>()
        .and_then(|s| s.0.lock().ok().map(|f| f.widget_window_wanted()))
        .unwrap_or(true);
    if wanted {
        WidgetManager::open_widget(&app, &state)?;
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn close_widget(app: AppHandle, state: State<'_, MouseTrackerState>) -> Result<(), AppError> {
    WidgetManager::close_widget(&app, &state)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_widget(app: AppHandle, state: State<'_, MouseTrackerState>) -> Result<(), AppError> {
    WidgetManager::toggle_widget(&app, &state)?;
    Ok(())
}

// ignore 값을 받아서 해당 앱 윈도우를 무시하게 하거나 클릭될 수 있게 하는 함수
#[tauri::command]
#[specta::specta]
pub async fn toggle_ignore_cursor(window: tauri::Window, ignore: bool) -> Result<(), AppError> {
    window.set_ignore_cursor_events(ignore)?;
    Ok(())
}

// =======================================
// 자동시작 설정
// =======================================

#[tauri::command]
#[specta::specta]
pub async fn is_autostart_enabled(state: State<'_, AutoLaunchManager>) -> Result<bool, AppError> {
    state
        .is_enabled()
        .map_err(|e| AppError::window(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn set_autostart(state: State<'_, AutoLaunchManager>, enabled: bool) -> Result<(), AppError> {
    if enabled {
        state.enable().map_err(|e| AppError::io(e.to_string()))?;
    } else {
        state.disable().map_err(|e| AppError::io(e.to_string()))?;
    }

    Ok(())
}

// =======================================
// 클릭스루
// =======================================
#[tauri::command]
#[specta::specta]
pub fn update_interactive_regions(
    upserts: Vec<InteractiveRegion>,
    removed: Vec<String>,
    state: State<'_, WidgetInteractionState>,
) -> Result<(), AppError> {
    let mut regions = state.regions.lock().unwrap();

    for region in upserts {
        regions.insert(region.id.clone(), region);
    }
    for id in removed {
        regions.remove(&id);
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn remove_interactive_regions(
    ids: Vec<String>,
    state: State<WidgetInteractionState>,
) -> Result<(), AppError> {
    let mut regions = state.regions.lock().unwrap();

    for id in ids {
        regions.remove(&id);
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_widget_click_through(window: WebviewWindow, enabled: bool) -> Result<(), AppError> {
    window.set_ignore_cursor_events(enabled)?;
    Ok(())
}

// =======================================
// 음악 플레이어
// =======================================
#[tauri::command]
#[specta::specta]
pub fn get_widget_music_settings(app: AppHandle) -> Result<WidgetMusicSetting, AppError> {
    let store = app.store("widget.json").map_err(|e| AppError::io(e.to_string()))?;
    let result: WidgetMusicSetting = store.get("music")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn save_widget_music_settings(app: AppHandle, patch: serde_json::Value) -> Result<(), AppError> {
    let store = app.store("widget.json").map_err(|e| AppError::io(e.to_string()))?;
    let setting: WidgetMusicSetting = store.get("music").and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    let mut data: serde_json::Value = serde_json::to_value(&setting)?;

    // 이전 데이터와 통합
    merge_json(&mut data, &patch);
    
    store.set("music", serde_json::to_value(&data).unwrap());
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_select_folder_music_data(app: AppHandle, state: State<'_, AppState>, folder_path: String) -> Result<LibraryRoot, AppError> {
    let cache_dir = app.path().app_cache_dir()?;
    // 에셋 프로토콜 범위는 앱 폴더로 좁혀 두었으므로(tauri.conf.json), 사용자가 고른 음악 폴더만 실행 중에 연다
    app.asset_protocol_scope()
        .allow_directory(&folder_path, true)
        .map_err(|e| AppError::io(format!("음악 폴더를 열 수 없습니다: {e}")))?;
    let result = music::folder_indexing(&state.db, folder_path.clone(), &cache_dir).await?;
    Ok(result)
}

// ---------------------------------------------------------
//  플레이 리스트
// ---------------------------------------------------------
#[tauri::command]
#[specta::specta]
pub async fn get_all_playlist(state: State<'_, AppState>) -> Result<Vec<Playlist>, AppError> {
    let result = db::widget::get_all_playlist(&state.db).await?;
    Ok(result)
}
#[tauri::command]
#[specta::specta]
pub async fn create_playlist(state: State<'_, AppState>, input: PlaylistInput) -> Result<Playlist, AppError> {
    let result = db::widget::create_playlist(&state.db, input).await?;
    Ok(result)
}
#[tauri::command]
#[specta::specta]
pub async fn update_playlist(state: State<'_, AppState>, id: String, input: PlaylistInput) -> Result<(), AppError> {
    db::widget::update_playlist(&state.db, &id, input).await?;
    Ok(())
}
#[tauri::command]
#[specta::specta]
pub async fn delete_playlist(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    db::widget::delete_playlist(&state.db, &id).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_playlist_entry(state: State<'_, AppState>, playlist_id: String) -> Result<Vec<PlaylistItem>, AppError> {
    let result = db::widget::get_all_playlist_entry(&state.db, &playlist_id).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn append_playlist_entry(state: State<'_, AppState>, playlist_id: String, music_src: String, sort_order: Option<u32>) -> Result<(), AppError> {
    db::widget::append_playlist_entry(&state.db, &playlist_id, &music_src, sort_order).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn remove_playlist_entry(state: State<'_, AppState>, playlist_id: String, music_src: String) -> Result<(), AppError> {
    db::widget::remove_playlist_entry(&state.db, &playlist_id, &music_src).await?;
    Ok(())
}

// ---------------------------------------------------------
//  음악 데이터 업데이트
// ---------------------------------------------------------
#[tauri::command]
#[specta::specta]
pub async fn update_music_meta(state: State<'_, AppState>, input: MusicMetaInput) -> Result<(), AppError> {
    db::widget::upsert_music_meta(&state.db, input).await?;
    Ok(())
}
#[tauri::command]
#[specta::specta]
pub async fn update_playlist_music_order(state: State<'_, AppState>, playlist_id: String, input: Vec<PlaylistItem>) -> Result<(), AppError> {
    for item in input {
        db::widget::update_playlist_music_order(&state.db, &playlist_id, &item.meta.path, item.sort_order).await?;
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_music_album_art_pos(state: State<'_, AppState>, path: String, pos: serde_json::Value) -> Result<(), AppError> {
    db::widget::upsert_music_album_art_pos(&state.db, &path, pos).await?;
    Ok(())
}

// ---------------------------------------------------------
//  파일 접근 또는 변경
// ---------------------------------------------------------
#[tauri::command]
#[specta::specta]
pub async fn set_album_art(app: AppHandle, file_path: String, img_path: String) -> Result<String, AppError> {
    let cache_dir = app.path().app_cache_dir()?;
    music::set_album_art(file_path, img_path, &cache_dir).await
}

#[tauri::command]
#[specta::specta]
pub async fn filter_existing_paths(paths: Vec<String>) -> Result<Vec<String>, AppError> {
    let existing = paths
        .into_iter()
        .filter(|p| Path::new(p).is_file()) // exists() 대신 is_file()로 폴더는 제외
        .collect();
    Ok(existing)
}
