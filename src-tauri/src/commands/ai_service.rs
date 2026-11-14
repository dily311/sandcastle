use crate::db::ai_service;
use crate::models::ai_service::{ApiProfiles, ApiProfilesInput};
use crate::service::ai::AiService;
use std::sync::Arc;
use crate::service::secret_store::{api_key_name, SecretStore};
use crate::{
    error::{AppError, Result},
    state::AppState,
};
use tauri::{AppHandle, State};

// =======================================
// API Key
// =======================================

#[tauri::command]
#[specta::specta]
pub fn set_api_key(profile_id: String, api_key: String) -> Result<()> {
    SecretStore::set(&api_key_name(&profile_id), &api_key)
        .map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn delete_api_key(profile_id: String) -> Result<()> {
    SecretStore::delete(&api_key_name(&profile_id)).map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn has_api_key(profile_id: String) -> Result<bool> {
    Ok(SecretStore::exists(&api_key_name(&profile_id)))
}

// =======================================
// API Profile
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_all_api_profile(state: State<'_, AppState>) -> Result<Vec<ApiProfiles>> {
    let get_all_api_profile = ai_service::get_all_api_profile(&state.db).await?;
    let result = get_all_api_profile;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn get_api_profile(state: State<'_, AppState>, id: String) -> Result<ApiProfiles> {
    let result = ai_service::get_api_profile(&state.db, &id).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
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
#[specta::specta]
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
#[specta::specta]
pub async fn delete_api_profile(state: State<'_, AppState>, id: String) -> Result<()> {
    delete_api_key(id.clone())?;
    ai_service::delete_api_profile(&state.db, &id).await?;
    Ok(())
}

// =======================================
// AI 연결
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_models(ai: State<'_, Arc<AiService>>, profile_id: String) -> Result<Vec<String>> {
    ai.list_models(&profile_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn test_message(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<()> {
    Ok(())
}
