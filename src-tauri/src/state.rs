use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    game::runtime::{GameDefinition, GameRuntime},
    models::game::GameProject,
};
pub struct AppState {
    pub db: SqlitePool,
    pub http_client: Client,
    // 현재 로드된 게임 아카이브
    pub active_archive: Mutex<Option<GameArchiveHandle>>, // 기존 그대로
    pub active_runtime: tokio::sync::Mutex<Option<GameRuntime>>, // 새로 추가

    pub ai_rate_limit: Mutex<HashMap<String, VecDeque<i64>>>,

    // 현재 윈도우 모드
    pub window_mode: Mutex<WindowMode>,
    pub launch_mode: LaunchMode,
}

pub struct GameArchiveHandle {
    pub path: PathBuf,
    pub manifest: GameProject,
    pub game_id: String,
    pub definition: Arc<GameDefinition>,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum WindowMode {
    Explore,
    Mascot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode {
    Normal,
    Widget,
}

impl LaunchMode {
    pub fn from_args() -> Self {
        if std::env::args().any(|arg| arg == "--widget") {
            Self::Widget
        } else {
            Self::Normal
        }
    }
}
