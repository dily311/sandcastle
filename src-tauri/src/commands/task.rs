//! 작업 관리 커맨드. 모두 TaskManager에 "요청"만 하고, 상태 변경은 TaskManager가 한다.
use std::sync::Arc;

use tauri::State;

use crate::{
    error::Result,
    task::{TaskInfo, TaskManager},
};

#[tauri::command]
#[specta::specta]
pub async fn get_task(manager: State<'_, Arc<TaskManager>>, task_id: String) -> Result<TaskInfo> {
    manager.get_task(&task_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_active_tasks(manager: State<'_, Arc<TaskManager>>) -> Result<Vec<TaskInfo>> {
    manager.list_active().await
}

/// 앱 시작 후 프론트가 호출: 재개/포기를 물어볼 중단된 작업 목록
#[tauri::command]
#[specta::specta]
pub async fn get_recoverable_tasks(manager: State<'_, Arc<TaskManager>>) -> Result<Vec<TaskInfo>> {
    manager.list_recoverable().await
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_task(manager: State<'_, Arc<TaskManager>>, task_id: String) -> Result<()> {
    manager.cancel(&task_id).await
}

/// 재개 요청. 결과는 `task-event`로 전달되므로 접수된 작업 정보만 바로 돌려준다.
#[tauri::command]
#[specta::specta]
pub async fn resume_task(manager: State<'_, Arc<TaskManager>>, task_id: String) -> Result<TaskInfo> {
    let handle = manager.inner().resume_task(&task_id).await?;
    Ok(handle.task().clone())
}

#[tauri::command]
#[specta::specta]
pub async fn dismiss_task(manager: State<'_, Arc<TaskManager>>, task_id: String) -> Result<()> {
    manager.dismiss_task(&task_id).await
}
