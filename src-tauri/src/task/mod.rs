//! 전역 작업(Task) 관리.
//!
//! ```text
//! task/
//!     task_manager.rs     - 작업 생성·실행·취소·재개. 상태를 바꾸는 유일한 곳 (pub)
//!     task_runner.rs      - TaskExecutor 계약, Executor가 받는 읽기 전용 TaskContext (pub)
//!     task_event.rs       - 프론트엔드 이벤트 출구 (`task-event`) (pub)
//!     task_models.rs      - 공용 타입 (pub)
//!     task_repository.rs  - SQLite 저장소 (task 모듈 내부 전용)
//!     task_state.rs       - 상태 전이 규칙 (task 모듈 내부 전용)
//! ```
//!
//! ## 상태 변경 규칙
//! 작업 상태는 TaskManager만 바꾼다. 이를 컴파일러가 보장하도록:
//! - `TaskRepository`와 `task_state`는 비공개 모듈이라 밖에서 이름조차 쓸 수 없다.
//! - Executor가 받는 `TaskContext` / `TaskReporter`에는 진행 이벤트를 보내는 메서드만 있다.
//! - `TaskInfo`는 조회용 사본이라 값을 바꿔도 DB에 반영되지 않는다.
//!
//! ## 다른 Manager에서 쓰는 방법
//! ```ignore
//! // 1) 앱 시작 시 자기 작업 종류의 Executor 등록
//! task_manager.register_executor(Arc::new(ChatStreamExecutor::new(...)));
//!
//! // 2) 작업 요청 → 핸들을 받아 즉시 반환하거나, 결과를 기다림
//! let handle = task_manager
//!     .submit(CreateTaskRequest::new(TaskType::ChatStream, TaskOwner::Chat, json!({ ... }))
//!         .owner_ref(conversation_id)
//!         .max_attempts(3))
//!     .await?;
//! let output = handle.wait().await;
//!
//! // 3) 취소/재개도 TaskManager에 요청만 한다
//! task_manager.cancel(&task_id).await?;
//! ```
mod task_context;
mod task_event;
mod task_manager;
pub mod task_models;
mod task_repository;
mod task_runner;
mod task_state;

pub use task_context::{current as current_task, CurrentTask};
#[cfg(test)]
pub(crate) use task_context::scope as task_context_scope_for_test;
pub use task_event::{NoopEventSink, TaskEventEmitter, TaskEventSink, TASK_EVENT};
pub use task_manager::{TaskHandle, TaskManager, TaskResult};
pub use task_models::{
    AttemptInfo, CancelReason, CreateTaskRequest, RunBy, TaskError, TaskErrorInfo, TaskEvent, TaskInfo,
    TaskOutput, TaskOwner, TaskStatus, TaskType,
};
pub use task_runner::{TaskContext, TaskExecutor, TaskReporter};
