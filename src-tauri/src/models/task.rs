use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::task::task_models::{AttemptStatus, TaskStatus, TaskType};

// =======================================
// DB 내 테이블 정의
// =======================================
// generation_tasks
#[derive(FromRow)]
pub struct GenerationTaskRow {
    pub id: String,
    pub task_type: String,
    pub status: String,
    pub payload_json: Option<String>,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTask {
    pub id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub payload_json: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}

impl From<GenerationTaskRow> for GenerationTask {
    fn from(r: GenerationTaskRow) -> Self {
        Self {
            id: r.id,
            task_type: TaskType::from_str(&r.task_type)
                .ok()
                .unwrap_or_else(|| TaskType::ChatStream),
            status: TaskStatus::from_str(&r.status)
                .ok()
                .unwrap_or_else(|| TaskStatus::Completed),
            payload_json: r
                .payload_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            error_message: r.error_message,
            created_at: r.created_at,
            started_at: r.started_at,
            updated_at: r.updated_at,
            finished_at: r.finished_at,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationTaskInput {
    pub id: Option<String>,
    pub task_type: String,
    pub status: String,
    pub payload_json: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

// ai_submissions

#[derive(FromRow)]
pub struct AiSubmissionRow {
    pub id: String,
    pub task_id: String,
    pub conversation_id: String,
    pub seq: u64,
    pub input_message_id: Option<String>,
    pub status: String,
    pub current_attempt_id: Option<String>,
    pub attempt_count: i64,
    pub max_attempts: i64,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSubmission {
    pub id: String,
    pub task_id: String,
    pub conversation_id: String,
    pub seq: u64,
    pub input_message_id: Option<String>,
    pub status: String,
    pub current_attempt_id: Option<String>,
    pub attempt_count: i64,
    pub max_attempts: i64,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}

impl From<AiSubmissionRow> for AiSubmission {
    fn from(r: AiSubmissionRow) -> Self {
        Self {
            id: r.id,
            task_id: r.task_id,
            conversation_id: r.conversation_id,
            seq: r.seq,
            input_message_id: r.input_message_id,
            status: r.status,
            current_attempt_id: r.current_attempt_id,
            attempt_count: r.attempt_count,
            max_attempts: r.max_attempts,
            created_at: r.created_at,
            started_at: r.started_at,
            updated_at: r.updated_at,
            finished_at: r.finished_at,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSubmissionInput {
    pub id: Option<String>,
    pub task_id: String,
    pub conversation_id: String,
    pub seq: u64,
    pub input_message_id: Option<String>,
    pub status: String,
    pub current_attempt_id: Option<String>,
    pub attempt_count: i64,
    pub max_attempts: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

#[derive(FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct AttemptInfo {
    pub id: String,
    pub submission_id: String,
    pub attempt_no: i64,
    pub status: AttemptStatus,
    pub error_code: String,
    pub error_message: String,
    pub started_at: i64,
    pub finished_at: i64,
}
