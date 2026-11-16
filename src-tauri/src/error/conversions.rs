// =======================================
// From 구현
// =======================================
// From 구현으로 std::io::Error, rusqlite::Error, zip::result::ZipError 등을
// AppError로 변환 (?  연산자 그대로 쓰기 위함)

use crate::error::{AppError, ErrorCode};

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io { message: e.to_string() }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => Self::not_found("요청한 데이터를 찾을 수 없습니다"),
            sqlx::Error::Database(ref db_err) if db_err.code().as_deref() == Some("5") || db_err.code().as_deref() == Some("6") => Self::Database {
                code: ErrorCode::DatabaseLocked,
                message: "데이터베이스가 사용 중입니다".into(),
            },
            other => Self::database(other.to_string()),
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        let mut code = ErrorCode::AiProviderError;
        if e.is_timeout() {
            code = ErrorCode::AiProviderTimeout;
        }

        Self::ai_provider(code, e.to_string(), false)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::internal(format!("JSON 처리 실패: {e}"))
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        Self::Window { message: e.to_string() }
    }
}

impl From<notify::Error> for AppError {
    fn from(e: notify::Error) -> Self {
        Self::Window { message: e.to_string() }
    }
}
