//! 프롬프트 프리셋 관리
use serde::Serialize;
use tauri::State;

use crate::{
    db::ai_service as db,
    error::{AppError, Result},
    models::ai_service::{PromptBlocks, PromptBlocksInput, PromptPresets, PromptPresetsInput},
    state::AppState,
};

/// 프리셋 + 블록 (블록은 실행 순서대로)
#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptPresetDetail {
    pub preset: PromptPresets,
    pub blocks: Vec<PromptBlocks>,
}

async fn detail(state: &AppState, id: &str) -> Result<PromptPresetDetail> {
    let preset = db::get_prompt_preset(&state.db, id)
        .await
        .map_err(|_| AppError::not_found(format!("프리셋을 찾을 수 없습니다: {id}")))?;
    let blocks = db::get_prompt_blocks_by_preset(&state.db, id).await?;
    Ok(PromptPresetDetail { preset, blocks })
}

#[tauri::command]
#[specta::specta]
pub async fn list_prompt_presets(state: State<'_, AppState>) -> Result<Vec<PromptPresets>> {
    let mut list = db::get_all_prompt_presets(&state.db).await?;
    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(list)
}

#[tauri::command]
#[specta::specta]
pub async fn get_prompt_preset(state: State<'_, AppState>, id: String) -> Result<PromptPresetDetail> {
    detail(&state, &id).await
}

/// 프리셋과 블록을 한 번에 저장. `blocks` 순서가 실행 순서가 되고, 목록에 없는 기존 블록은 지워진다.
#[tauri::command]
#[specta::specta]
pub async fn save_prompt_preset(state: State<'_, AppState>, preset: PromptPresetsInput, blocks: Vec<PromptBlocksInput>) -> Result<PromptPresetDetail> {
    if preset.name.trim().is_empty() {
        return Err(AppError::validation("프리셋 이름이 비어 있습니다", None));
    }
    if !matches!(preset.preset_type.as_str(), "chat_completion" | "text_completion") {
        return Err(AppError::validation("preset_type은 chat_completion 또는 text_completion이어야 합니다", None));
    }
    if let Some(b) = blocks.iter().find(|b| !matches!(b.role.as_str(), "system" | "user" | "assistant")) {
        return Err(AppError::validation(format!("블록 '{}'의 role이 올바르지 않습니다", b.name), None));
    }
    if let Some(b) = blocks.iter().find(|b| !matches!(b.position.as_str(), "relative" | "in_chat")) {
        return Err(AppError::validation(format!("블록 '{}'의 position이 올바르지 않습니다", b.name), None));
    }
    let mut tx = state.db.begin().await?;
    let id = db::upsert_prompt_preset(&mut *tx, preset).await?;
    db::replace_prompt_blocks(&mut tx, &id, blocks).await?;
    tx.commit().await?;
    detail(&state, &id).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_prompt_preset(state: State<'_, AppState>, id: String) -> Result<()> {
    if !db::delete_prompt_preset(&state.db, &id).await? {
        return Err(AppError::not_found(format!("프리셋을 찾을 수 없습니다: {id}")));
    }
    Ok(())
}
