//! Task 공용 타입.
//!
//! 이 파일의 타입은 다른 모듈(ChatManager, GameRuntime, ImageManager, 커맨드)에서도 읽을 수 있다.
//! 단, 상태를 "바꾸는" 수단은 여기 없다. 상태 변경은 TaskManager → TaskRepository 경로로만 일어난다.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::prelude::FromRow;
use std::str::FromStr;
use strum_macros::{Display, EnumString};
use thiserror::Error;

use crate::error::{AppError, ErrorCode};

// =======================================
// 상태 / 종류
// =======================================

/// 작업 상태. 전이 규칙은 task_state.rs에 있다.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, EnumString, Display, specta::Type)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TaskStatus {
    /// 실행 대기 (큐에 들어갔지만 실행 슬롯을 아직 못 받음)
    Pending,
    /// 실행 중 (attempt가 하나 열려 있음)
    Running,
    /// 정상 완료
    Completed,
    /// 실패 (재시도 횟수 소진 또는 재시도 불가 오류)
    Failed,
    /// 사용자 취소 / 복구 포기
    Cancelled,
    /// 앱·서비스 종료로 중단. 사용자가 재개(resume)하거나 포기(dismiss)할 수 있다.
    Interrupted,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }

    /// 아직 메모리 큐/실행기에 올라가 있어야 하는 상태
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Pending | Self::Running)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, EnumString, Display, specta::Type)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AttemptStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

/// 작업 종류. 종류마다 TaskExecutor가 하나씩 등록된다.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, EnumString, Display, specta::Type)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TaskType {
    /// 일반 채팅 응답 생성 (ChatManager)
    ChatStream,
    /// 게임 NPC/씬 대화 생성 (GameRuntime)
    GameDialogue,
    /// 이미지 생성 (ImageManager)
    ImageGen,
    BackgroundSync,
    MemorySummary,
    Embedding,
    /// 번역 (코어 플러그인)
    Translate,
    /// 음성 읽기 (코어 플러그인)
    Tts,
    /// API 연결 확인 등 설정 화면의 짧은 호출
    ApiCall,
    /// 커뮤니티 플러그인이 부른 AI 글 생성 (feature `plugin:<id>`, 6-D-7)
    PluginAi,
}

/// 작업을 누가 시작했나. 비용이 드는 작업은 항상 실행 주체가 분명해야 한다 (CLAUDE.md 원칙)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, EnumString, Display, specta::Type, Default)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RunBy {
    /// 사용자가 누른 버튼·전송
    #[default]
    User,
    /// 사용자가 켜 둔 자동 실행 (자동 번역·자동 읽기 등)
    Auto,
}

/// 작업을 요청한 쪽. 프론트가 어느 화면에 결과를 붙일지 고를 때 쓴다.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, EnumString, Display, specta::Type)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TaskOwner {
    Chat,
    Game,
    Image,
    System,
}

// =======================================
// 요청 / 결과
// =======================================

/// 작업 생성 요청. 각 Manager는 이 값을 만들어 TaskManager::submit에 넘기기만 한다.
#[derive(Debug, Clone)]
pub struct CreateTaskRequest {
    pub task_type: TaskType,
    pub owner: TaskOwner,
    /// owner 쪽 식별자 (conversation_id, game session_id, asset_id 등)
    pub owner_ref: Option<String>,
    /// Executor가 해석할 인자
    pub payload: Value,
    /// 재시도 가능한 오류가 났을 때 한 번의 실행(run)에서 시도할 최대 횟수. 최소 1.
    pub max_attempts: u32,
    /// 기능 id (코어 플러그인 id 또는 chat/game/settings)
    pub feature: Option<String>,
    pub run_by: RunBy,
    /// 화면 표시용 한 줄
    pub label: Option<String>,
}

impl CreateTaskRequest {
    pub fn new(task_type: TaskType, owner: TaskOwner, payload: Value) -> Self {
        Self {
            task_type,
            owner,
            owner_ref: None,
            payload,
            max_attempts: 1,
            feature: None,
            run_by: RunBy::User,
            label: None,
        }
    }

    /// 즉시 실행(run_inline) 작업: 기능 id·표시 이름·실행 주체만 정한다 (payload 없음)
    pub fn inline(task_type: TaskType, owner: TaskOwner, feature: &str, label: impl Into<String>, auto: bool) -> Self {
        Self::new(task_type, owner, Value::Null)
            .feature(feature)
            .label(label)
            .run_by(if auto { RunBy::Auto } else { RunBy::User })
    }

    pub fn feature(mut self, feature: impl Into<String>) -> Self {
        self.feature = Some(feature.into());
        self
    }

    pub fn run_by(mut self, run_by: RunBy) -> Self {
        self.run_by = run_by;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn owner_ref(mut self, owner_ref: impl Into<String>) -> Self {
        self.owner_ref = Some(owner_ref.into());
        self
    }

    pub fn max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts.max(1);
        self
    }
}

/// Executor가 성공했을 때 돌려주는 값. DB(result_json)에 그대로 저장된다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TaskOutput {
    /// 생성된 텍스트 원문 (게임 대화 등 호출자가 직접 후처리하는 경우)
    Text {
        content: String,
    },
    /// DB에 저장된 메시지
    #[serde(rename_all = "camelCase")]
    Message {
        message_id: String,
        content: String,
    },
    /// DB에 저장된 메시지 여러 개 (저장 순서대로)
    #[serde(rename_all = "camelCase")]
    Messages {
        message_ids: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    Image {
        asset_id: String,
    },
    #[serde(rename_all = "camelCase")]
    Sync {
        synced_count: u64,
    },
    Memory {
        summary: String,
    },
    Empty,
}

/// 실패 정보. DB와 프론트 이벤트에 같은 형태로 나간다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaskErrorInfo {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

/// Executor 실행 결과의 오류.
#[derive(Error, Debug, Clone)]
pub enum TaskError {
    #[error("지원하지 않는 작업 타입: {0}")]
    UnsupportedTaskType(TaskType),

    #[error("작업 인자가 올바르지 않습니다: {0}")]
    InvalidPayload(String),

    /// 취소 토큰을 보고 Executor가 스스로 멈췄을 때
    #[error("작업이 취소되었습니다")]
    Cancelled,

    #[error("{message}")]
    Failed { code: String, message: String, retryable: bool },
}

impl TaskError {
    pub fn failed(code: ErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self::Failed {
            code: error_code_str(code),
            message: message.into(),
            retryable,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Failed { retryable: true, .. })
    }

    pub fn to_info(&self) -> TaskErrorInfo {
        match self {
            Self::UnsupportedTaskType(_) => TaskErrorInfo {
                code: error_code_str(ErrorCode::TaskExecutorMissing),
                message: self.to_string(),
                retryable: false,
            },
            Self::InvalidPayload(_) => TaskErrorInfo {
                code: error_code_str(ErrorCode::InvalidInput),
                message: self.to_string(),
                retryable: false,
            },
            Self::Cancelled => TaskErrorInfo {
                code: error_code_str(ErrorCode::TaskCancelled),
                message: self.to_string(),
                retryable: false,
            },
            Self::Failed { code, message, retryable } => TaskErrorInfo {
                code: code.clone(),
                message: message.clone(),
                retryable: *retryable,
            },
        }
    }
}

impl From<AppError> for TaskError {
    fn from(e: AppError) -> Self {
        let res = e.to_response();
        if matches!(res.code, ErrorCode::TaskCancelled | ErrorCode::AiRequestCancelled) {
            return Self::Cancelled;
        }
        Self::Failed {
            code: error_code_str(res.code),
            message: res.message,
            retryable: res.retryable,
        }
    }
}

/// 작업 결과를 기다린 커맨드가 프론트에 원래 오류 코드로 돌려줄 때
impl From<TaskErrorInfo> for AppError {
    fn from(info: TaskErrorInfo) -> Self {
        let code: ErrorCode = serde_json::from_value(serde_json::Value::String(info.code)).unwrap_or(ErrorCode::InternalError);
        let message = info.message;
        match code {
            ErrorCode::ResourceNotFound => AppError::NotFound { code, message },
            ErrorCode::InvalidInput
            | ErrorCode::TaskInvalidTransition
            | ErrorCode::TaskExecutorMissing
            | ErrorCode::TaskCancelled
            | ErrorCode::TaskInterrupted => AppError::Validation { code, message, details: None },
            ErrorCode::DatabaseError | ErrorCode::DatabaseLocked => AppError::Database { code, message },
            _ => AppError::AiProvider {
                code,
                message,
                retryable: info.retryable,
                details: None,
            },
        }
    }
}

impl From<serde_json::Error> for TaskError {
    fn from(e: serde_json::Error) -> Self {
        Self::InvalidPayload(e.to_string())
    }
}

/// ErrorCode → "SCREAMING_SNAKE_CASE" 문자열 (프론트 AppErrorResponse.code와 같은 표기)
pub(crate) fn error_code_str(code: ErrorCode) -> String {
    serde_json::to_value(code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "INTERNAL_ERROR".to_string())
}

/// 취소 사유. 사용자 취소는 Cancelled로, 종료에 의한 취소는 Interrupted로 기록된다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReason {
    UserCancelled,
    ServiceShutdown,
    AppShutdown,
}

impl CancelReason {
    /// 이 사유로 멈춘 작업이 도달할 상태
    pub fn final_status(&self) -> TaskStatus {
        match self {
            Self::UserCancelled => TaskStatus::Cancelled,
            Self::ServiceShutdown | Self::AppShutdown => TaskStatus::Interrupted,
        }
    }
}

// =======================================
// 프론트 이벤트 (`task-event`)
// =======================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum TaskEvent {
    #[serde(rename_all = "camelCase")]
    Queued {
        task_id: String,
        task_type: TaskType,
        owner: TaskOwner,
        owner_ref: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Started { task_id: String, attempt_no: u32 },
    #[serde(rename_all = "camelCase")]
    Progress {
        task_id: String,
        current: u64,
        total: Option<u64>,
        message: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    TextDelta { task_id: String, delta: String },
    #[serde(rename_all = "camelCase")]
    ToolCallStarted { task_id: String, tool_call_id: String, tool_name: String },
    #[serde(rename_all = "camelCase")]
    ToolCallCompleted { task_id: String, tool_call_id: String },
    /// 재시도 가능한 오류로 다음 attempt를 준비 중
    #[serde(rename_all = "camelCase")]
    Retrying { task_id: String, attempt_no: u32, error: TaskErrorInfo },
    #[serde(rename_all = "camelCase")]
    Completed { task_id: String, output: TaskOutput },
    #[serde(rename_all = "camelCase")]
    Failed { task_id: String, error: TaskErrorInfo },
    #[serde(rename_all = "camelCase")]
    Cancelled { task_id: String },
    #[serde(rename_all = "camelCase")]
    Interrupted { task_id: String },
}

impl TaskEvent {
    pub fn task_id(&self) -> &str {
        match self {
            Self::Queued { task_id, .. }
            | Self::Started { task_id, .. }
            | Self::Progress { task_id, .. }
            | Self::TextDelta { task_id, .. }
            | Self::ToolCallStarted { task_id, .. }
            | Self::ToolCallCompleted { task_id, .. }
            | Self::Retrying { task_id, .. }
            | Self::Completed { task_id, .. }
            | Self::Failed { task_id, .. }
            | Self::Cancelled { task_id }
            | Self::Interrupted { task_id } => task_id,
        }
    }
}

// =======================================
// 조회용 모델
// =======================================

/// 작업 스냅샷 (읽기 전용 사본). 값을 바꿔도 DB에는 반영되지 않는다.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
    pub id: String,
    pub task_type: TaskType,
    pub feature: Option<String>,
    pub run_by: RunBy,
    pub label: Option<String>,
    pub owner: TaskOwner,
    pub owner_ref: Option<String>,
    pub status: TaskStatus,
    pub payload: Value,
    pub result: Option<TaskOutput>,
    pub error: Option<TaskErrorInfo>,
    pub attempt_count: u32,
    pub max_attempts: u32,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}

impl TaskInfo {
    /// payload를 Executor 전용 타입으로 해석
    pub fn payload_as<T: serde::de::DeserializeOwned>(&self) -> Result<T, TaskError> {
        serde_json::from_value(self.payload.clone()).map_err(TaskError::from)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AttemptInfo {
    pub id: String,
    pub task_id: String,
    pub attempt_no: u32,
    pub status: AttemptStatus,
    pub error: Option<TaskErrorInfo>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

// ---------------------------------------
// DB row
// ---------------------------------------

#[derive(FromRow)]
pub(crate) struct TaskInfoRow {
    pub id: String,
    pub task_type: String,
    pub feature: Option<String>,
    pub run_by: String,
    pub label: Option<String>,
    pub owner: String,
    pub owner_ref: Option<String>,
    pub status: String,
    pub payload_json: Option<String>,
    pub result_json: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub attempt_count: i64,
    pub max_attempts: i64,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}

impl TryFrom<TaskInfoRow> for TaskInfo {
    type Error = AppError;

    fn try_from(r: TaskInfoRow) -> Result<Self, Self::Error> {
        let task_type = TaskType::from_str(&r.task_type).map_err(|_| AppError::database(format!("알 수 없는 task_type: {}", r.task_type)))?;
        let status = TaskStatus::from_str(&r.status).map_err(|_| AppError::database(format!("알 수 없는 task status: {}", r.status)))?;
        let owner = TaskOwner::from_str(&r.owner).unwrap_or(TaskOwner::System);
        let error = r.error_message.map(|message| TaskErrorInfo {
            code: r.error_code.unwrap_or_else(|| error_code_str(ErrorCode::InternalError)),
            message,
            retryable: false,
        });

        Ok(Self {
            id: r.id,
            task_type,
            feature: r.feature,
            run_by: RunBy::from_str(&r.run_by).unwrap_or_default(),
            label: r.label,
            owner,
            owner_ref: r.owner_ref,
            status,
            payload: r.payload_json.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null),
            result: r.result_json.and_then(|s| serde_json::from_str(&s).ok()),
            error,
            attempt_count: r.attempt_count.max(0) as u32,
            max_attempts: r.max_attempts.max(1) as u32,
            created_at: r.created_at,
            started_at: r.started_at,
            updated_at: r.updated_at,
            finished_at: r.finished_at,
        })
    }
}

#[derive(FromRow)]
pub(crate) struct AttemptRow {
    pub id: String,
    pub task_id: String,
    pub attempt_no: i64,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

impl TryFrom<AttemptRow> for AttemptInfo {
    type Error = AppError;

    fn try_from(r: AttemptRow) -> Result<Self, Self::Error> {
        let status = AttemptStatus::from_str(&r.status).map_err(|_| AppError::database(format!("알 수 없는 attempt status: {}", r.status)))?;
        Ok(Self {
            id: r.id,
            task_id: r.task_id,
            attempt_no: r.attempt_no.max(0) as u32,
            status,
            error: r.error_message.map(|message| TaskErrorInfo {
                code: r.error_code.unwrap_or_else(|| error_code_str(ErrorCode::InternalError)),
                message,
                retryable: false,
            }),
            started_at: r.started_at,
            finished_at: r.finished_at,
        })
    }
}
