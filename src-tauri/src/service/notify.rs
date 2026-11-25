//! 작업이 끝났는데 앱이 가려져 있으면 운영체제 알림 (roadmap 5-E-17·18, 기본 꺼짐).
//! 사용자는 알림 제목 글(`{{char}}` = 캐릭터/작업 이름), 아이콘 그림, 알림 소리(기본·무음·커스텀 파일)를 바꾼다.
//! 본문에는 대화 내용을 싣지 않는다(잠금 화면 보호).
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::models::settings::{AppSetting, DoneNotify, NotifySoundMode};
use crate::sys::settings::SettingsState;

pub const DEFAULT_TITLE: &str = "샌드캐슬";
pub const PLAY_SOUND_EVENT: &str = "notify:play-sound";

/// 프런트에 커스텀 알림음 재생을 요청하는 이벤트 페이로드
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NotifySoundPayload {
    pub path: String,
    pub volume: f32,
}

/// 알림 제목: 비면 앱 이름, `{{char}}`는 대상 이름
pub fn title_of(cfg: &DoneNotify, target_name: &str) -> String {
    let t = cfg.title.trim();
    let t = if t.is_empty() { DEFAULT_TITLE } else { t };
    t.replace("{{char}}", target_name).chars().take(80).collect()
}

/// 앱 창이 앞에 있고 포커스가 잡혀 있으면 알리지 않는다 (보고 있으니까)
fn app_in_front(app: &AppHandle) -> bool {
    let Some(w) = app.get_webview_window("main") else {
        return false;
    };
    if w.is_minimized().unwrap_or(false) || !w.is_visible().unwrap_or(true) {
        return false;
    }
    w.is_focused().unwrap_or(false)
}

fn read_cfg(app: &AppHandle) -> Option<DoneNotify> {
    app.try_state::<SettingsState<AppSetting>>()
        .and_then(|s| s.0.lock().ok().map(|g| g.done_notify.clone()))
}

/// 응답이 도착함 (켜져 있고 앱이 가려져 있을 때만). 실패해도 대화에는 영향 없음
pub fn reply_done(app: &AppHandle, char_name: &str) {
    task_done(app, char_name, &format!("{char_name}의 응답이 도착함"));
}

/// 일반 작업이 끝남 (켜져 있고 앱이 가려져 있을 때만). 실패해도 작업 결과에는 영향 없음
pub fn task_done(app: &AppHandle, target_name: &str, body: &str) {
    // 위젯 시메지에도 알림 이벤트를 전달한다 (OS 알림 설정과 무관하게 시메지가 반응할 수 있도록)
    let _ = app.emit(
        "widget_mascot::notify",
        crate::models::widget::WidgetMascotNotifyPayload {
            target_name: target_name.to_string(),
            body: body.to_string(),
        },
    );
    let Some(cfg) = read_cfg(app).filter(|c| c.enabled) else {
        return;
    };
    if app_in_front(app) {
        return;
    }
    if let Err(e) = dispatch(app, &cfg, target_name, body) {
        eprintln!("[notify] 알림을 띄우지 못함: {e}");
    }
}

/// 실제 알림 표시 + 커스텀 사운드 재생
fn dispatch(app: &AppHandle, cfg: &DoneNotify, target_name: &str, body: &str) -> Result<(), String> {
    let title = title_of(cfg, target_name);
    let icon = cfg
        .icon
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty() && std::path::Path::new(p).exists())
        .map(str::to_string);

    // 커스텀 사운드 파일이 실제로 존재하면 OS 토스트는 무음으로 띄우고 앱에서 오디오 재생
    let custom_sound = match cfg.sound_mode {
        NotifySoundMode::Custom => cfg
            .sound_path
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty() && std::path::Path::new(p).exists())
            .map(str::to_string),
        _ => None,
    };

    let os_silent = match cfg.sound_mode {
        NotifySoundMode::Mute => true,
        NotifySoundMode::Custom => custom_sound.is_some(),
        NotifySoundMode::Default => false,
    };

    show_os_notification(app, &title, body, icon.as_deref(), os_silent)?;

    if let Some(path) = custom_sound {
        let volume = cfg.sound_volume.clamp(0.0, 1.0);
        let _ = app.emit(PLAY_SOUND_EVENT, NotifySoundPayload { path, volume });
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn show_os_notification(app: &AppHandle, title: &str, body: &str, icon: Option<&str>, silent: bool) -> Result<(), String> {
    use std::path::Path;
    use tauri_winrt_notification::{Duration, IconCrop, Sound, Toast};

    let identifier = app.config().identifier.clone();
    let app_id = if ensure_windows_aumid(&identifier, DEFAULT_TITLE) {
        identifier
    } else {
        Toast::POWERSHELL_APP_ID.to_string()
    };

    let title = title.to_string();
    let body = body.to_string();
    let icon = icon.map(|p| p.strip_prefix(r"\\?\").unwrap_or(p).to_string());
    let app_for_click = app.clone();

    let send = move |aumid: &str| -> tauri_winrt_notification::Result<()> {
        let mut toast = Toast::new(aumid)
            .title(&title)
            .text1(&body)
            .duration(Duration::Short)
            .sound(if silent { None } else { Some(Sound::Default) });
        if let Some(ref icon_path) = icon {
            toast = toast.icon(Path::new(icon_path), IconCrop::Square, "");
        }
        let click_handle = app_for_click.clone();
        toast = toast.on_activated(move |_| {
            if let Some(w) = click_handle.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
            Ok(())
        });
        toast.show()
    };

    // WinRT Toast는 백그라운드 스레드에서 호출해 호출자(async task)를 블로킹하지 않는다
    std::thread::Builder::new()
        .name("win-notify".into())
        .spawn(move || {
            if let Err(e) = send(&app_id) {
                eprintln!("[notify] Windows 토스트 실패 ({app_id}): {e}");
                if app_id != Toast::POWERSHELL_APP_ID {
                    if let Err(e2) = send(Toast::POWERSHELL_APP_ID) {
                        eprintln!("[notify] Windows PowerShell 폴백 토스트도 실패: {e2}");
                    }
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Windows 10/11에서 비패키지(개발/포터블 포함) 앱이 Toast를 띄우려면 HKCU에 AUMID가 등록되어 있어야 한다.
/// 관리자 권한 없이 현재 사용자 레지스트리(`HKCU\Software\Classes\AppUserModelId\<aumid>`)에 한 번 등록한다.
#[cfg(target_os = "windows")]
fn ensure_windows_aumid(aumid: &str, display_name: &str) -> bool {
    use std::sync::OnceLock;
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegCloseKey, RegCreateKeyW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, REG_DWORD, REG_SZ};

    static REGISTERED: OnceLock<bool> = OnceLock::new();
    *REGISTERED.get_or_init(|| {
        let subkey: Vec<u16> = format!(r"Software\Classes\AppUserModelId\{aumid}")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut hkey = HKEY::default();
        // SAFETY: subkey is null-terminated UTF-16 and hkey is a valid stack pointer
        let ok = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, PCWSTR(subkey.as_ptr()), &mut hkey).is_ok() };
        if !ok {
            return false;
        }
        let name_key: Vec<u16> = "DisplayName".encode_utf16().chain(std::iter::once(0)).collect();
        let name_val: Vec<u16> = display_name.encode_utf16().chain(std::iter::once(0)).collect();
        let name_bytes: &[u8] = unsafe { std::slice::from_raw_parts(name_val.as_ptr() as *const u8, name_val.len() * 2) };
        let show_key: Vec<u16> = "ShowInSettings".encode_utf16().chain(std::iter::once(0)).collect();
        let show_val: u32 = 1;
        let show_bytes = show_val.to_le_bytes();
        // SAFETY: keys and values are valid UTF-16 / DWORD buffers for the open hkey
        unsafe {
            let _ = RegSetValueExW(hkey, PCWSTR(name_key.as_ptr()), 0, REG_SZ, Some(name_bytes));
            let _ = RegSetValueExW(hkey, PCWSTR(show_key.as_ptr()), 0, REG_DWORD, Some(&show_bytes));
            let _ = RegCloseKey(hkey);
        }
        true
    })
}

#[cfg(not(target_os = "windows"))]
fn show_os_notification(app: &AppHandle, title: &str, body: &str, icon: Option<&str>, silent: bool) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    let mut b = app.notification().builder().title(title.to_string()).body(body.to_string());
    if let Some(icon) = icon {
        b = b.icon(icon.to_string());
    }
    if silent {
        b = b.silent();
    }
    b.show().map_err(|e| e.to_string())
}

/// 알림을 켤 때 (Android는 사용자에게 권한을 묻는다). 허락됐으면 true
#[tauri::command]
#[specta::specta]
pub fn request_notify_permission(app: AppHandle) -> crate::error::Result<bool> {
    use tauri_plugin_notification::{NotificationExt, PermissionState};
    let n = app.notification();
    let state = match n.permission_state() {
        Ok(PermissionState::Granted) => PermissionState::Granted,
        _ => n.request_permission().map_err(|e| crate::error::AppError::internal(e.to_string()))?,
    };
    Ok(matches!(state, PermissionState::Granted))
}

/// 설정 화면에서 즉시 테스트 알림을 보낸다 (포커스 여부와 무관하게 표시 + 설정된 소리 재생)
#[tauri::command]
#[specta::specta]
pub fn test_notify(app: AppHandle) -> crate::error::Result<()> {
    let cfg = read_cfg(&app).unwrap_or_default();
    dispatch(&app, &cfg, "엘리안", "테스트 알림입니다").map_err(crate::error::AppError::internal)
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

    #[test]
    fn default_sound_settings_are_sensible() {
        let c = DoneNotify::default();
        assert!(!c.enabled);
        assert_eq!(c.sound_mode, NotifySoundMode::Default);
        assert!(c.sound_path.is_none());
        assert!((c.sound_volume - 0.8).abs() < f32::EPSILON);
    }
}
