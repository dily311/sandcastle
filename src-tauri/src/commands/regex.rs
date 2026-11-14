//! 정규식 치환 스크립트 관리 / 적용 커맨드
use tauri::State;

use crate::{
    db,
    error::{AppError, Result},
    models::{
        regex::{RegexPlacement, RegexScope, RegexScript, RegexScriptInput, RegexTarget},
        settings::AppSetting,
    },
    service::chat::regex_processor::{preview_script, validate_script, RegexMacros, RegexProcessor},
    state::AppState,
    sys::settings::SettingsState,
};

#[tauri::command]
#[specta::specta]
pub async fn list_regex_scripts(state: State<'_, AppState>) -> Result<Vec<RegexScript>> {
    Ok(db::regex::get_all_regex_scripts(&state.db).await?)
}

/// 생성/수정. 패턴이 컴파일되지 않거나 범위 대상이 없으면 저장하지 않는다.
#[tauri::command]
#[specta::specta]
pub async fn upsert_regex_script(state: State<'_, AppState>, input: RegexScriptInput) -> Result<RegexScript> {
    validate_input(&input)?;
    let id = db::regex::upsert_regex_script(&state.db, &input).await?;
    Ok(db::regex::get_regex_script(&state.db, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_regex_script(state: State<'_, AppState>, id: String) -> Result<()> {
    if !db::regex::delete_regex_script(&state.db, &id).await? {
        return Err(AppError::not_found(format!("정규식 스크립트를 찾을 수 없습니다: {id}")));
    }
    Ok(())
}

/// 편집 화면 미리보기: 저장하지 않고 텍스트에 적용한 결과
#[tauri::command]
#[specta::specta]
pub fn preview_regex_script(
    input: RegexScriptInput,
    text: String,
    char_name: Option<String>,
    user_name: Option<String>,
) -> Result<String> {
    let script = input.to_script(String::new(), 0);
    let macros = RegexMacros {
        char_name: char_name.unwrap_or_default(),
        user_name: user_name.unwrap_or_default(),
    };
    preview_script(&script, &text, &macros)
}

/// 대화 문맥으로 텍스트에 정규식 적용.
/// 프론트는 주로 화면 표시용(target = display)으로 쓴다. 저장/프롬프트용 치환은 백엔드가 알아서 한다.
#[tauri::command]
#[specta::specta]
pub async fn apply_regex(
    state: State<'_, AppState>,
    settings: State<'_, SettingsState<AppSetting>>,
    conversation_id: String,
    text: String,
    placement: RegexPlacement,
    target: RegexTarget,
    depth: Option<usize>,
) -> Result<String> {
    let preset_id = selected_preset(&settings);
    let processor = RegexProcessor::for_conversation(&state.db, &conversation_id, preset_id.as_deref()).await?;
    Ok(processor.apply(&text, placement, &[target], depth))
}

pub(crate) fn selected_preset(settings: &SettingsState<AppSetting>) -> Option<String> {
    settings
        .0
        .lock()
        .ok()
        .map(|s| s.selected_preset.clone())
        .filter(|s| !s.is_empty())
}

fn validate_input(input: &RegexScriptInput) -> Result<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("스크립트 이름이 비어 있습니다", None));
    }
    if input.placement.is_empty() {
        return Err(AppError::validation("적용 대상(placement)을 하나 이상 골라 주세요", None));
    }
    if input.scope != RegexScope::Global && input.scope_id.as_deref().unwrap_or("").is_empty() {
        return Err(AppError::validation("캐릭터/프리셋 범위에는 대상 id가 필요합니다", None));
    }
    validate_script(&input.to_script(String::new(), 0))
}
