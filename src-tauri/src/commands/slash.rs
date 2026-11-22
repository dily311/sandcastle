//! 슬래시 커맨드 (5-E-20-5, ADR-025). 스크립트 엔진·대부분의 커맨드는 화면에 있고,
//! 여기는 AI를 부르는 /gen만 둔다. 사용자가 입력칸에서 보내거나 빠른 답장을 눌러 실행한 것이라 `run_by = user`.
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{
    commands::chat::into_app_error,
    error::{AppError, Result},
    models::settings::AppSetting,
    service::chat::chat_manager::ChatManager,
    state::AppState,
    sys::{plugins, settings::SettingsState},
    task::{CreateTaskRequest, TaskManager, TaskOwner, TaskType},
};

/// /gen: 이 대화의 맥락 끝에 지시를 붙여 AI 글을 받는다 (저장하지 않음)
#[tauri::command]
#[specta::specta]
pub async fn slash_generate(
    app: AppHandle,
    state: State<'_, AppState>,
    tasks: State<'_, Arc<TaskManager>>,
    settings: State<'_, SettingsState<AppSetting>>,
    conversation_id: String,
    prompt: String,
) -> Result<String> {
    plugins::require(&app, plugins::id::SLASH)?;
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err(AppError::validation("/gen: 지시가 비어 있음", None));
    }
    let job = CreateTaskRequest::inline(TaskType::SlashGen, TaskOwner::Chat, plugins::id::SLASH, "/gen", false).owner_ref(&conversation_id);
    tasks
        .run_inline(job, |cancel| async move {
            ChatManager
                .quiet_gen(&app, &state.db, &settings, &conversation_id, &prompt, &cancel)
                .await
                .map_err(into_app_error)
        })
        .await
}
