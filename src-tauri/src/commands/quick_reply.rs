//! 빠른 답장 세트 커맨드 (5-E-20-5-4, ADR-025). 실행은 화면(슬래시 엔진)에서 한다
use tauri::{AppHandle, State};

use crate::{
    db::quick_reply,
    error::{AppError, Result},
    models::{
        quick_reply::{QuickReplySet, QuickReplySetInput},
        settings::PluginSetting,
    },
    state::AppState,
    sys::settings::SettingsState,
};

/// 목록. 처음 한 번, 예전처럼 설정에 둔 세트(5-E-20-5-3)가 있으면 전역 세트로 옮기고 설정에서 비운다
#[tauri::command]
#[specta::specta]
pub async fn list_quick_reply_sets(app: AppHandle, state: State<'_, AppState>, plugin: State<'_, SettingsState<PluginSetting>>) -> Result<Vec<QuickReplySet>> {
    let old = plugin
        .0
        .lock()
        .map(|s| s.quick_reply.sets.clone())
        .map_err(|_| AppError::internal("settings lock poisoned"))?;
    if !old.is_empty() {
        for (i, s) in old.iter().enumerate() {
            let input = QuickReplySetInput {
                id: Some(s.id.clone()).filter(|x| !x.is_empty()),
                name: s.name.clone(),
                scope: "global".into(),
                scope_id: None,
                enabled: s.enabled,
                auto_send: s.auto_send,
                items: s.items.clone(),
                sort_order: i as i64,
            };
            quick_reply::upsert_quick_reply_set(&state.db, &input).await?;
        }
        crate::sys::settings::apply_patch(&app, &plugin.0, serde_json::json!({ "quickReply": { "sets": [] } }))?;
    }
    Ok(quick_reply::get_all_quick_reply_sets(&state.db).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn upsert_quick_reply_set(state: State<'_, AppState>, input: QuickReplySetInput) -> Result<QuickReplySet> {
    if !["global", "character", "preset"].contains(&input.scope.as_str()) {
        return Err(AppError::validation(format!("모르는 적용 범위: {}", input.scope), None));
    }
    if input.scope != "global" && input.scope_id.as_deref().is_none_or(str::is_empty) {
        return Err(AppError::validation("캐릭터·프리셋 전용 세트는 대상을 골라야 합니다", None));
    }
    let id = quick_reply::upsert_quick_reply_set(&state.db, &input).await?;
    Ok(quick_reply::get_quick_reply_set(&state.db, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_quick_reply_set(state: State<'_, AppState>, id: String) -> Result<()> {
    quick_reply::delete_quick_reply_set(&state.db, &id).await?;
    Ok(())
}
