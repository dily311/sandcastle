use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, EnumString, Display)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, EnumString, Display)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, EnumString, Display)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    ChatStream,
    ImageGen,
    BackgroundSync,
    MemorySummary,
    Embedding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TaskEvent {
    Started {
        task_id: String,
    },
    Progress {
        task_id: String,
        current: u64,
        total: Option<u64>,
    },
    TextDelta {
        task_id: String,
        content: String,
    },
    ToolCallStarted {
        task_id: String,
        tool_call_id: String,
        tool_name: String,
    },
    ToolCallCompleted {
        task_id: String,
        tool_call_id: String,
    },
    Completed {
        task_id: String,
    },
    Failed {
        task_id: String,
        error: TaskErrorInfo,
    },
    Interrupted {
        task_id: String,
    },
}

#[derive(Debug, Clone)]
pub struct TaskContext {
    pub task_id: String,
    pub attempt_id: String,
    pub attempt_no: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskErrorInfo {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug)]
pub enum TaskOutput {
    Chat { message_id: String },
    Image { asset_id: String },
    Sync { synced_count: u64 },
    Memory { summary: String },
}

pub enum CancelReason {
    UserCancelled,
    ServiceShutdown,
    AppShutdown,
}

// =======================================
// DB 내 테이블 정의
// =======================================
#[derive(FromRow)]
pub struct TaskInfoRow {
    pub id: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub payload_json: Option<String>,
    pub error_message: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
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

impl From<TaskInfoRow> for TaskInfo {
    fn from(r: TaskInfoRow) -> Self {
        Self {
            id: r.id,
            task_type: r.task_type,
            status: r.status,
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
pub struct TaskInfoInput {
    pub id: Option<String>,
    pub task_type: String,
    pub status: String,
    pub payload_json: Option<serde_json::Value>,
    pub error_message: Option<String>,
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

pub struct AttemptResult {}

pub struct CreateTaskRequest {}

pub struct NewTask {}
use thiserror::Error;
#[derive(Error, Debug, Serialize, Clone)]
#[serde(tag = "kind", content = "message")]
pub enum TaskError {
    #[error("지원하지 않는 타입에러")]
    UnsupportedTaskType(TaskType),
}
