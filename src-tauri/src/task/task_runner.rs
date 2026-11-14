//! Executor 계약.
//!
//! ChatManager / GameRuntime / ImageManager는 자기 작업 종류에 대한 `TaskExecutor`를 구현해
//! TaskManager에 등록한다. Executor가 할 수 있는 일은 딱 세 가지다.
//!
//! 1. `TaskContext::reporter()`로 진행 이벤트(진행률, 텍스트 조각, 도구 호출) 보내기
//! 2. `TaskContext::cancel_token()`을 보고 협조적으로 멈추기
//! 3. `Result<TaskOutput, TaskError>`를 반환하기
//!
//! 상태(Running/Completed/Failed/...)는 Executor가 반환한 값을 보고 TaskManager가 결정한다.
//! TaskContext/TaskReporter에는 상태를 바꾸는 메서드가 없고, 생성자도 task 모듈 안에서만 보인다.
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tokio_util::sync::CancellationToken;

use crate::task::{
    task_event::TaskEventSink,
    task_models::{TaskError, TaskEvent, TaskInfo, TaskOutput, TaskType},
};

#[async_trait::async_trait]
pub trait TaskExecutor: Send + Sync + 'static {
    /// 이 Executor가 처리하는 작업 종류
    fn task_type(&self) -> TaskType;

    /// 작업 한 번(attempt 하나) 실행.
    /// 재시도 가능한 오류(`TaskError::Failed { retryable: true, .. }`)를 반환하면
    /// max_attempts 안에서 TaskManager가 새 attempt로 다시 부른다.
    async fn execute(&self, task: &TaskInfo, ctx: &TaskContext) -> Result<TaskOutput, TaskError>;
}

/// Executor 전용 이벤트 송신기. 진행 상황만 보낼 수 있다.
#[derive(Clone)]
pub struct TaskReporter {
    task_id: String,
    sink: Arc<dyn TaskEventSink>,
}

impl TaskReporter {
    pub(super) fn new(task_id: String, sink: Arc<dyn TaskEventSink>) -> Self {
        Self { task_id, sink }
    }

    pub fn progress(&self, current: u64, total: Option<u64>, message: Option<String>) {
        self.sink.emit(&TaskEvent::Progress {
            task_id: self.task_id.clone(),
            current,
            total,
            message,
        });
    }

    pub fn text_delta(&self, delta: impl Into<String>) {
        self.sink.emit(&TaskEvent::TextDelta {
            task_id: self.task_id.clone(),
            delta: delta.into(),
        });
    }

    pub fn tool_call_started(&self, tool_call_id: impl Into<String>, tool_name: impl Into<String>) {
        self.sink.emit(&TaskEvent::ToolCallStarted {
            task_id: self.task_id.clone(),
            tool_call_id: tool_call_id.into(),
            tool_name: tool_name.into(),
        });
    }

    pub fn tool_call_completed(&self, tool_call_id: impl Into<String>) {
        self.sink.emit(&TaskEvent::ToolCallCompleted {
            task_id: self.task_id.clone(),
            tool_call_id: tool_call_id.into(),
        });
    }
}

/// 한 attempt의 실행 문맥 (읽기 전용)
pub struct TaskContext {
    task_id: String,
    attempt_id: String,
    attempt_no: u32,
    cancel: CancellationToken,
    reporter: TaskReporter,
}

impl TaskContext {
    pub(super) fn new(
        task_id: String,
        attempt_id: String,
        attempt_no: u32,
        cancel: CancellationToken,
        reporter: TaskReporter,
    ) -> Self {
        Self {
            task_id,
            attempt_id,
            attempt_no,
            cancel,
            reporter,
        }
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }

    pub fn attempt_no(&self) -> u32 {
        self.attempt_no
    }

    /// Executor 안에서 취소를 감지할 때 사용 (clone해서 하위 호출에 넘겨도 됨)
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub fn reporter(&self) -> &TaskReporter {
        &self.reporter
    }
}

/// 작업 종류 → Executor 레지스트리
pub(super) struct ExecutorRegistry {
    executors: RwLock<HashMap<TaskType, Arc<dyn TaskExecutor>>>,
}

impl ExecutorRegistry {
    pub(super) fn new() -> Self {
        Self {
            executors: RwLock::new(HashMap::new()),
        }
    }

    pub(super) fn register(&self, executor: Arc<dyn TaskExecutor>) {
        let mut map = self.executors.write().unwrap_or_else(|e| e.into_inner());
        map.insert(executor.task_type(), executor);
    }

    pub(super) fn get(&self, task_type: TaskType) -> Option<Arc<dyn TaskExecutor>> {
        let map = self.executors.read().unwrap_or_else(|e| e.into_inner());
        map.get(&task_type).cloned()
    }
}
