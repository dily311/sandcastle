use serde_json::Value;
use thiserror::Error;

use crate::error::{error_code::ErrorCategory, error_response::AppErrorResponse, ErrorCode};

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("archive error: {message}")]
    // zip 관련 (경로 순회, 손상된 파일 등)
    Archive { code: ErrorCode, message: String, details: Option<Value> },

    #[error("not found: {message}")]
    // 존재하지 않는 리소스
    NotFound { code: ErrorCode, message: String },

    #[error("validation error: {message}")]
    // 입력 문제
    Validation { code: ErrorCode, message: String, details: Option<Value> },

    #[error("db error: {message}")] // DB 에러 -> 재시도 유도 등
    Database { code: ErrorCode, message: String },

    #[error("ai provider error: {message}")]
    // API 호출 실패
    AiProvider {
        code: ErrorCode,
        message: String,
        retryable: bool,
        details: Option<Value>,
    },

    #[error("game error: {message}")]
    Game { code: ErrorCode, message: String, details: Option<Value> },

    #[error("io error: {message}")]
    Io { message: String },

    #[error("window error: {message}")]
    Window { message: String },

    #[error("internal error: {message}")]
    Internal { message: String },
}

impl AppError {
    pub fn to_response(&self) -> AppErrorResponse {
        let (code, category, message, retryable, details) = match self {
            Self::Archive { code, message, details } => (*code, ErrorCategory::Archive, message.clone(), false, details.clone()),

            Self::NotFound { code, message } => (*code, ErrorCategory::NotFound, message.clone(), false, None),

            Self::Validation { code, message, details } => (*code, ErrorCategory::Validation, message.clone(), false, details.clone()),

            Self::Database { code, message } => (*code, ErrorCategory::Database, message.clone(), *code == ErrorCode::DatabaseLocked, None),

            Self::AiProvider {
                code,
                message,
                retryable,
                details,
            } => (*code, ErrorCategory::AiProvider, message.clone(), *retryable, details.clone()),

            Self::Game { code, message, details } => (*code, ErrorCategory::Game, message.clone(), false, details.clone()),

            Self::Io { message } => (ErrorCode::IoError, ErrorCategory::Io, message.clone(), false, None),

            Self::Window { message } => (ErrorCode::WindowError, ErrorCategory::Window, message.clone(), false, None),

            Self::Internal { message } => (ErrorCode::InternalError, ErrorCategory::Internal, message.clone(), false, None),
        };

        AppErrorResponse {
            code,
            category,
            message,
            retryable,
            details,
        }
    }

    pub fn archive(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Archive {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn validation(message: impl Into<String>, details: Option<Value>) -> Self {
        Self::Validation {
            code: ErrorCode::InvalidInput,
            message: message.into(),
            details,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            code: ErrorCode::ResourceNotFound,
            message: message.into(),
        }
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::Database {
            code: ErrorCode::DatabaseError,
            message: message.into(),
        }
    }

    pub fn ai_provider(code: ErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self::AiProvider {
            code,
            message: message.into(),
            retryable,
            details: None,
        }
    }

    pub fn game(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Game {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::Io { message: message.into() }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal { message: message.into() }
    }

    pub fn window(message: impl Into<String>) -> Self {
        Self::Window { message: message.into() }
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_response().serialize(serializer)
    }
}

/// 프론트에는 `to_response()` 형태(AppErrorResponse)로 직렬화되므로 타입도 그것으로 내보낸다.
impl specta::Type for AppError {
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType {
        <AppErrorResponse as specta::Type>::definition(types)
    }
}
