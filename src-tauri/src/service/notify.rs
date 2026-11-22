//! 응답이 끝났는데 앱이 가려져 있으면 운영체제 알림 (roadmap 5-E-17·18, 기본 꺼짐).
//! 사용자는 알림 제목 글(`{{char}}` = 캐릭터 이름)과 아이콘 그림만 바꾼다. 본문은 "누구의 응답이 도착함" — 대화 내용은 잠금 화면에 싣지 않는다.
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::models::settings::{AppSetting, DoneNotify};
use crate::sys::settings::SettingsState;

pub const DEFAULT_TITLE: &str = "샌드캐슬";

/// 알림 제목: 비면 앱 이름, `{{char}}`는 캐릭터 이름
pub fn title_of(cfg: &DoneNotify, char_name: &str) -> String {
    let t = cfg.title.trim();
    let t = if t.is_empty() { DEFAULT_TITLE } else { t };
    t.replace("{{char}}", char_name).chars().take(80).collect()
}

/// 앱 창이 앞에 있으면 알리지 않는다 (보고 있으니까)
fn app_in_front(app: &AppHandle) -> bool {
    app.get_webview_window("main").and_then(|w| w.is_focused().ok()).unwrap_or(false)
}

/// 응답이 도착함 (켜져 있고 앱이 가려져 있을 때만). 실패해도 대화에는 영향 없음
pub fn reply_done(app: &AppHandle, char_name: &str) {
    let cfg = match app
        .try_state::<SettingsState<AppSetting>>()
        .and_then(|s| s.0.lock().ok().map(|g| g.done_notify.clone()))
    {
        Some(c) if c.enabled => c,
        _ => return,
    };
    if app_in_front(app) {
        return;
    }
    let mut b = app
        .notification()
        .builder()
        .title(title_of(&cfg, char_name))
        .body(format!("{char_name}의 응답이 도착함"));
    if let Some(icon) = cfg.icon.as_deref().filter(|p| std::path::Path::new(p).exists()) {
        b = b.icon(icon.to_string());
    }
    if let Err(e) = b.show() {
        eprintln!("[notify] 알림을 띄우지 못함: {e}");
    }
}

/// 알림을 켤 때 (Android는 사용자에게 권한을 묻는다). 허락됐으면 true
#[tauri::command]
#[specta::specta]
pub fn request_notify_permission(app: AppHandle) -> crate::error::Result<bool> {
    use tauri_plugin_notification::PermissionState;
    let n = app.notification();
    let state = match n.permission_state() {
        Ok(PermissionState::Granted) => PermissionState::Granted,
        _ => n.request_permission().map_err(|e| crate::error::AppError::internal(e.to_string()))?,
    };
    Ok(matches!(state, PermissionState::Granted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_uses_char_and_falls_back_to_app_name() {
        let mut c = DoneNotify::default();
        assert_eq!(title_of(&c, "아리아"), "샌드캐슬");
        c.title = "💌 {{char}}에게서 답장".into();
        assert_eq!(title_of(&c, "아리아"), "💌 아리아에게서 답장");
    }
}
