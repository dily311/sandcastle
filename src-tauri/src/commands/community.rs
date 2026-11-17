//! 커뮤니티 플러그인 커맨드 (CLAUDE.md 6-D-1, ADR-012). 코어 플러그인 `community`(기본 꺼짐)가 켜져 있을 때만.
//! 파일 고르기는 화면이 한다(Android의 content 주소는 stage_picked_file로 먼저 바꿔서 넘김).
use std::path::Path;

use tauri::{AppHandle, State};

use crate::{
    commands::{character_image::app_data_dir, data::move_to_trash},
    error::Result,
    service::{
        community::{self, PluginPreview, UserPlugin},
        data::trash::{TrashItem, TrashKind},
    },
    state::AppState,
    sys::plugins::{id, require},
};

/// 설치된 플러그인 목록
#[tauri::command]
#[specta::specta]
pub async fn list_user_plugins(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<UserPlugin>> {
    require(&app, id::COMMUNITY)?;
    community::list(&state.db, &app_data_dir(&app)?).await
}

/// zip 파일이나 폴더를 읽어 미리보기 (아무것도 쓰지 않음)
#[tauri::command]
#[specta::specta]
pub async fn preview_user_plugin(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<PluginPreview> {
    require(&app, id::COMMUNITY)?;
    community::preview(&state.db, &app_data_dir(&app)?, Path::new(&path)).await
}

/// 미리보기에서 확인한 것을 설치 (hash가 다르면 거절). 새로 설치한 플러그인은 꺼져 있다
#[tauri::command]
#[specta::specta]
pub async fn install_user_plugin(app: AppHandle, state: State<'_, AppState>, path: String, hash: String) -> Result<UserPlugin> {
    require(&app, id::COMMUNITY)?;
    community::install(&state.db, &app_data_dir(&app)?, Path::new(&path), &hash).await
}

/// 켜기(권한 전부에 동의한 목록과 함께) · 끄기
#[tauri::command]
#[specta::specta]
pub async fn set_user_plugin_enabled(app: AppHandle, state: State<'_, AppState>, plugin_id: String, enabled: bool, grant: Vec<String>) -> Result<UserPlugin> {
    require(&app, id::COMMUNITY)?;
    community::set_enabled(&state.db, &app_data_dir(&app)?, &plugin_id, enabled, grant).await
}

/// 삭제 = 휴지통으로 (되살리면 꺼진 채)
#[tauri::command]
#[specta::specta]
pub async fn uninstall_user_plugin(app: AppHandle, state: State<'_, AppState>, plugin_id: String) -> Result<TrashItem> {
    require(&app, id::COMMUNITY)?;
    community::prepare_uninstall(&state.db, &app_data_dir(&app)?, &plugin_id).await?;
    move_to_trash(&app, &state, TrashKind::UserPlugin, &plugin_id).await
}
