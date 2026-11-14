//! 대화 화면 커맨드: 보내기/다시 생성/멈추기, 폴더, 대화 관리(이름·고정·이동·복제·분기·설정)
use tauri::{ipc::Channel, AppHandle, State};

use crate::{
    db::{chat_ui, conversation},
    error::{AppError, ErrorCode, Result},
    models::{
        ai_service::{PromptMessage, StreamPayload},
        conversation::{ChatTurn, Conversation, ConversationFolder, ConversationOptions},
        settings::AppSetting,
    },
    service::chat::chat_manager::{cancel_turn, ChatManager, TurnKind},
    state::AppState,
    sys::settings::SettingsState,
};

/// ChatManager(anyhow) 오류에서 원래 AppError(코드·재시도 여부)를 되살린다.
pub(crate) fn into_app_error(e: anyhow::Error) -> AppError {
    e.downcast::<AppError>()
        .unwrap_or_else(|e| AppError::ai_provider(ErrorCode::AiProviderError, e.to_string(), false))
}

fn found(ok: bool, what: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(AppError::not_found(format!("{what}을(를) 찾을 수 없음")))
    }
}

fn required(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::validation("이름이 비었음", None));
    }
    Ok(name.to_string())
}

// ---------- 보내기 ----------

/// 입력을 저장하고 응답을 만든다. `text`가 비었으면 이어서 응답만 (이어쓰기).
/// 응답 조각은 `on_event`로 흐르고, 저장된 메시지는 반환값으로 온다.
#[tauri::command]
#[specta::specta]
pub async fn chat_send(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: State<'_, SettingsState<AppSetting>>,
    conversation_id: String,
    text: String,
    on_event: Channel<StreamPayload>,
) -> Result<ChatTurn> {
    ChatManager
        .turn(&app, &state.db, &settings, &conversation_id, TurnKind::Send(text), Some(on_event))
        .await
        .map_err(into_app_error)
}

/// 마지막 AI 메시지 다시 만들기. `swipe`면 스와이프로 쌓고, 아니면 스와이프를 버리고 바꾼다.
#[tauri::command]
#[specta::specta]
pub async fn chat_regenerate(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: State<'_, SettingsState<AppSetting>>,
    conversation_id: String,
    swipe: bool,
    on_event: Channel<StreamPayload>,
) -> Result<ChatTurn> {
    ChatManager
        .turn(&app, &state.db, &settings, &conversation_id, TurnKind::Regenerate { swipe }, Some(on_event))
        .await
        .map_err(into_app_error)
}

/// 진행 중인 응답 멈추기 (받은 데까지는 저장됨). 멈출 게 있었으면 true
#[tauri::command]
#[specta::specta]
pub fn chat_cancel(conversation_id: String) -> bool {
    cancel_turn(&conversation_id)
}

/// 이 AI 메시지를 만들 때 실제로 보낸 프롬프트 (없으면 null: 가져온 메시지, 취소된 응답 등)
#[tauri::command]
#[specta::specta]
pub async fn get_message_prompt(state: State<'_, AppState>, message_id: String) -> Result<Option<PromptMessage>> {
    let v = chat_ui::get_prompt(&state.db, &message_id).await?;
    Ok(v.and_then(|v| serde_json::from_value(v).ok()))
}

// ---------- 폴더 ----------

#[tauri::command]
#[specta::specta]
pub async fn list_conversation_folders(state: State<'_, AppState>, character_id: String) -> Result<Vec<ConversationFolder>> {
    Ok(chat_ui::list_folders(&state.db, &character_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_conversation_folder(state: State<'_, AppState>, character_id: String, name: String) -> Result<ConversationFolder> {
    let name = required(&name)?;
    Ok(chat_ui::create_folder(&state.db, &character_id, &name).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn rename_conversation_folder(state: State<'_, AppState>, id: String, name: String) -> Result<()> {
    let name = required(&name)?;
    found(chat_ui::rename_folder(&state.db, &id, &name).await?, "폴더")
}

/// 폴더만 지운다. 안의 대화는 폴더 밖으로 나온다.
#[tauri::command]
#[specta::specta]
pub async fn delete_conversation_folder(state: State<'_, AppState>, id: String) -> Result<()> {
    found(chat_ui::delete_folder(&state.db, &id).await?, "폴더")
}

// ---------- 대화 관리 ----------

#[tauri::command]
#[specta::specta]
pub async fn rename_conversation(state: State<'_, AppState>, id: String, title: String) -> Result<()> {
    let title = required(&title)?;
    found(chat_ui::rename_conversation(&state.db, &id, &title).await?, "대화")
}

#[tauri::command]
#[specta::specta]
pub async fn set_conversation_pinned(state: State<'_, AppState>, id: String, pinned: bool) -> Result<()> {
    found(chat_ui::set_pinned(&state.db, &id, pinned).await?, "대화")
}

/// 폴더로 옮기기. `folder_id`가 null이면 폴더 밖으로
#[tauri::command]
#[specta::specta]
pub async fn move_conversation(state: State<'_, AppState>, id: String, folder_id: Option<String>) -> Result<()> {
    found(chat_ui::move_to_folder(&state.db, &id, folder_id.as_deref()).await?, "대화")
}

/// 대화별 페르소나·프리셋·API (프리셋/API가 null이면 앱 설정의 선택값을 씀)
#[tauri::command]
#[specta::specta]
pub async fn set_conversation_options(state: State<'_, AppState>, id: String, options: ConversationOptions) -> Result<Conversation> {
    found(chat_ui::set_options(&state.db, &id, &options).await?, "대화")?;
    Ok(conversation::get_conversation(&state.db, &id).await?)
}

/// 대화 통째로 복제
#[tauri::command]
#[specta::specta]
pub async fn duplicate_conversation(state: State<'_, AppState>, id: String) -> Result<Conversation> {
    let src = conversation::get_conversation(&state.db, &id).await?;
    let new_id = chat_ui::copy_conversation(&state.db, &id, &format!("{} (복제)", src.title), None).await?;
    Ok(conversation::get_conversation(&state.db, &new_id).await?)
}

/// 분기 나누기: 이 메시지까지 복사한 새 대화
#[tauri::command]
#[specta::specta]
pub async fn branch_conversation(state: State<'_, AppState>, id: String, message_id: String) -> Result<Conversation> {
    let src = conversation::get_conversation(&state.db, &id).await?;
    let msg = chat_ui::get_message(&state.db, &message_id).await?;
    let title = format!("{} · #{} 분기", src.title, msg.seq);
    let new_id = chat_ui::copy_conversation(&state.db, &id, &title, Some(&message_id)).await?;
    Ok(conversation::get_conversation(&state.db, &new_id).await?)
}
