mod background_service;
pub use background_service::AppBackgroundService;

pub mod secret_store;

pub mod chat;
pub use chat::chat_manager::ChatManager;