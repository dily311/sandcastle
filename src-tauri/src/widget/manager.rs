use crate::{error::Result, widget::interaction::MouseTrackerState};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
const WIDGET_LABEL: &str = "widget";
const WIDGET_TRAY_LABEL: &str = "widget-tray";
const EVENT_WIDGET_SHOW: &str = "widget:shown";
const EVENT_WIDGET_HIDE: &str = "widget:hidden";

fn show_tray_for_window<R: Runtime>(window: &WebviewWindow<R>) -> Result<()> {
    let app_handle = window.app_handle();

    // 1. 트레이가 이미 존재하지 않는 경우에만 새로 생성 (중복 생성 방지)
    if app_handle.tray_by_id(WIDGET_TRAY_LABEL).is_none() {
        let is_en = app_handle
            .try_state::<crate::sys::settings::SettingsState<crate::models::settings::AppSetting>>()
            .and_then(|s| s.0.lock().ok().map(|g| g.language.trim().to_ascii_lowercase()))
            .is_some_and(|l| !l.is_empty() && !l.starts_with("ko"));
        let (open_lbl, focus_lbl, quit_lbl) = if is_en {
            ("Open Main Window", "Focus Widget", "Quit Widget")
        } else {
            ("메인 윈도우 열기", "위젯 포커스", "위젯 종료")
        };
        // 2. 트레이 메뉴 구성
        let open_main_i = MenuItem::with_id(app_handle, "open_main", open_lbl, true, None::<&str>)?;
        let focus_i = MenuItem::with_id(app_handle, "focus", focus_lbl, true, None::<&str>)?;
        let quit_i = MenuItem::with_id(app_handle, "quit", quit_lbl, true, None::<&str>)?;
        let menu = Menu::with_items(app_handle, &[&open_main_i, &focus_i, &quit_i])?;

        let widget_window = window.clone();
        // 3. 트레이 아이콘 동적 생성
        let _tray = TrayIconBuilder::with_id(WIDGET_TRAY_LABEL)
            .icon(app_handle.default_window_icon().unwrap().clone())
            .menu(&menu)
            .on_menu_event(move |app, event| match event.id.as_ref() {
                "open_main" => {
                    let _ = open_main_window(app);
                }
                "focus" => {
                    let _ = widget_window.set_focus();
                }
                "quit" => {
                    let _ = widget_window.close();
                    let _ = app.remove_tray_by_id(WIDGET_TRAY_LABEL);
                }
                _ => {}
            })
            .build(app_handle)?;
    }

    Ok(())
}

fn add_tray_event<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    if let Some(window) = app.get_webview_window(WIDGET_LABEL) {
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(_tray) = app.tray_by_id(WIDGET_TRAY_LABEL) {
                    let _ = app.remove_tray_by_id(WIDGET_TRAY_LABEL);
                }
            }
        });
    }
    Ok(())
}

fn open_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.show();
        let _ = main_window.set_focus();
    } else {
        // 꺼져 있는 경우 키려고 할때
        let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
            .title("sand")
            .decorations(false)
            .transparent(true)
            .resizable(true)
            .fullscreen(false)
            .devtools(true)
            .build()?;
        let _ = window.show();
        let _ = window.set_focus();
    }

    Ok(())
}

/// 창 열기·닫기를 한 번에 하나씩 (설정 이벤트와 명령이 겹쳐 같은 창이 둘 생기지 않게)
static WINDOW_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct WidgetManager;
impl WidgetManager {
    // 위젯 윈도우
    pub fn show(app: &AppHandle, state: &MouseTrackerState) -> Result<()> {
        if let Some(window) = app.get_webview_window(WIDGET_LABEL) {
            window.show()?;
            window.set_focus()?;
            MouseTrackerState::start_tracker(app.clone(), state);
            app.emit(EVENT_WIDGET_SHOW, ())?;
        }

        Ok(())
    }

    pub fn hide(app: &AppHandle, state: &MouseTrackerState) -> Result<()> {
        if let Some(window) = app.get_webview_window(WIDGET_LABEL) {
            MouseTrackerState::stop_tracker(state);
            window.hide()?;
            app.emit(EVENT_WIDGET_HIDE, ())?;
        }

        Ok(())
    }

    pub fn toggle_widget(app: &AppHandle, state: &MouseTrackerState) -> Result<()> {
        if let Some(window) = app.get_webview_window(WIDGET_LABEL) {
            if window.is_visible()? {
                return Self::hide(app, state);
            } else {
                return Self::show(app, state);
            }
        }

        let _ = &Self::open_widget(app, state)?;
        Ok(())
    }

    pub fn open_widget(app: &AppHandle, state: &MouseTrackerState) -> Result<WebviewWindow> {
        let _guard = WINDOW_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // 이미 존재하면 새로 만들지 않는다.
        if let Some(window) = app.get_webview_window(WIDGET_LABEL) {
            Self::show(app, state)?;
            return Ok(window);
        }

        let window = WebviewWindowBuilder::new(app, WIDGET_LABEL, WebviewUrl::App("widget.html".into()))
            .title("widget")
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .maximized(true)
            .shadow(false)
            .skip_taskbar(true)
            .devtools(true)
            .build()?;

        window.set_ignore_cursor_events(true)?;
        // 시스템 트레이 생성
        show_tray_for_window(&window)?;
        // 윈도우 이벤트 등록
        add_tray_event(app.clone())?;

        // 마우스 트레커 관찰 시작
        MouseTrackerState::start_tracker(app.clone(), state);
        Ok(window)
    }

    pub fn close_widget(app: &AppHandle, state: &MouseTrackerState) -> Result<()> {
        let _guard = WINDOW_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        MouseTrackerState::stop_tracker(state);
        if let Some(window) = app.get_webview_window(WIDGET_LABEL) {
            // close는 닫기 요청(창이 막을 수도 있음), destroy는 바로 없앤다. 설정으로 끈 것이므로 바로 없앤다
            window.destroy()?;
        }
        let _ = app.remove_tray_by_id(WIDGET_TRAY_LABEL);
        Ok(())
    }
}
