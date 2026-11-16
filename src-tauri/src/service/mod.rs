mod background_service;
pub use background_service::AppBackgroundService;

pub mod secret_store;

pub mod ai;
pub mod app_prompts;
pub mod budget;
pub mod image_meta;
pub mod chat;
pub mod data;
pub mod import;
pub mod plugins;
pub mod prompt;
pub use chat::chat_manager::ChatManager;