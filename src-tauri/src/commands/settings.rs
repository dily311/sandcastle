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
    // 위젯 창은 여기서 직접 열고 닫는다 (어느 화면에서 바꿔도 창 상태가 설정과 같게)
    #[cfg(desktop)]
    if id == crate::sys::plugins::id::WIDGET {
        use tauri::Manager;
        if let Some(tracker) = app.try_state::<crate::widget::MouseTrackerState>() {
            let result = if enabled {
                crate::widget::manager::WidgetManager::open_widget(&app, &tracker).map(|_| ())
            } else {
                crate::widget::manager::WidgetManager::close_widget(&app, &tracker)
            };
            if let Err(e) = result {
                eprintln!("[widget] 창 {} 실패: {e}", if enabled { "열기" } else { "닫기" });
            }
        }
    }

    // 모든 창에 알린다: 메인 창은 코어 플러그인(메뉴·화면), 위젯 창은 위젯 하위 기능
    let json = json!({"id": id, "enabled": enabled});
    app.emit(FeatureSetting::EVENT_NAME, json).map_err(|e| AppError::io(e.to_string()))?;

    Ok(())
}

/// 배경 이미지 정하기: 고른 파일을 앱 데이터 backgrounds/에 복사해 쓴다 (원본은 건드리지 않음).
/// path가 None이면 배경을 없앤다. 전에 복사해 둔 배경 파일은 지운다. 바뀐 설정을 돌려준다
#[tauri::command]
#[specta::specta]
pub async fn set_theme_background(app: AppHandle, state: State<'_, SettingsState<ThemeSetting>>, path: Option<String>) -> Result<ThemeSetting> {
    let dir = crate::commands::character_image::app_data_dir(&app)?.join("backgrounds");
    let old = state.0.lock().map_err(|_| AppError::internal("settings lock poisoned"))?.background_image.clone();
    let new_path = match path {
        Some(src) => {
            let src = std::path::PathBuf::from(src);
            let ext = src.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
            if !["png", "jpg", "jpeg", "webp", "gif", "avif"].contains(&ext.as_str()) {
                return Err(AppError::validation("이미지 파일(png·jpg·webp·gif)만 배경으로 쓸 수 있습니다", None));
            }
            std::fs::create_dir_all(&dir)?;
            let dest = dir.join(format!("{}.{ext}", uuid::Uuid::new_v4()));
            std::fs::copy(&src, &dest)?;
            Some(dest.to_string_lossy().into_owned())
        }
        None => None,
    };
    let settings = apply_patch(&app, &state.0, json!({ "backgroundImage": new_path }))?;
    // 앱이 복사해 둔 예전 배경만 지운다 (backgrounds/ 안의 파일)
    if let Some(old) = old.filter(|o| Some(o) != new_path.as_ref()) {
        let old = std::path::PathBuf::from(old);
        if old.parent() == Some(dir.as_path()) {
            let _ = std::fs::remove_file(old);
        }
    }
    app.emit(ThemeSetting::EVENT_NAME, &settings).map_err(|e| AppError::io(e.to_string()))?;
    Ok(settings)
}
