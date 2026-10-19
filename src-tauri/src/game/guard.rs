//! commands.rs / npc.rs 가 공통으로 쓰는 검증 헬퍼.
use crate::state::AppState;
use crate::error::AppError;
use crate::models::settings::AppSetting;
use crate::sys::settings::SettingsState;
use tauri::State;

pub const MAX_SLOT: i64 = 20; // 앱이 임의로 두는 상한. 스키마 자체엔 슬롯 수 제한 없음.

/// 락은 여기서만 잡고 바로 놓는다. std::sync::Mutex 가드를 .await 너머로 들고 가면 안 된다.
pub fn current_game_id(state: &AppState) -> Result<String, AppError> {
    let guard = state
        .active_archive
        .lock()
        .map_err(|_| AppError::Archive("archive lock poisoned".into()))?;
    guard
        .as_ref()
        .map(|a| a.game_id.clone())
        .ok_or_else(|| AppError::NotFound("재생 중인 게임이 없습니다".into()))
}

pub fn check_slot(slot: i64) -> Result<(), AppError> {
    if (1..=MAX_SLOT).contains(&slot) {
        Ok(())
    } else {
        Err(AppError::Validation(format!("slot은 1~{MAX_SLOT} 이어야 합니다")))
    }
}

/// 설정에서 선택돼 있는 페르소나 id. 페르소나는 항상 기본값을 갖는다(AppSetting.selected_persona_id: String).
pub fn selected_persona_id(settings: &State<'_, SettingsState<AppSetting>>) -> Result<String, AppError> {
    let setting = settings.0.lock().map_err(|_| AppError::Validation("설정을 불러오지 못했습니다".into()))?;
    Ok(setting.selected_persona_id.clone())
}
