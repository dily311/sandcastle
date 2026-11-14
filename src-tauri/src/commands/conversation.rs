use crate::db::conversation::{self};
use crate::error::{AppError, Result};
use crate::models::conversation::*;
use crate::models::regex::{RegexPlacement, RegexTarget};
use crate::models::settings::AppSetting;
use crate::service::chat::regex_processor::RegexProcessor;
use crate::state::AppState;
use crate::sys::settings::SettingsState;
use serde_json::json;
use tauri::State;

// =======================================
// 시스템 기본 페르소나
// =======================================
#[tauri::command]
pub async fn get_conv_settings(state: State<'_, AppState>) -> Result<ConvSettings> {
    let default = conversation::get_system_default_persona(&state.db).await?;
    Ok(ConvSettings {
        default_persona_id: default.into(),
    })
}

#[tauri::command]
pub async fn set_system_default_persona(state: State<'_, AppState>, id: String) -> Result<()> {
    conversation::set_system_default_persona(&state.db, &id).await?;
    Ok(())
}

// =======================================
// Conversations
// =======================================
#[tauri::command]
pub async fn get_all_conversations(state: State<'_, AppState>) -> Result<Vec<Conversation>> {
    let rows = conversation::get_all_conversations(&state.db).await?;
    Ok(rows)
}

#[tauri::command]
pub async fn get_conversation(state: State<'_, AppState>, id: String) -> Result<Conversation> {
    let row = conversation::get_conversation(&state.db, &id).await?;
    Ok(row)
}

#[tauri::command]
pub async fn create_conversation(
    state: State<'_, AppState>,
    mut input: ConversationInput,
) -> Result<Conversation> {
    // 모드: 비우면 chat, 그 외에는 chat / phone만 허용
    input.mode = if input.mode.trim().is_empty() {
        ConversationMode::Chat.to_string()
    } else {
        input
            .mode
            .trim()
            .to_ascii_lowercase()
            .parse::<ConversationMode>()
            .map_err(|_| AppError::validation(format!("알 수 없는 대화 모드입니다: {}", input.mode), None))?
            .to_string()
    };
    // character_id가 실제로 존재하는지 확인: 없는 캐릭터로 대화가 생기는 걸 막음
    if !conversation::char_exists(&state.db, &input.character_id).await? {
        return Err(AppError::validation("존재하지 않는 캐릭터입니다", Some(json!({"data": &input.character_id}).into())));
    }

    let conversation = conversation::create_conversation(&state.db, input).await?;
    Ok(conversation)
}

#[tauri::command]
pub async fn touch_conversation(state: State<'_, AppState>, id: String) -> Result<i64> {
    let result = conversation::touch_conversation(&state.db, &id).await?;
    Ok(result)
}

#[tauri::command]
pub async fn delete_conversation(state: State<'_, AppState>, id: String) -> Result<()> {
    let result = conversation::delete_conversation(&state.db, &id).await?;
    if !result {
        return Err(AppError::not_found(format!("해당 대화 세션을 찾을 수 없습니다: {id}")));
    }
    Ok(())
}

// =======================================
// Character
// =======================================
#[tauri::command]
pub async fn get_all_characters(state: State<'_, AppState>) -> Result<Vec<Character>> {
    Ok(conversation::get_all_characters(&state.db).await?)
}

#[tauri::command]
pub async fn get_list_characters(state: State<'_, AppState>) -> Result<Vec<CharacterOption>> {
    let rows = conversation::get_list_characters(&state.db).await?;
    Ok(rows)
}

#[tauri::command]
pub async fn create_character(
    state: State<'_, AppState>,
    input: CharacterInput,
) -> Result<Character> {
    let character = conversation::create_character(&state.db, input).await?;
    Ok(character)
}

#[tauri::command]
pub async fn update_character(
    state: State<'_, AppState>,
    id: String,
    input: CharacterInput,
) -> Result<()> {
    conversation::update_character(&state.db, &id, &input).await?;
    Ok(())
}

#[tauri::command]
pub async fn toggle_favorite_character(
    state: State<'_, AppState>,
    id: String,
    is_favorite: bool,
) -> Result<()> {
    conversation::update_character_is_favorite(&state.db, &id, is_favorite).await?;
    Ok(())
}

#[tauri::command]
pub async fn delete_character(app: tauri::AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    let result = conversation::delete_character(&state.db, &id).await?;
    if !result {
        return Err(AppError::not_found(format!(
            "캐릭터를 찾을 수 없습니다: {id}"
        )));
    }
    // 이미지 행은 CASCADE로 지워지고, 파일 폴더는 여기서 정리
    if let Ok(base) = crate::commands::character_image::app_data_dir(&app) {
        crate::service::chat::chat_images::remove_character_dir(&base, &id);
    }
    Ok(())
}

// =======================================
// Persona
// =======================================
#[tauri::command]
pub async fn get_all_personas(state: State<'_, AppState>) -> Result<Vec<Persona>> {
    Ok(conversation::get_all_personas(&state.db).await?)
}

#[tauri::command]
pub async fn create_persona(state: State<'_, AppState>, input: PersonaInput) -> Result<Persona> {
    Ok(conversation::create_persona(&state.db, input).await?)
}

#[tauri::command]
pub async fn update_persona(
    state: State<'_, AppState>,
    id: String,
    input: PersonaInput,
) -> Result<()> {
    let exists = conversation::persona_exists(&state.db, &id).await?;
    if !exists {
        return Err(AppError::not_found(format!("캐릭터를 찾을 수 없습니다: {id}")));
    }

    conversation::update_persona(&state.db, &id, input).await?;
    Ok(())
}

#[tauri::command]
pub async fn delete_persona(state: State<'_, AppState>, id: String) -> Result<()> {
    // 삭제 전에 어디서 쓰이고 있는지 확인해서 경고용 정보를 돌려줄 수도 있음
    let result = conversation::delete_persona(&state.db, &id).await?;
    if !result {
        return Err(AppError::not_found(format!("해당 페르소나를 찾을 수 없습니다: {id}")));
    }
    Ok(())
}

// 삭제 전 영향 범위를 프론트에 알려주기 위한 커맨드 (선택)
#[tauri::command]
pub async fn get_persona_usage(state: State<'_, AppState>, id: String) -> Result<PersonaUsage> {
    let usage = conversation::get_persona_usage(&state.db, &id).await?;
    Ok(usage)
}

// =======================================
// Message
// =======================================

#[tauri::command]
pub async fn get_messages(
    state: State<'_, AppState>,
    conversation_id: String,
    limit: i64,
    before: Option<i64>,
) -> Result<Vec<Message>> {
    let rows = conversation::get_messages(&state.db, &conversation_id, limit, before).await?;
    Ok(rows)
}

#[tauri::command]
pub async fn append_message(
    state: State<'_, AppState>,
    settings: State<'_, SettingsState<AppSetting>>,
    mut input: MessageInput,
) -> Result<Message> {
    if input.content.is_empty() && input.role == "user" {
        return Err(AppError::validation("빈 메시지는 전송할 수 없습니다", None));
    }
    // 사용자 입력은 저장 전에 정규식(user_input / stored) 적용
    if input.role == "user" {
        let preset_id = crate::commands::regex::selected_preset(&settings);
        let regex = RegexProcessor::for_conversation(&state.db, &input.conversation_id, preset_id.as_deref()).await?;
        input.content = regex.apply(&input.content, RegexPlacement::UserInput, &[RegexTarget::Stored], Some(0));
    }
    let conversation_id = input.conversation_id.clone();
    let mes = conversation::append_message(&state.db, input).await?;
    // 메시지 업데이트 시간 업데이트
    conversation::touch_conversation(&state.db, &conversation_id).await?;
    Ok(mes)
}

#[tauri::command]
pub async fn update_message(
    state: State<'_, AppState>,
    id: String,
    content: String,
    meta: Option<serde_json::Value>,
) -> Result<()> {
    let result = conversation::update_message(&state.db, &id, &content, &meta).await?;
    if !result {
        return Err(AppError::not_found(format!("메시지를 찾을 수 없습니다: {id}")));
    }
    Ok(())
}

#[tauri::command]
pub async fn update_message_swipe_id(
    state: State<'_, AppState>,
    id: String,
    swipe_id: i64,
) -> Result<()> {
    let mut meta = conversation::get_message_meta(&state.db, &id)
        .await?
        .unwrap_or_default();
    if let Some(obj) = meta.as_object_mut() {
        obj.insert("swipeId".to_string(), serde_json::json!(swipe_id));
    }
    // 업데이트
    conversation::update_message_meta(&state.db, &id, meta).await?;

    Ok(())
}

#[tauri::command]
pub async fn update_message_content(
    state: State<'_, AppState>,
    id: String,
    content: String,
) -> Result<()> {
    let result = conversation::update_message_content(&state.db, &id, &content).await?;
    if !result {
        return Err(AppError::not_found(format!("메시지를 찾을 수 없습니다: {id}")));
    }
    Ok(())
}

#[tauri::command]
pub async fn update_message_meta(
    state: State<'_, AppState>,
    id: String,
    meta: serde_json::Value,
) -> Result<()> {
    let result = conversation::update_message_meta(&state.db, &id, meta).await?;
    if !result {
        return Err(AppError::not_found(format!(
            "메시지를 찾을 수 없습니다: {id}"
        )));
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_message(state: State<'_, AppState>, id: String) -> Result<()> {
    let result = conversation::delete_message(&state.db, &id).await?;
    if !result {
        return Err(AppError::not_found(format!(
            "메시지를 찾을 수 없습니다: {id}"
        )));
    }
    Ok(())
}
// =======================================
// WorldInfo
// =======================================

#[tauri::command]
pub async fn get_all_worldinfo_books(state: State<'_, AppState>) -> Result<Vec<WorldInfoBook>> {
    Ok(conversation::get_all_worldinfo_books(&state.db).await?)
}

#[tauri::command]
pub async fn create_worldinfo_book(
    state: State<'_, AppState>,
    input: WorldInfoBookInput,
) -> Result<WorldInfoBook> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("월드인포 이름은 비어있을 수 없습니다", None));
    }

    let result = conversation::create_worldinfo_book(&state.db, input).await?;
    Ok(result)
}

#[tauri::command]
pub async fn delete_worldinfo_book(state: State<'_, AppState>, id: String) -> Result<()> {
    let result = conversation::delete_worldinfo_book(&state.db, &id).await?;
    if !result {
        return Err(AppError::not_found(format!("월드인포 북을 찾을 수 없습니다: {id}")));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_worldinfo_entries(
    state: State<'_, AppState>,
    book_id: String,
) -> Result<Vec<WorldInfoEntry>> {
    let result = conversation::get_all_worldinfo_entries(&state.db, &book_id).await?;
    Ok(result)
}

#[tauri::command]
pub async fn upsert_worldinfo_entry(
    state: State<'_, AppState>,
    input: WorldInfoEntryInput,
) -> Result<WorldInfoEntry> {
    if input.keys.is_empty() && !input.constant {
        return Err(AppError::validation("상시 항목이 아니면 트리거 키워드가 최소 1개 필요합니다", None));
    }

    let result = conversation::upsert_worldinfo_entry(&state.db, input).await?;
    Ok(result)
}

#[tauri::command]
pub async fn delete_worldinfo_entry(state: State<'_, AppState>, id: String) -> Result<()> {
    let result = conversation::delete_worldinfo_entry(&state.db, &id).await?;
    if !result {
        return Err(AppError::not_found(format!("월드인포 항목을 찾을 수 없습니다: {id}")));
    }
    Ok(())
}
