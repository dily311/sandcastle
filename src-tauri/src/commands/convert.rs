//! 캐릭터 ↔ 페르소나, 게임 캐릭터 ↔ 캐릭터 복사.
//! 원본은 그대로 두고 상대 쪽에 새로 만든다. 대화 기록은 옮기지 않는다.
use std::path::Path;

use tauri::{AppHandle, State};

use crate::{
    commands::{character_image::app_data_dir, conversation::copy_avatar},
    db::{conversation, game},
    error::{AppError, Result},
    models::{
        conversation::{Character, CharacterInput, Persona, PersonaInput},
        game::{GameCharacters, GameCharactersInput},
    },
    state::AppState,
};

/// 아바타 파일이 실제로 있으면 새 파일로 복사 (원본과 따로 지울 수 있게). 없거나 경로가 아니면 버림
fn copy_avatar_file(app: &AppHandle, avatar: Option<&str>, prefix: &str) -> Option<String> {
    let src = avatar?;
    if !Path::new(src).is_file() {
        return None;
    }
    let dir = app_data_dir(app).ok()?.join("characters").join("avatars");
    copy_avatar(&dir, src, prefix).ok()
}

fn none_if_blank(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.trim().is_empty())
}

#[tauri::command]
#[specta::specta]
pub async fn copy_character_to_persona(app: AppHandle, state: State<'_, AppState>, character_id: String) -> Result<Persona> {
    let c = conversation::get_character(&state.db, &character_id)
        .await
        .map_err(|_| AppError::not_found(format!("캐릭터를 찾을 수 없습니다: {character_id}")))?;
    let input = PersonaInput {
        name: c.name,
        avatar: copy_avatar_file(&app, c.avatar.as_deref(), "persona"),
        description: c.description,
        world_info_ids: c.world_info_ids.clone(),
        tags: c.tags,
        color: c.color.unwrap_or_default(),
        is_system_default: false,
    };
    let mut tx = state.db.begin().await?;
    let p = conversation::create_persona(&mut *tx, input).await?;
    conversation::replace_persona_worldinfo(&mut tx, &p.id, &c.world_info_ids).await?;
    tx.commit().await?;
    Ok(conversation::get_persona(&state.db, &p.id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn copy_persona_to_character(app: AppHandle, state: State<'_, AppState>, persona_id: String) -> Result<Character> {
    let p = conversation::get_persona(&state.db, &persona_id)
        .await
        .map_err(|_| AppError::not_found(format!("페르소나를 찾을 수 없습니다: {persona_id}")))?;
    let input = CharacterInput {
        name: p.name,
        avatar: copy_avatar_file(&app, p.avatar.as_deref(), "character"),
        description: p.description,
        alternate_descriptions: vec![],
        first_message: None,
        alternate_first_message: vec![],
        example_dialogue: None,
        system_prompt_override: None,
        creator_notes: None,
        world_info_ids: p.world_info_ids.clone(),
        tags: p.tags,
        color: none_if_blank(Some(p.color)),
        category: None,
        default_persona_id: None,
        default_prompt_preset_id: None,
        favorite: false,
    };
    let mut tx = state.db.begin().await?;
    let c = conversation::create_character(&mut *tx, input).await?;
    conversation::replace_character_worldinfo(&mut tx, &c.id, &p.world_info_ids).await?;
    tx.commit().await?;
    Ok(conversation::get_character(&state.db, &c.id).await?)
}

/// 캐릭터를 게임에 넣음 (게임 캐릭터는 원본 캐릭터를 기억함). 아바타는 게임 에셋으로 따로 넣어야 함
#[tauri::command]
#[specta::specta]
pub async fn copy_character_to_game(state: State<'_, AppState>, character_id: String, game_id: String) -> Result<GameCharacters> {
    let c = conversation::get_character(&state.db, &character_id)
        .await
        .map_err(|_| AppError::not_found(format!("캐릭터를 찾을 수 없습니다: {character_id}")))?;
    game::get_game_project(&state.db, &game_id)
        .await
        .map_err(|_| AppError::not_found(format!("게임을 찾을 수 없습니다: {game_id}")))?;
    let id = uuid::Uuid::new_v4().to_string();
    let input = GameCharactersInput {
        id: Some(id.clone()),
        game_id: game_id.clone(),
        character_id: Some(c.id),
        name: c.name,
        avatar: None,
        description: Some(c.description).filter(|s| !s.trim().is_empty()),
        first_message: c.first_message,
        example_dialogue: c.example_dialogue,
        system_prompt_override: c.system_prompt_override,
        behavior_type: None,
        world_info_ids: vec![],
        tags: Some(serde_json::to_string(&c.tags).unwrap_or_else(|_| "[]".into())),
        settings_json: serde_json::json!({}),
    };
    game::upsert_game_character(&state.db, &game_id, input).await?;
    Ok(game::get_game_character(&state.db, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn copy_game_character_to_character(app: AppHandle, state: State<'_, AppState>, game_character_id: String) -> Result<Character> {
    let g = game::get_game_character(&state.db, &game_character_id)
        .await
        .map_err(|_| AppError::not_found(format!("게임 캐릭터를 찾을 수 없습니다: {game_character_id}")))?;
    let input = CharacterInput {
        name: g.name,
        avatar: copy_avatar_file(&app, g.avatar.as_deref(), "character"),
        description: g.description.unwrap_or_default(),
        alternate_descriptions: vec![],
        first_message: none_if_blank(g.first_message),
        alternate_first_message: vec![],
        example_dialogue: none_if_blank(g.example_dialogue),
        system_prompt_override: none_if_blank(g.system_prompt_override),
        creator_notes: None,
        world_info_ids: g.world_info_ids.clone(),
        tags: g.tags.unwrap_or_default(),
        color: None,
        category: None,
        default_persona_id: None,
        default_prompt_preset_id: None,
        favorite: false,
    };
    let mut tx = state.db.begin().await?;
    let c = conversation::create_character(&mut *tx, input).await?;
    conversation::replace_character_worldinfo(&mut tx, &c.id, &g.world_info_ids).await?;
    tx.commit().await?;
    Ok(conversation::get_character(&state.db, &c.id).await?)
}
