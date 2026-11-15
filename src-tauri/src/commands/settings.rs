use crate::{
    error::{AppError, ErrorCode, Result}, models::settings::{
        AppSetting, CssTheme, CssThemeMeta, FeatureSetting, SettingsDomain, ThemeSetting,
    }, sys::{
        settings::{SettingsState, apply_patch, save}, themes::ThemeManager,
    },
};
use serde_json::json;
use tauri::{AppHandle, Emitter, State};

// =======================================
// ---- AppSetting ----
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_app_setting(state: State<'_, SettingsState<AppSetting>>) -> Result<AppSetting> {
    Ok(state.0.lock().map_err(|_| AppError::archive(ErrorCode::InternalError, "archive lock poisoned"))?.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn update_app_setting(
    app: AppHandle,
    state: State<'_, SettingsState<AppSetting>>,
    patch: serde_json::Value,
) -> Result<AppSetting> {
    let settings = apply_patch(&app, &state.0, patch)?;
    // 이벤트 호출
    app.emit(AppSetting::EVENT_NAME, &settings)
        .map_err(|e| AppError::io(e.to_string()))?;
    Ok(settings)
}

// =======================================
// ---- ThemeSetting ----
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_theme_setting(
    state: State<'_, SettingsState<ThemeSetting>>,
) -> Result<ThemeSetting> {
    Ok(state.0.lock().map_err(|_| AppError::archive(ErrorCode::InternalError, "archive lock poisoned"))?.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn update_theme_setting(
    app: AppHandle,
    state: State<'_, SettingsState<ThemeSetting>>,
    patch: serde_json::Value,
) -> Result<ThemeSetting> {
    let settings = apply_patch(&app, &state.0, patch)?;
    // 이벤트 호출
    app.emit(ThemeSetting::EVENT_NAME, &settings)
        .map_err(|e| AppError::io(e.to_string()))?;
    Ok(settings)
}

#[tauri::command]
#[specta::specta]
pub fn get_theme_list(theme_mgr: State<'_, ThemeManager>) -> Result<Vec<CssThemeMeta>> {
    theme_mgr
        .get_list()
        .map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn load_theme_content(theme_mgr: State<'_, ThemeManager>, filename: String) -> Result<String> {
    theme_mgr
        .load_content(&filename)
        .map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn load_theme(theme_mgr: State<'_, ThemeManager>, filename: String) -> Result<CssTheme> {
    theme_mgr
        .load(&filename)
        .map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn save_theme(
    theme_mgr: State<'_, ThemeManager>,
    filename: String,
    content: CssTheme,
) -> Result<()> {
    theme_mgr
        .save(&filename, &content)
        .map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn delete_theme(theme_mgr: State<'_, ThemeManager>, filename: String) -> Result<()> {
    theme_mgr
        .delete(&filename)
        .map_err(|e| AppError::io(e.to_string()))
}

// =======================================
// ---- FeatureSetting ----
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_feature_setting(state: State<'_, SettingsState<FeatureSetting>>) -> Result<FeatureSetting> {
    Ok(state.0.lock().map_err(|_| AppError::archive(ErrorCode::InternalError, "archive lock poisoned"))?.clone())
}

#[tauri::command]
#[specta::specta]
pub async fn update_feature_setting(
    app: AppHandle,
    state: State<'_, SettingsState<FeatureSetting>>,
    id: String,
    enabled: bool,
) -> Result<()> {
    let mut settings = state.0.lock().map_err(|e| AppError::io(e.to_string()))?;
    settings.feature.insert(id.clone(), enabled);

    save(&app, &*settings)?;

    if id == crate::sys::plugins::id::REGEX {
        crate::service::chat::regex_processor::set_enabled(enabled);
    }

    // 모든 창에 알린다: 메인 창은 코어 플러그인(메뉴·화면), 위젯 창은 위젯 하위 기능
    let json = json!({"id": id, "enabled": enabled});
    app.emit(FeatureSetting::EVENT_NAME, json).map_err(|e| AppError::io(e.to_string()))?;

    Ok(())
}
