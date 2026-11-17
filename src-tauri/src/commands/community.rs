//! 커뮤니티 플러그인 커맨드 (CLAUDE.md 6-D-1, ADR-012). 코어 플러그인 `community`(기본 꺼짐)가 켜져 있을 때만.
//! 파일 고르기는 화면이 한다(Android의 content 주소는 stage_picked_file로 먼저 바꿔서 넘김).
use std::path::Path;

use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use crate::{
    commands::{character_image::app_data_dir, data::move_to_trash},
    error::{AppError, Result},
    service::{
        community::{
            self,
            runtime::{self, PluginLogEntry, RuntimeStart},
            PluginPreview, UserPlugin,
        },
        data::trash::{TrashItem, TrashKind},
    },
    state::AppState,
    sys::plugins::{id, require},
};

/// 설치·켜기·지우기 뒤 — 화면의 플러그인 호스트가 다시 시작한다
pub const CHANGED_EVENT: &str = "user-plugins-changed";
/// 오류가 많아 멈춤 — 호스트가 그 iframe을 내린다
pub const SUSPENDED_EVENT: &str = "user-plugin-suspended";

fn changed(app: &AppHandle) {
    let _ = app.emit(CHANGED_EVENT, ());
}

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
    let p = community::install(&state.db, &app_data_dir(&app)?, Path::new(&path), &hash).await?;
    changed(&app);
    Ok(p)
}

/// 켜기(권한 전부에 동의한 목록과 함께) · 끄기
#[tauri::command]
#[specta::specta]
pub async fn set_user_plugin_enabled(app: AppHandle, state: State<'_, AppState>, plugin_id: String, enabled: bool, grant: Vec<String>) -> Result<UserPlugin> {
    require(&app, id::COMMUNITY)?;
    let p = community::set_enabled(&state.db, &app_data_dir(&app)?, &plugin_id, enabled, grant).await?;
    changed(&app);
    Ok(p)
}

/// 삭제 = 휴지통으로 (되살리면 꺼진 채)
#[tauri::command]
#[specta::specta]
pub async fn uninstall_user_plugin(app: AppHandle, state: State<'_, AppState>, plugin_id: String) -> Result<TrashItem> {
    require(&app, id::COMMUNITY)?;
    community::prepare_uninstall(&state.db, &app_data_dir(&app)?, &plugin_id).await?;
    let item = move_to_trash(&app, &state, TrashKind::UserPlugin, &plugin_id).await?;
    changed(&app);
    Ok(item)
}

// ---------------------------------------------------------------------------
// 런타임 (6-D-2)
// ---------------------------------------------------------------------------

/// 화면의 플러그인 호스트가 시작할 때: 켜진 플러그인을 도는 목록에 올리고 iframe 주소를 돌려준다
#[tauri::command]
#[specta::specta]
pub async fn start_user_plugins(app: AppHandle, state: State<'_, AppState>) -> Result<RuntimeStart> {
    if require(&app, id::COMMUNITY).is_err() {
        runtime::stop_all();
        return Err(AppError::validation("커뮤니티 플러그인이 꺼져 있음", None));
    }
    let base = app_data_dir(&app)?;
    let list = community::list(&state.db, &base).await?;
    Ok(runtime::start(&list, &base))
}

/// 호스트를 내릴 때 (코어 플러그인을 끔 등)
#[tauri::command]
#[specta::specta]
pub fn stop_user_plugins() {
    runtime::stop_all();
}

/// 플러그인 iframe의 호출. 화면이 보낸 창을 확인해 플러그인 id를 붙여 넘긴다. 권한은 여기서 다시 본다
#[tauri::command]
#[specta::specta]
pub async fn call_user_plugin(app: AppHandle, state: State<'_, AppState>, plugin_id: String, method: String, args: Value) -> Result<Value> {
    require(&app, id::COMMUNITY)?;
    let r = community::call(&state.db, &plugin_id, &method, &args).await;
    if r.is_ok() && method.ends_with(".propose") {
        crate::commands::proposals::changed(&app);
    }
    if runtime::is_suspended(&plugin_id) {
        let _ = app.emit(SUSPENDED_EVENT, &plugin_id);
    }
    r
}

/// 플러그인 기록 (최근 것, 메모리 — 앱을 다시 켜면 비어 있음)
#[tauri::command]
#[specta::specta]
pub fn get_user_plugin_log(plugin_id: String) -> Vec<PluginLogEntry> {
    runtime::log_of(&plugin_id)
}
