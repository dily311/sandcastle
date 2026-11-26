use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::error::{error_code::ErrorCategory, ErrorCode};

// 프론트엔드 전달용 에러 구조체 (ADR-032 ④: 원문 message와 함께 다국어 key·args 전달)
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorResponse {
    pub code: ErrorCode,
    pub category: ErrorCategory,
    pub message: String,

    pub retryable: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<BTreeMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}
