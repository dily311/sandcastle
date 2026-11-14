//! 폰 화면 커맨드: 페르소나별 연락처. 폰 대화는 일반 대화(mode = "phone")와 같은 커맨드를 쓴다.
use tauri::State;

use crate::{
    db::phone,
    error::{AppError, Result},
    models::conversation::PhoneContact,
    state::AppState,
};

#[tauri::command]
#[specta::specta]
pub async fn list_phone_contacts(state: State<'_, AppState>, persona_id: String) -> Result<Vec<PhoneContact>> {
    Ok(phone::list_contacts(&state.db, &persona_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn add_phone_contact(state: State<'_, AppState>, persona_id: String, character_id: String) -> Result<PhoneContact> {
    Ok(phone::add_contact(&state.db, &persona_id, &character_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_phone_contact(state: State<'_, AppState>, persona_id: String, character_id: String) -> Result<()> {
    if !phone::remove_contact(&state.db, &persona_id, &character_id).await? {
        return Err(AppError::not_found("연락처를 찾을 수 없습니다"));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn set_phone_contact_status(
    state: State<'_, AppState>,
    persona_id: String,
    character_id: String,
    status: String,
) -> Result<()> {
    if !phone::set_status(&state.db, &persona_id, &character_id, status.trim()).await? {
        return Err(AppError::not_found("연락처를 찾을 수 없습니다"));
    }
    Ok(())
}
