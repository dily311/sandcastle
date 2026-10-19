use lofty::error::LoftyError;
use serde::Serialize;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug, Serialize, Clone)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    #[error("archive error: {0}")]
    Archive(String), // zip 관련 (경로 순회, 손상된 파일 등)

    #[error("not found: {0}")]
    NotFound(String), // 존재하지 않는 리소스

    #[error("validation error: {0}")]
    Validation(String), // 입력 문제

    #[error("db error: {0}")] // DB 에러 -> 재시도 유도 등
    Database(String),

    #[error("ai provider error: {0}")]
    AiProvider(String), // API 호출 실패

    #[error("io error: {0}")]
    Io(String),

    #[error("window error: {0}")]
    Window(String),
}

// =======================================
// From 구현
// =======================================
// From 구현으로 std::io::Error, rusqlite::Error, zip::result::ZipError 등을
// AppError로 변환 (?  연산자 그대로 쓰기 위함)
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}
impl From<std::string::String> for AppError {
    fn from(e: std::string::String) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => {
                AppError::NotFound("요청한 데이터를 찾을 수 없습니다".into())
            }
            other => AppError::Database(other.to_string()),
        }
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Window(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

#[cfg(not(mobile))]
impl From<LoftyError> for AppError {
    fn from(e: LoftyError) -> Self {
        AppError::Io(e.to_string())
    }
}
