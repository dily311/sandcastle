pub mod settings;
pub mod task;
pub mod ai_service;
pub mod regex;

pub mod conversation;

pub mod game_manager;
pub mod game_engine;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod widget;
