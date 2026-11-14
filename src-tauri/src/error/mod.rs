mod app_error;
mod conversions;
mod error_code;
mod error_response;

pub use app_error::{AppError, Result};
pub use error_code::ErrorCode;
pub use error_response::AppErrorResponse;

pub fn network_error(status: reqwest::StatusCode) -> Result<()> {
    match status.as_u16() {
        401 | 403 => Err(AppError::ai_provider(
            ErrorCode::AiProviderAuthFailed,
            "API 인증에 실패했습니다",
            false,
        )),

        404 => Err(AppError::ai_provider(
            ErrorCode::AiProviderModelNotFound,
            "요청한 모델을 찾을 수 없습니다",
            false,
        )),

        429 => Err(AppError::ai_provider(
            ErrorCode::AiProviderRateLimit,
            "API 요청 한도를 초과했습니다",
            true,
        )),

        500..=599 => Err(AppError::ai_provider(
            ErrorCode::AiProviderError,
            "AI 서버에서 오류가 발생했습니다",
            true,
        )),

        _ => Ok(()),
    }
}
