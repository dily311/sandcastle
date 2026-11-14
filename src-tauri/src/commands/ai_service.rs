use crate::db::ai_service;
use crate::error::ErrorCode;
use crate::models::ai_service::{ApiProfiles, ApiProfilesInput, ChatOptions, StreamPayload};
use crate::models::settings::AppSetting;
use crate::service::secret_store::{api_key_name, SecretStore};
use crate::sys::settings::SettingsState;
use crate::{
    error::{AppError, Result},
    service::chat::chat_manager::ChatManager,
    state::AppState,
};
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

// =======================================
// API Key
// =======================================

#[tauri::command]
pub fn set_api_key(profile_id: String, api_key: String) -> Result<()> {
    SecretStore::set(&api_key_name(&profile_id), &api_key)
        .map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
pub fn delete_api_key(profile_id: String) -> Result<()> {
    SecretStore::delete(&api_key_name(&profile_id)).map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
pub fn has_api_key(profile_id: String) -> Result<bool> {
    Ok(SecretStore::exists(&api_key_name(&profile_id)))
}

// =======================================
// API Profile
// =======================================
#[tauri::command]
pub async fn get_all_api_profile(state: State<'_, AppState>) -> Result<Vec<ApiProfiles>> {
    let get_all_api_profile = ai_service::get_all_api_profile(&state.db).await?;
    let result = get_all_api_profile;
    Ok(result)
}

#[tauri::command]
pub async fn get_api_profile(state: State<'_, AppState>, id: String) -> Result<ApiProfiles> {
    let result = ai_service::get_api_profile(&state.db, &id).await?;
    Ok(result)
}

#[tauri::command]
pub async fn create_api_profile(
    state: State<'_, AppState>,
    input: ApiProfilesInput,
    api_key: String,
) -> Result<ApiProfiles> {
    let api_profile = ai_service::create_api_profile(&state.db, input).await?;

    // TODO: 연결 테스트해서 응답이 200으로 나왔을때만 has_key: true가 되게 해야 함
    if !api_key.is_empty() {
        set_api_key(api_profile.id.clone(), api_key)?;
    }

    Ok(api_profile)
}

#[tauri::command]
pub async fn update_api_profile(
    state: State<'_, AppState>,
    id: String,
    input: ApiProfilesInput,
    api_key: String,
) -> Result<()> {
    ai_service::update_api_profile(&state.db, &id, input).await?;

    // TODO: 연결 테스트해서 응답이 200으로 나왔을때만 has_key: true가 되게 해야 함
    if !api_key.is_empty() {
        set_api_key(id.clone(), api_key)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_api_profile(state: State<'_, AppState>, id: String) -> Result<()> {
    delete_api_key(id.clone())?;
    ai_service::delete_api_profile(&state.db, &id).await?;
    Ok(())
}

// =======================================
// AI 연결
// =======================================
#[tauri::command]
pub async fn get_models(state: State<'_, AppState>, profile_id: String) -> Result<Vec<String>> {
    let api_profile = ai_service::get_api_profile(&state.db, &profile_id).await?;
    let result = ChatManager
        .get_models(api_profile, state.http_client.clone())
        .await
        .map_err(|e| AppError::ai_provider(ErrorCode::AiProviderError, e.to_string(), false))?;
    Ok(result)
}

#[tauri::command]
pub async fn test_message(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<()> {
    Ok(())
}

/** 사용자 메시지 전송 즉시 반환하고, 실제 응답은 이벤트로 스트리밍됨.
 * 내부적으로
 * 1. 유저 메시지 DB 저장
 * 2. 최근 N턴 컨텍스트 조립
 * 3. systemPrompt + 컨텍스트로 스트리밍 이벤트 호출
 * 4. 토큰마다 dialogue:token 이벤트로 emit
 * 5. 완료시 dialogue:done 이벤트 emit + 어시스턴트 메시지 DB 저장
 */
#[tauri::command]
pub async fn send_messages(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: State<'_, SettingsState<AppSetting>>,
    conversation_id: String,
    message: String,
) -> Result<()> {
    ChatManager
        .chat(app, &state, &settings, conversation_id, message, None)
        .await
        .map_err(|e| AppError::ai_provider(crate::error::ErrorCode::AiProviderAuthFailed, e.to_string(), false))?;
    Ok(())
}

#[tauri::command]
pub async fn stream_message(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: State<'_, SettingsState<AppSetting>>,
    conversation_id: String,
    message: String,
    on_event: Channel<StreamPayload>,
) -> Result<()> {
    ChatManager
        .chat(app, &state, &settings, conversation_id, message, Some(on_event))
        .await
        .map_err(|e| AppError::ai_provider(crate::error::ErrorCode::AiProviderAuthFailed, e.to_string(), false))?;
    Ok(())
}
