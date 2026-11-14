//! 작업 이벤트를 프론트엔드로 내보내는 출구.
//!
//! TaskManager는 `TaskEventSink` 트레이트만 알고, 실제 앱에서는 Tauri 이벤트(`task-event`)로,
//! 테스트에서는 메모리 수집기로 바꿔 끼운다.
use tauri::{AppHandle, Emitter};

use crate::task::task_models::TaskEvent;

/// 프론트엔드 이벤트 이름
pub const TASK_EVENT: &str = "task-event";

pub trait TaskEventSink: Send + Sync + 'static {
    fn emit(&self, event: &TaskEvent);
}

/// Tauri 앱 전체에 `task-event`를 브로드캐스트
pub struct TaskEventEmitter {
    app_handle: AppHandle,
}

impl TaskEventEmitter {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl TaskEventSink for TaskEventEmitter {
    fn emit(&self, event: &TaskEvent) {
        // 이벤트 전송 실패(창이 닫힘 등)는 작업 진행에 영향을 주지 않는다.
        if let Err(e) = self.app_handle.emit(TASK_EVENT, event) {
            eprintln!("[task] 이벤트 전송 실패 ({}): {e}", event.task_id());
        }
    }
}

/// 이벤트를 버리는 Sink (헤드리스 실행/테스트용)
pub struct NoopEventSink;

impl TaskEventSink for NoopEventSink {
    fn emit(&self, _event: &TaskEvent) {}
}
