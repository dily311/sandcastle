use crate::widget::WidgetInteractionState;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::{AppHandle, Manager, WebviewWindow};

#[cfg(target_os = "windows")]
use windows::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};

#[cfg(target_os = "windows")]
fn get_cursor_position() -> Option<(i32, i32)> {
    unsafe {
        let mut point = POINT::default();

        if GetCursorPos(&mut point).is_ok() {
            Some((point.x, point.y))
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn get_cursor_position() -> Option<(i32, i32)> {
    None
}

fn get_cursor_position_in_window(window: &WebviewWindow) -> Option<(f64, f64)> {
    let (cursor_x, cursor_y) = get_cursor_position()?;
    let window_position = window.outer_position().ok()?;

    let scale_factor = window.scale_factor().ok()?;

    let x = (cursor_x - window_position.x) as f64 / scale_factor;
    let y = (cursor_y - window_position.y) as f64 / scale_factor;
    Some((x, y))
}

// 위젯 윈도우의 setIgnoreCursor의 값이 변하는 것을 체크
fn check_interactive(window: &tauri::WebviewWindow, app: &AppHandle) -> bool {
    let Some((x, y)) = get_cursor_position_in_window(window) else {
        return false;
    };

    let state = app.state::<WidgetInteractionState>();

    let regions = match state.regions.lock() {
        Ok(regions) => regions,
        Err(_) => return false,
    };

    regions.iter().any(|region| region.1.contains(x, y))
}

pub struct MouseTrackerState {
    pub generation: Arc<AtomicU64>,
}

impl MouseTrackerState {
    pub fn new() -> Self {
        Self {
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn start_tracker(app: AppHandle, state: &MouseTrackerState) {
        let generation_atomic = state.generation.clone();
        let generation = generation_atomic.fetch_add(1, Ordering::SeqCst) + 1;

        std::thread::spawn(move || {
            while generation_atomic.load(Ordering::SeqCst) == generation {
                let Some(window) = app.get_webview_window("widget") else {
                    break;
                };

                if !window.is_visible().unwrap_or(false) {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Hit Test
                let is_interactive = check_interactive(&window, &app);
                let _ = window.set_ignore_cursor_events(!is_interactive);
                std::thread::sleep(Duration::from_millis(16));
            }
        });
    }
    pub fn stop_tracker(state: &MouseTrackerState) {
        state.generation.fetch_add(1, Ordering::SeqCst);
    }
}
