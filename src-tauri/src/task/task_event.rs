
use tauri::{AppHandle, Emitter};
use crate::{error::AppError, task::{task_models::{TaskContext, TaskErrorInfo, TaskEvent}, task_repository::TaskRepository}};

pub struct TaskEventEmitter {
    app_handle: AppHandle,
}

impl TaskEventEmitter {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub async fn emit(&self, event: TaskEvent) -> Result<(), AppError> {
        todo!()
    }

    pub async fn emit_persistent(&self, event: TaskEvent, context: &TaskContext) -> Result<i64, AppError> {
        todo!()
    }

    pub fn emit_runtime(&self, event: TaskEvent) -> Result<(), AppError> {
        self.app_handle.emit("task-event", event).map_err(AppError::from)
    }
}
