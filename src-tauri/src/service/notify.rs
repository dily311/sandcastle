//! 작업이 끝났는데 앱이 가려져 있으면 운영체제 알림 (roadmap 5-E-17·18, 기본 꺼짐).
//! 사용자는 알림 제목 글(`{{char}}` = 캐릭터/작업 이름), 아이콘 그림, 알림 소리(기본·무음·커스텀 파일)를 바꾼다.
//! 본문에는 대화 내용을 싣지 않는다(잠금 화면 보호).
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::models::settings::{AppSetting, DoneNotify, NotifySoundMode};
use crate::sys::settings::SettingsState;

pub const DEFAULT_TITLE: &str = "샌드캐슬";
pub const DEFAULT_TITLE_EN: &str = "Sandcastle";
pub const PLAY_SOUND_EVENT: &str = "notify:play-sound";

/// 프런트에 커스텀 알림음 재생을 요청하는 이벤트 페이로드
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NotifySoundPayload {
    pub path: String,
    pub volume: f32,
}

fn is_english_lang(lang: &str) -> bool {
    let l = lang.trim().to_ascii_lowercase();
    !l.is_empty() && !l.starts_with("ko")
}

/// 알림 제목: 비면 앱 이름(현재 언어 기준), `{{char}}`는 대상 이름
pub fn title_of(cfg: &DoneNotify, target_name: &str) -> String {
    title_of_lang(cfg, target_name, "ko")
}

pub fn title_of_lang(cfg: &DoneNotify, target_name: &str, lang: &str) -> String {
    let t = cfg.title.trim();
    let fallback = if is_english_lang(lang) { DEFAULT_TITLE_EN } else { DEFAULT_TITLE };
    let t = if t.is_empty() { fallback } else { t };
    t.replace("{{char}}", target_name).chars().take(80).collect()
}

/// 백엔드가 고정 문구로 넣은 알림 대상·본문을 현재 설정 언어(`AppSetting.language`)에 맞게 변환한다.
pub fn localize_pair(target_name: &str, body: &str, lang: &str) -> (String, String) {
    if !is_english_lang(lang) {
        return (target_name.to_string(), body.to_string());
    }
    let loc_target = match target_name {
        "게임 대화" => "Game Dialogue",
        "장면 진행" => "Scene",
        "게임 응답" => "Game Reply",
        "각본가" => "Story Planner",
        "대필" => "Ghostwrite",
        "메모리 색인" => "Memory Index",
        "그림 생성" => "Image Generation",
        "자동 그림" => "Auto Image",
        "게임 장면 그림" => "Game Scene Image",
        "폰 카메라" => "Phone Camera",
        "그림 만들기" => "Image Generation",
        "대화 요약" => "Summary",
        "구글 드라이브" => "Google Drive",
        "폴더 동기화" => "Folder Sync",
        "SillyTavern" => "SillyTavern",
        other => other,
    };
    let loc_body = match body {
        "캐릭터 응답이 도착함" => "Character reply arrived".to_string(),
        "장면 응답이 도착함" => "Scene reply arrived".to_string(),
        "다시 생성된 응답이 도착함" => "Regenerated reply arrived".to_string(),
        "스토리 플랜 갱신이 끝남" => "Story plan updated".to_string(),
        "대필이 끝남" => "Ghostwrite completed".to_string(),
        "메모리 색인이 끝남" => "Memory indexing completed".to_string(),
        "메시지 그림이 완성됨" => "Message image generated".to_string(),
        "자동 그림이 완성됨" => "Auto image generated".to_string(),
        "그림이 완성됨" => "Image generated".to_string(),
        "생성이 끝남" => "Generation completed".to_string(),
        "대화 요약이 끝남" => "Conversation summary completed".to_string(),
        "동기화가 완료됨" => "Sync completed".to_string(),
        "SillyTavern 데이터 가져오기가 완료됨" => "SillyTavern data import completed".to_string(),
        "SillyTavern 데이터 내보내기가 완료됨" => "SillyTavern data export completed".to_string(),
        other => {
            if let Some(who) = other.strip_suffix("의 응답이 도착함") {
                format!("Reply from {who} arrived")
            } else if let Some(n) = other.strip_prefix("드라이브 동기화 파일 ").and_then(|s| s.strip_suffix("개를 삭제했습니다")) {
                format!("Deleted {n} sync files from Google Drive")
            } else if let Some(reason) = other.strip_prefix("드라이브 데이터 삭제 실패: ") {
                format!("Failed to delete Google Drive data: {reason}")
            } else if let Some(reason) = other.strip_prefix("동기화 실패: ") {
                format!("Sync failed: {reason}")
            } else if let Some(reason) = other.strip_prefix("SillyTavern 가져오기 실패: ") {
                format!("SillyTavern import failed: {reason}")
            } else if let Some(reason) = other.strip_prefix("SillyTavern 내보내기 실패: ") {
                format!("SillyTavern export failed: {reason}")
            } else {
                other.to_string()
            }
        }
    };
    (loc_target.to_string(), loc_body)
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

fn read_cfg_and_lang(app: &AppHandle) -> (Option<DoneNotify>, String) {
    match app.try_state::<SettingsState<AppSetting>>() {
        Some(s) => match s.0.lock() {
            Ok(g) => (Some(g.done_notify.clone()), g.language.clone()),
            Err(_) => (None, String::new()),
        },
        None => (None, String::new()),
    }
}

/// 응답이 도착함 (켜져 있고 앱이 가려져 있을 때만). 실패해도 대화에는 영향 없음
pub fn reply_done(app: &AppHandle, char_name: &str) {
    task_done(app, char_name, &format!("{char_name}의 응답이 도착함"));
}

/// 일반 작업이 끝남 (켜져 있고 앱이 가려져 있을 때만). 실패해도 작업 결과에는 영향 없음
pub fn task_done(app: &AppHandle, target_name: &str, body: &str) {
    let (cfg_opt, lang) = read_cfg_and_lang(app);
    let (target_loc, body_loc) = localize_pair(target_name, body, &lang);
    // 위젯 시메지에도 알림 이벤트를 전달한다 (OS 알림 설정과 무관하게 시메지가 반응할 수 있도록)
    let _ = app.emit(
        "widget_mascot::notify",
        crate::models::widget::WidgetMascotNotifyPayload {
            target_name: target_loc.clone(),
            body: body_loc.clone(),
        },
    );
    let Some(cfg) = cfg_opt.filter(|c| c.enabled) else {
        return;
    };
    if app_in_front(app) {
        return;
    }
    if let Err(e) = dispatch(app, &cfg, &lang, &target_loc, &body_loc) {
        eprintln!("[notify] 알림을 띄우지 못함: {e}");
    }
}

/// 앱 시작 시 알림 등록 초기화 (Windows AUMID 사전 등록, 1-J-5)
pub fn init_on_startup(app: &AppHandle) {
    #[cfg(target_os = "windows")]
    {
        let _ = ensure_windows_aumid(&app.config().identifier, DEFAULT_TITLE);
    }
    #[cfg(not(target_os = "windows"))]
    let _ = app;
}

/// 동기화·드라이브 데이터 삭제 등 백그라운드 시스템 작업 알림.
/// `even_in_front`가 true면 앱이 앞에 있어도 OS 알림을 함께 띄운다 (드라이브 데이터 삭제 결과 등).
pub fn system_notify(app: &AppHandle, target_name: &str, body: &str, even_in_front: bool) {
    let (cfg_opt, lang) = read_cfg_and_lang(app);
    let (target_loc, body_loc) = localize_pair(target_name, body, &lang);
    let _ = app.emit(
        "widget_mascot::notify",
        crate::models::widget::WidgetMascotNotifyPayload {
            target_name: target_loc.clone(),
            body: body_loc.clone(),
        },
    );
    if !even_in_front && app_in_front(app) {
        return;
    }
    let cfg = cfg_opt.unwrap_or_default();
    if let Err(e) = dispatch(app, &cfg, &lang, &target_loc, &body_loc) {
        eprintln!("[notify] 시스템 알림을 띄우지 못함: {e}");
    }
}

/// 실제 알림 표시 + 커스텀 사운드 재생
fn dispatch(app: &AppHandle, cfg: &DoneNotify, lang: &str, target_name: &str, body: &str) -> Result<(), String> {
    let title = title_of_lang(cfg, target_name, lang);
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
    let (cfg_opt, lang) = read_cfg_and_lang(&app);
    let cfg = cfg_opt.unwrap_or_default();
    let (who, body) = if is_english_lang(&lang) {
        ("Elian", "This is a test notification.")
    } else {
        ("엘리안", "테스트 알림입니다")
    };
    dispatch(&app, &cfg, &lang, who, body).map_err(crate::error::AppError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_uses_char_and_falls_back_to_app_name() {
        let mut c = DoneNotify::default();
        assert_eq!(title_of(&c, "아리아"), "샌드캐슬");
        assert_eq!(title_of_lang(&c, "Aria", "en"), "Sandcastle");
        c.title = "💌 {{char}}에게서 답장".into();
        assert_eq!(title_of(&c, "아리아"), "💌 아리아에게서 답장");
    }

    #[test]
    fn localizes_notification_pairs_in_english() {
        assert_eq!(
            localize_pair("아리아", "아리아의 응답이 도착함", "en"),
            ("아리아".into(), "Reply from 아리아 arrived".into())
        );
        assert_eq!(
            localize_pair("게임 대화", "캐릭터 응답이 도착함", "en"),
            ("Game Dialogue".into(), "Character reply arrived".into())
        );
        assert_eq!(localize_pair("대필", "대필이 끝남", "ko"), ("대필".into(), "대필이 끝남".into()));
        assert_eq!(
            localize_pair("구글 드라이브", "드라이브 동기화 파일 12개를 삭제했습니다", "en"),
            ("Google Drive".into(), "Deleted 12 sync files from Google Drive".into())
        );
        assert_eq!(
            localize_pair("SillyTavern", "SillyTavern 데이터 내보내기가 완료됨", "en"),
            ("SillyTavern".into(), "SillyTavern data export completed".into())
        );
        assert_eq!(
            localize_pair("SillyTavern", "SillyTavern 가져오기 실패: 디스크 부족", "en"),
            ("SillyTavern".into(), "SillyTavern import failed: 디스크 부족".into())
        );
    }

    #[test]
    fn default_sound_settings_are_sensible() {
        let c = DoneNotify::default();
        assert!(c.enabled);
        assert!(c.initialized_on);
        assert_eq!(c.sound_mode, NotifySoundMode::Default);
        assert!(c.sound_path.is_none());
        assert!((c.sound_volume - 0.8).abs() < f32::EPSILON);
    }
}
