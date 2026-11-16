mod mouse_tracker;
pub use mouse_tracker::MouseTrackerState;
use std::{collections::HashMap, sync::Mutex};

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

#[derive(Debug, Clone, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InteractiveRegion {
    pub id: String,

    pub x: f64,
    pub y: f64,

    pub width: f64,
    pub height: f64,
}

impl InteractiveRegion {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}
