use std::sync::Arc;
use tauri::State;
use crate::{error::AppError, task::TaskManager};

#[tauri::command]
async fn resume_task(manager: State<'_, Arc<TaskManager>>, task_id: String) -> Result<(), AppError> {
    manager.resume_task(&task_id).await
}
