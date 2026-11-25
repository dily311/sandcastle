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

/// /gen·/genraw 결과. truncated: 응답 길이 한도에서 잘림 (화면이 경고를 띄움)
#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SlashGenResult {
    pub text: String,
    pub truncated: bool,
    /// 보낸 프롬프트 (PromptMessage). 이 글이 들어간 메시지에 save_message_prompt로 남긴다 (5-E-20-6-10)
    pub prompt: serde_json::Value,
}

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
) -> Result<SlashGenResult> {
    plugins::require(&app, plugins::id::SLASH)?;
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err(AppError::validation("/gen: 지시가 비어 있음", None));
    }
    let label = if raw { "/genraw" } else { "/gen" };
    let job = CreateTaskRequest::inline(TaskType::SlashGen, TaskOwner::Chat, plugins::id::SLASH, label, false).owner_ref(&conversation_id);
    let raw = raw.then_some(system);
    let notify_app = app.clone();
    let out = tasks
        .run_inline(job, |cancel| async move {
            ChatManager
                .quiet_gen(&app, &state.db, &settings, &conversation_id, &prompt, raw, &cancel)
                .await
                .map(|(text, truncated, prompt)| SlashGenResult { text, truncated, prompt })
                .map_err(into_app_error)
        })
        .await?;
    crate::service::notify::task_done(&notify_app, label, "생성이 끝남");
    Ok(out)
}

/// 생성 중인 /gen·/genraw 멈추기 (5-E-20-6-8, 대필 멈추기와 같은 모양). 멈출 게 있었으면 true
#[tauri::command]
#[specta::specta]
pub async fn slash_cancel(tasks: State<'_, Arc<TaskManager>>, conversation_id: String) -> Result<bool> {
    let running: Vec<_> = tasks
        .list_by_owner_ref(&conversation_id)
        .await?
        .into_iter()
        .filter(|t| t.task_type == TaskType::SlashGen && matches!(t.status, crate::task::TaskStatus::Pending | crate::task::TaskStatus::Running))
        .collect();
    for t in &running {
        tasks.cancel(&t.id).await?;
    }
    Ok(!running.is_empty())
}

/// 메시지에 '보낸 프롬프트'를 남긴다 — 슬래시 스크립트가 /gen 결과로 만든 메시지용 (5-E-20-6-10).
/// 형식이 PromptMessage가 아니면 거절 (보기 화면이 읽을 수 있는 것만)
#[tauri::command]
#[specta::specta]
pub async fn save_message_prompt(state: State<'_, AppState>, message_id: String, prompt: serde_json::Value) -> Result<()> {
    if serde_json::from_value::<crate::models::ai_service::PromptMessage>(prompt.clone()).is_err() {
        return Err(AppError::validation("보낸 프롬프트 형식이 아님", None));
    }
    crate::db::chat_ui::save_prompt(&state.db, &message_id, &prompt).await?;
    Ok(())
}
