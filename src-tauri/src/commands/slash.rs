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

/// /gen: 이 대화의 맥락 끝에 지시를 붙여 AI 글을 받는다 (저장하지 않음).
/// raw면 /genraw: 맥락 없이 지시만 (system이 있으면 그 앞에 시스템 글)
#[allow(clippy::too_many_arguments)]
#[tauri::command]
#[specta::specta]
pub async fn slash_generate(
    app: AppHandle,
    state: State<'_, AppState>,
    tasks: State<'_, Arc<TaskManager>>,
    settings: State<'_, SettingsState<AppSetting>>,
    conversation_id: String,
    prompt: String,
    raw: bool,
    system: Option<String>,
) -> Result<String> {
    plugins::require(&app, plugins::id::SLASH)?;
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err(AppError::validation("/gen: 지시가 비어 있음", None));
    }
    let label = if raw { "/genraw" } else { "/gen" };
    let job = CreateTaskRequest::inline(TaskType::SlashGen, TaskOwner::Chat, plugins::id::SLASH, label, false).owner_ref(&conversation_id);
    let raw = raw.then_some(system);
    tasks
        .run_inline(job, |cancel| async move {
            ChatManager
                .quiet_gen(&app, &state.db, &settings, &conversation_id, &prompt, raw, &cancel)
                .await
                .map_err(into_app_error)
        })
        .await
}
