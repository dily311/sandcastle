//! 대화 요약 커맨드 (CLAUDE.md 3-D, ADR-016). 보기·고치기·지우기·되돌리기는 플러그인이 꺼져 있어도 된다(데이터를 다루는 길은 늘 열려 있어야 함).
//! 요약을 만드는 것("지금 요약")만 플러그인이 켜져 있어야 하고, 사용자가 누른 것이라 `run_by = user`이다.
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{
    db::conversation,
    error::{AppError, Result},
    models::settings::AppSetting,
    service::{
        ai::AiService,
        summary::{self, hooks, RunCtx, RunOutcome, SummaryInfo},
    },
    state::AppState,
    sys::{plugins, settings::SettingsState},
    task::TaskManager,
};

#[tauri::command]
#[specta::specta]
pub async fn get_conversation_summary(state: State<'_, AppState>, conversation_id: String) -> Result<Option<SummaryInfo>> {
    Ok(summary::get(&state.db, &conversation_id).await?.map(|r| r.info()))
}

/// 사용자가 요약 글을 고침 (고친 글은 그대로 AI에게 가고, 다음 자동 요약이 그 위에 이어 씀)
#[tauri::command]
#[specta::specta]
pub async fn set_conversation_summary(state: State<'_, AppState>, conversation_id: String, text: String) -> Result<SummaryInfo> {
    summary::save_user(&state.db, &conversation_id, &text).await
}

#[tauri::command]
#[specta::specta]
pub async fn clear_conversation_summary(state: State<'_, AppState>, conversation_id: String) -> Result<()> {
    summary::clear(&state.db, &conversation_id).await
}

/// 한 단계 전 글로 되돌리기 (없으면 None)
#[tauri::command]
#[specta::specta]
pub async fn undo_conversation_summary(state: State<'_, AppState>, conversation_id: String) -> Result<Option<SummaryInfo>> {
    summary::undo(&state.db, &conversation_id).await
}

/// 지금 요약: 사용자가 누른 것(`run_by = user`). 창과 상관없이 최근 몇 개만 원문으로 남기고 요약한다. 요약할 것이 없으면 오류
#[tauri::command]
#[specta::specta]
pub async fn summarize_conversation_now(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    tasks: State<'_, Arc<TaskManager>>,
    settings: State<'_, SettingsState<AppSetting>>,
    conversation_id: String,
) -> Result<SummaryInfo> {
    plugins::require(&app, plugins::id::SUMMARY)?;
    let conv = conversation::get_conversation(&state.db, &conversation_id).await?;
    let (app_profile, app_preset) = settings
        .0
        .lock()
        .map(|s| (s.selected_api_profile.clone().unwrap_or_default(), s.selected_preset.clone()))
        .map_err(|_| AppError::internal("settings lock poisoned"))?;
    let conv_profile = conv.api_profile_id.clone().filter(|s| !s.is_empty()).unwrap_or(app_profile);
    if conv_profile.is_empty() {
        return Err(AppError::validation("API를 먼저 고르세요 (대화 설정 또는 설정 > API 키)", None));
    }
    let profile = hooks::profile_for(&app, &conv_profile);
    let preset = conv.prompt_preset_id.clone().filter(|s| !s.is_empty()).unwrap_or(app_preset);
    let limit = hooks::limit_of(&state.db, &preset).await;
    let ctx = RunCtx {
        db: &state.db,
        ai: &ai,
        tasks: &tasks,
        conversation_id: &conversation_id,
        profile_id: &profile,
        limit_tokens: limit,
        guards: &summary::GLOBAL,
    };
    match summary::run_once(&ctx, false).await? {
        RunOutcome::Updated(_) => Ok(summary::get(&state.db, &conversation_id)
            .await?
            .map(|r| r.info())
            .ok_or_else(|| AppError::internal("요약을 저장하지 못함"))?),
        RunOutcome::Discarded => Err(AppError::validation("요약하는 사이 요약이 바뀌어 결과를 버렸습니다. 다시 눌러 주세요", None)),
        RunOutcome::NotDue => Err(AppError::validation("이 대화는 이미 요약하는 중입니다", None)),
        RunOutcome::Failed(reason) => Err(AppError::validation(
            format!("요약하지 못함 ({reason}) — 실행 기록에서 이유를 볼 수 있습니다"),
            None,
        )),
    }
}
