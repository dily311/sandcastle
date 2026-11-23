use crate::{
    error::{AppError, ErrorCode, Result},
    models::settings::{AppSetting, CssTheme, CssThemeMeta, FeatureSetting, SettingsDomain, ThemeSetting},
    sys::{
        settings::{apply_patch, save, SettingsState},
        themes::ThemeManager,
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
    Ok(state
        .0
        .lock()
        .map_err(|_| AppError::archive(ErrorCode::InternalError, "archive lock poisoned"))?
        .clone())
}

#[tauri::command]
#[specta::specta]
pub async fn update_app_setting(app: AppHandle, state: State<'_, SettingsState<AppSetting>>, patch: serde_json::Value) -> Result<AppSetting> {
    let settings = apply_patch(&app, &state.0, patch)?;
    crate::service::ai::call_log::set_enabled(settings.ai_call_log);
    // 이벤트 호출
    app.emit(AppSetting::EVENT_NAME, &settings).map_err(|e| AppError::io(e.to_string()))?;
    Ok(settings)
}

/// 타이핑 표시 그림 (5-E-17-4): 고른 그림·GIF를 앱 데이터 `themes/typing/`에 복사해 쓰고, 전에 쓰던 복사본은 지운다.
/// `None`이면 그림을 뺀다 (원본 파일은 건드리지 않음). 테마 폴더라 백업·동기화에 같이 간다
#[tauri::command]
#[specta::specta]
pub async fn set_typing_image(app: AppHandle, state: State<'_, SettingsState<AppSetting>>, path: Option<String>) -> Result<AppSetting> {
    let dir = crate::commands::character_image::app_data_dir(&app)?.join("themes").join("typing");
    let cur = state.0.lock().map_err(|_| AppError::internal("settings lock poisoned"))?.typing.clone();
    let image = match path {
        Some(src) => Some(copy_typing_image(&dir, std::path::Path::new(&src))?),
        None => None,
    };
    let mut typing = serde_json::to_value(&cur)?;
    typing["image"] = json!(image);
    let settings = apply_patch(&app, &state.0, json!({ "typing": typing }))?;
    // 새 설정을 저장한 뒤에 옛 복사본을 지운다 (우리 폴더 안의 것만)
    if let Some(old) = cur.image.filter(|o| Some(o) != image.as_ref()) {
        let old = std::path::PathBuf::from(old);
        if old.parent() == Some(dir.as_path()) {
            let _ = std::fs::remove_file(old);
        }
    }
    app.emit(AppSetting::EVENT_NAME, &settings).map_err(|e| AppError::io(e.to_string()))?;
    Ok(settings)
}

/// 타이핑 표시 그림 최대 크기 (GIF 움직임을 담을 만큼)
const TYPING_IMAGE_MAX: u64 = 8 * 1024 * 1024;

fn copy_typing_image(dir: &std::path::Path, src: &std::path::Path) -> Result<String> {
    let ext = src.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    if !["png", "jpg", "jpeg", "webp", "gif", "avif"].contains(&ext.as_str()) {
        return Err(AppError::validation("그림 파일(png·jpg·webp·gif)만 쓸 수 있습니다", None));
    }
    if std::fs::metadata(src)?.len() > TYPING_IMAGE_MAX {
        return Err(AppError::validation("그림이 너무 큽니다 (8MB까지)", None));
    }
    std::fs::create_dir_all(dir)?;
    let dest = dir.join(format!("{}.{ext}", uuid::Uuid::new_v4()));
    std::fs::copy(src, &dest)?;
    Ok(dest.to_string_lossy().into_owned())
}

#[cfg(test)]
mod typing_image_tests {
    use super::copy_typing_image;

    #[test]
    fn copies_only_images_under_the_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("themes").join("typing");
        let gif = tmp.path().join("a.GIF");
        std::fs::write(&gif, b"GIF89a").unwrap();
        let out = copy_typing_image(&dir, &gif).unwrap();
        assert!(out.ends_with(".gif") && std::path::Path::new(&out).parent() == Some(dir.as_path()));
        assert_eq!(std::fs::read(&out).unwrap(), b"GIF89a");

        let txt = tmp.path().join("a.txt");
        std::fs::write(&txt, b"x").unwrap();
        assert!(copy_typing_image(&dir, &txt).is_err());

        let big = tmp.path().join("big.png");
        std::fs::File::create(&big).unwrap().set_len(super::TYPING_IMAGE_MAX + 1).unwrap();
        assert!(copy_typing_image(&dir, &big).is_err());
    }
}

// =======================================
// ---- ThemeSetting ----
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_theme_setting(state: State<'_, SettingsState<ThemeSetting>>) -> Result<ThemeSetting> {
    Ok(state
        .0
        .lock()
        .map_err(|_| AppError::archive(ErrorCode::InternalError, "archive lock poisoned"))?
        .clone())
}

#[tauri::command]
#[specta::specta]
pub async fn update_theme_setting(app: AppHandle, state: State<'_, SettingsState<ThemeSetting>>, patch: serde_json::Value) -> Result<ThemeSetting> {
    let settings = apply_patch(&app, &state.0, patch)?;
    // 이벤트 호출
    app.emit(ThemeSetting::EVENT_NAME, &settings).map_err(|e| AppError::io(e.to_string()))?;
    Ok(settings)
}

#[tauri::command]
#[specta::specta]
pub fn get_theme_list(theme_mgr: State<'_, ThemeManager>) -> Result<Vec<CssThemeMeta>> {
    theme_mgr.get_list().map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn load_theme_content(theme_mgr: State<'_, ThemeManager>, filename: String) -> Result<String> {
    theme_mgr.load_content(&filename).map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn load_theme(theme_mgr: State<'_, ThemeManager>, filename: String) -> Result<CssTheme> {
    theme_mgr.load(&filename).map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn save_theme(theme_mgr: State<'_, ThemeManager>, filename: String, content: CssTheme) -> Result<()> {
    theme_mgr.save(&filename, &content).map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn delete_theme(theme_mgr: State<'_, ThemeManager>, filename: String) -> Result<()> {
    theme_mgr.delete(&filename).map_err(|e| AppError::io(e.to_string()))
}

// =======================================
// ---- FeatureSetting ----
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_feature_setting(state: State<'_, SettingsState<FeatureSetting>>) -> Result<FeatureSetting> {
    Ok(state
        .0
        .lock()
        .map_err(|_| AppError::archive(ErrorCode::InternalError, "archive lock poisoned"))?
        .clone())
}

#[tauri::command]
#[specta::specta]
pub async fn update_feature_setting(app: AppHandle, state: State<'_, SettingsState<FeatureSetting>>, id: String, enabled: bool) -> Result<()> {
    let wanted = {
        let mut settings = state.0.lock().map_err(|e| AppError::io(e.to_string()))?;
        settings.feature.insert(id.clone(), enabled);
        save(&app, &*settings)?;
        settings.widget_window_wanted()
    };

    if id == crate::sys::plugins::id::REGEX {
        crate::service::chat::regex_processor::set_enabled(enabled);
    }
    // 모든 창에 먼저 알린다 (메인 창은 코어 플러그인, 위젯 창은 하위 기능). 닫을 창에 닫은 뒤 보내면
    // "잘못된 창 핸들" 오류가 나므로 알림 → 창 열고 닫기 순서
    let json = json!({"id": id, "enabled": enabled});
    app.emit(FeatureSetting::EVENT_NAME, json).map_err(|e| AppError::io(e.to_string()))?;

    // 위젯 창은 백엔드만 열고 닫는다: 위젯이 켜져 있고 하위 기능이 하나라도 켜져 있으면 열고, 아니면 닫는다
    #[cfg(desktop)]
    if id == crate::sys::plugins::id::WIDGET || id.starts_with("widget.") {
        use tauri::Manager;
        if let Some(tracker) = app.try_state::<crate::widget::MouseTrackerState>() {
            let result = if wanted {
                crate::widget::manager::WidgetManager::open_widget(&app, &tracker).map(|_| ())
            } else {
                crate::widget::manager::WidgetManager::close_widget(&app, &tracker)
            };
            if let Err(e) = result {
                eprintln!("[widget] 창 {} 실패: {e}", if wanted { "열기" } else { "닫기" });
            }
        }
    }
    #[cfg(not(desktop))]
    let _ = wanted;

    Ok(())
}

/// 배경 목록에 이미지 추가: 고른 파일들을 앱 데이터 backgrounds/에 복사한다 (원본은 건드리지 않음).
/// select면 마지막 것을 앱 배경으로 고른다. 바뀐 설정을 돌려준다
#[tauri::command]
#[specta::specta]
pub async fn add_backgrounds(app: AppHandle, state: State<'_, SettingsState<ThemeSetting>>, paths: Vec<String>, select: bool) -> Result<ThemeSetting> {
    let dir = crate::commands::character_image::app_data_dir(&app)?.join("backgrounds");
    std::fs::create_dir_all(&dir)?;
    let mut added = Vec::new();
    for src in paths.iter().take(50) {
        let src = std::path::PathBuf::from(src);
        let ext = src.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
        if !["png", "jpg", "jpeg", "webp", "gif", "avif"].contains(&ext.as_str()) {
            return Err(AppError::validation("이미지 파일(png·jpg·webp·gif)만 배경으로 쓸 수 있습니다", None));
        }
        let dest = dir.join(format!("{}.{ext}", uuid::Uuid::new_v4()));
        std::fs::copy(&src, &dest)?;
        added.push(dest.to_string_lossy().into_owned());
    }
    let mut list = state.0.lock().map_err(|_| AppError::internal("settings lock poisoned"))?.backgrounds.clone();
    list.extend(added.iter().cloned());
    let mut patch = json!({ "backgrounds": list });
    if select {
        if let Some(last) = added.last() {
            patch["backgroundImage"] = json!(last);
        }
    }
    let settings = apply_patch(&app, &state.0, patch)?;
    app.emit(ThemeSetting::EVENT_NAME, &settings).map_err(|e| AppError::io(e.to_string()))?;
    Ok(settings)
}

/// 배경 목록에서 빼고 복사해 둔 파일을 지운다 (앱 배경이었으면 배경 없음으로)
#[tauri::command]
#[specta::specta]
pub async fn remove_background(app: AppHandle, state: State<'_, SettingsState<ThemeSetting>>, path: String) -> Result<ThemeSetting> {
    let dir = crate::commands::character_image::app_data_dir(&app)?.join("backgrounds");
    let (list, selected) = {
        let cur = state.0.lock().map_err(|_| AppError::internal("settings lock poisoned"))?;
        (
            cur.backgrounds.iter().filter(|p| **p != path).cloned().collect::<Vec<_>>(),
            cur.background_image.clone(),
        )
    };
    let mut patch = json!({ "backgrounds": list });
    if selected.as_deref() == Some(path.as_str()) {
        patch["backgroundImage"] = serde_json::Value::Null;
    }
    let settings = apply_patch(&app, &state.0, patch)?;
    let p = std::path::PathBuf::from(&path);
    // 앱이 복사해 둔 파일(backgrounds/ 안)만 지운다
    if p.parent() == Some(dir.as_path()) {
        let _ = std::fs::remove_file(p);
    }
    app.emit(ThemeSetting::EVENT_NAME, &settings).map_err(|e| AppError::io(e.to_string()))?;
    Ok(settings)
}

/// 글꼴 올리기: 고른 파일들(ttf·otf·woff·woff2)을 앱 데이터 fonts/에 복사한다 (원본은 건드리지 않음).
/// 확장자와 파일 머리가 글꼴이 아니면 하나도 복사하지 않고 거절한다. select면 마지막 것을 앱 글꼴로 고른다
#[tauri::command]
#[specta::specta]
pub async fn add_fonts(app: AppHandle, state: State<'_, SettingsState<ThemeSetting>>, paths: Vec<String>, select: bool) -> Result<ThemeSetting> {
    let dir = crate::commands::character_image::app_data_dir(&app)?.join(crate::service::data::fonts::FONTS_DIR);
    let srcs: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
    let added = crate::service::data::fonts::import_fonts(&dir, &srcs)?;
    let mut list = state.0.lock().map_err(|_| AppError::internal("settings lock poisoned"))?.fonts.clone();
    list.extend(added.iter().cloned());
    let mut patch = json!({ "fonts": list });
    if select {
        if let Some(last) = added.last() {
            patch["appFont"] = json!(last.path);
        }
    }
    let settings = apply_patch(&app, &state.0, patch)?;
    app.emit(ThemeSetting::EVENT_NAME, &settings).map_err(|e| AppError::io(e.to_string()))?;
    Ok(settings)
}

/// 글꼴 목록에서 빼고 복사해 둔 파일을 지운다 (앱 글꼴이었으면 기본 글꼴로)
#[tauri::command]
#[specta::specta]
pub async fn remove_font(app: AppHandle, state: State<'_, SettingsState<ThemeSetting>>, path: String) -> Result<ThemeSetting> {
    let dir = crate::commands::character_image::app_data_dir(&app)?.join(crate::service::data::fonts::FONTS_DIR);
    let (list, selected) = {
        let cur = state.0.lock().map_err(|_| AppError::internal("settings lock poisoned"))?;
        (cur.fonts.iter().filter(|f| f.path != path).cloned().collect::<Vec<_>>(), cur.app_font.clone())
    };
    let mut patch = json!({ "fonts": list });
    if selected.as_deref() == Some(path.as_str()) {
        patch["appFont"] = serde_json::Value::Null;
    }
    let settings = apply_patch(&app, &state.0, patch)?;
    crate::service::data::fonts::remove_font_file(&dir, &path);
    app.emit(ThemeSetting::EVENT_NAME, &settings).map_err(|e| AppError::io(e.to_string()))?;
    Ok(settings)
}
