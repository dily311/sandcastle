//! 폰 화면 커맨드: 주인별 연락처, 엿보기(AI가 두 사람 대화를 모두 씀).
//! 폰 대화 자체는 일반 대화(mode = "phone")와 같은 커맨드를 쓴다.
use std::sync::Arc;

use tauri::{ipc::Channel, AppHandle, State};

use crate::{
    commands::chat::into_app_error,
    db::phone::{self, Who},
    error::{AppError, Result},
    models::{
        ai_service::StreamPayload,
        conversation::{Message, PhoneContact},
        settings::AppSetting,
    },
    service::chat::chat_manager::ChatManager,
    state::AppState,
    sys::settings::SettingsState,
    task::{CreateTaskRequest, TaskManager, TaskOwner, TaskType},
};

fn who<'a>(kind: &'a str, id: &'a str) -> Result<Who<'a>> {
    if !phone::valid_kind(kind) {
        return Err(AppError::validation(format!("종류는 persona 또는 character여야 합니다: {kind}"), None));
    }
    Ok(Who { kind, id })
}

#[tauri::command]
#[specta::specta]
pub async fn list_phone_contacts(state: State<'_, AppState>, owner_kind: String, owner_id: String) -> Result<Vec<PhoneContact>> {
    Ok(phone::list_contacts(&state.db, who(&owner_kind, &owner_id)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn add_phone_contact(
    state: State<'_, AppState>,
    owner_kind: String,
    owner_id: String,
    contact_kind: String,
    contact_id: String,
) -> Result<PhoneContact> {
    if owner_kind == contact_kind && owner_id == contact_id {
        return Err(AppError::validation("자기 자신은 연락처에 넣을 수 없습니다", None));
    }
    Ok(phone::add_contact(&state.db, who(&owner_kind, &owner_id)?, who(&contact_kind, &contact_id)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_phone_contact(state: State<'_, AppState>, owner_kind: String, owner_id: String, contact_kind: String, contact_id: String) -> Result<()> {
    if !phone::remove_contact(&state.db, who(&owner_kind, &owner_id)?, who(&contact_kind, &contact_id)?).await? {
        return Err(AppError::not_found("연락처를 찾을 수 없습니다"));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn set_phone_contact_status(
    state: State<'_, AppState>,
    owner_kind: String,
    owner_id: String,
    contact_kind: String,
    contact_id: String,
    status: String,
) -> Result<()> {
    if !phone::set_status(&state.db, who(&owner_kind, &owner_id)?, who(&contact_kind, &contact_id)?, status.trim()).await? {
        return Err(AppError::not_found("연락처를 찾을 수 없습니다"));
    }
    Ok(())
}

/// 엿보기: 캐릭터 폰의 대화를 AI가 두 사람 몫 모두 이어서 씀. 새로 저장된 메시지들을 돌려준다
#[tauri::command]
#[specta::specta]
pub async fn phone_peek(app: AppHandle, conversation_id: String, on_event: Channel<StreamPayload>) -> Result<Vec<Message>> {
    crate::service::chat::chat_task::run(&app, &conversation_id, crate::service::chat::chat_task::ChatJob::Peek, on_event).await
}

/// 폰 캐릭터 개인 페이지(SNS 프로필·상태 메시지·게시물)를 대화 맥락을 바탕으로 AI가 생성한다
#[tauri::command]
#[specta::specta]
pub async fn phone_generate_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    tasks: State<'_, Arc<TaskManager>>,
    settings: State<'_, SettingsState<AppSetting>>,
    conversation_id: String,
    prompt: String,
) -> Result<String> {
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err(AppError::validation("프로필 생성 지시가 비어 있습니다", None));
    }
    let job = CreateTaskRequest::inline(TaskType::SlashGen, TaskOwner::Chat, "phone", "폰 개인 페이지 생성", false).owner_ref(&conversation_id);
    tasks
        .run_inline(job, |cancel| async move {
            ChatManager
                .quiet_gen(&app, &state.db, &settings, &conversation_id, &prompt, None, &cancel)
                .await
                .map(|(text, _, _)| text)
                .map_err(into_app_error)
        })
        .await
}
