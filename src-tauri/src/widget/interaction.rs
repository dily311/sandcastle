use crate::error::AppError;
use super::regions::InteractiveRegion;
use std::{collections::HashMap, sync::Mutex};
use tauri::{State, WebviewWindow};

pub struct WidgetInteractionState {
    pub regions: Mutex<HashMap<String, InteractiveRegion>>,
}

impl WidgetInteractionState {
    pub fn new() -> Self {
        Self {
            regions: Mutex::new(HashMap::new()),
        }
    }
}

#[tauri::command]
pub fn update_interactive_regions(
    upserts: Vec<InteractiveRegion>,
    removed: Vec<String>,
    state: State<WidgetInteractionState>,
) -> Result<(), AppError> {
    let mut regions = state.regions.lock().unwrap();

    for region in upserts {
        regions.insert(region.id.clone(), region);
    }
    for id in removed {
        regions.remove(&id);
    }

    Ok(())
}

#[tauri::command]
pub fn remove_interactive_regions(
    ids: Vec<String>,
    state: State<WidgetInteractionState>,
) -> Result<(), AppError> {
    let mut regions = state.regions.lock().unwrap();

    for id in ids {
        regions.remove(&id);
    }
    Ok(())
}

#[tauri::command]
pub fn set_widget_click_through(window: WebviewWindow, enabled: bool) -> Result<(), AppError> {
    window.set_ignore_cursor_events(enabled)?;
    Ok(())
}
