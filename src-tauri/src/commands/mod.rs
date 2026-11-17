pub mod ai_service;
pub mod character_image;
pub mod community;
pub mod data;
pub mod import;
pub mod preset;
pub mod regex;
pub mod settings;
pub mod sync;
pub mod task;

pub mod chat;
pub mod conversation;
pub mod convert;
pub mod phone;
pub mod plugins;

pub mod game_engine;
pub mod game_manager;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod widget;
