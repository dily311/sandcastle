mod background_service;
pub use background_service::AppBackgroundService;

pub mod secret_store;

pub mod ai;
pub mod app_prompts;
pub mod budget;
pub mod chat;
pub mod community;
pub mod data;
pub mod device;
pub mod drive;
pub mod image_meta;
pub mod import;
pub mod memory;
pub mod picked_file;
pub mod plugins;
pub mod prices;
pub mod prompt;
pub mod proposals;
pub mod summary;
pub mod update;
pub use chat::chat_manager::ChatManager;
