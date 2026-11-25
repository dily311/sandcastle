//! 대화 요약의 앱 연결: 응답이 저장된 뒤 자동 요약 (백그라운드). 켜져 있을 때만 아무것도 읽거나 부른다.
use std::sync::Arc;

use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use super::*;
use crate::{
    models::{conversation::Conversation, settings::PluginSetting},
    service::prompt::{
        prompt_manager::{context_limit_from, DEFAULT_CONTEXT_LIMIT},
        PromptManager,
    },
    state::AppState,
    sys::{plugins, settings::SettingsState},
};

/// 이 대화의 컨텍스트 한도 (프리셋에서, 없으면 기본)
pub async fn limit_of(db: &SqlitePool, preset_id: &str) -> usize {
    preset_of(db, preset_id).await.0
}

/// 대화 프리셋의 컨텍스트 한도와, 프리셋이 고친 요약 지시 (ADR-030)
pub async fn preset_of(db: &SqlitePool, preset_id: &str) -> (usize, crate::service::app_prompts::PresetPrompts) {
    let loaded = PromptManager::load_preset(db, Some(preset_id)).await.ok().flatten();
    let limit = loaded.as_ref().and_then(|p| context_limit_from(&p.preset)).unwrap_or(DEFAULT_CONTEXT_LIMIT) as usize;
    let prompts = crate::service::app_prompts::PresetPrompts::from_settings(loaded.as_ref().and_then(|p| p.preset.settings_json.as_ref()));
    (limit, prompts)
}

/// 요약에 쓸 (API, 모델): 플러그인 설정에 API를 따로 정했으면 그 API와 요약 모델(비우면 그 API의 기본 모델),
/// 아니면 이 대화가 쓴 API와 이 대화에서 고른 모델 (ADR-020)
pub fn route_for(app: &AppHandle, conv: &Conversation, fallback: &str) -> (String, Option<String>) {
    let own = app
        .try_state::<SettingsState<PluginSetting>>()
        .and_then(|s| s.0.lock().ok().map(|p| p.summary.clone()));
    if let Some(s) = own {
        if let Some(id) = s.api_profile_id.filter(|s| !s.is_empty()) {
            return (id, Some(s.model.trim().to_string()).filter(|m| !m.is_empty()));
        }
    }
    let model = conv
        .api_profile_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .and(conv.model.clone())
        .filter(|m| !m.trim().is_empty());
    (fallback.to_string(), model)
}

/// 응답이 저장된 뒤 부른다 (스트리밍이 끝나고 화면에 돌려준 다음이라 보내기를 늦추지 않음). 쌓인 만큼이 안 되었으면 아무 일도 없다.
/// 실패는 다시 시도하지 않고 기록만 남긴다 (메시지가 더 쌓인 뒤에만 다시).
pub fn after_response(app: &AppHandle, conv: &Conversation, api_profile_id: &str, preset_id: &str) {
    if !plugins::is_enabled(app, plugins::id::SUMMARY) || api_profile_id.is_empty() {
        return;
    }
    let (profile, model) = route_for(app, conv, api_profile_id);
    let (app, conv_id, preset) = (app.clone(), conv.id.clone(), preset_id.to_string());
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let ai = app.state::<Arc<AiService>>();
        let tasks = app.state::<Arc<TaskManager>>();
        let (limit, prompts) = preset_of(&state.db, &preset).await;
        let ctx = RunCtx {
            db: &state.db,
            ai: &ai,
            tasks: &tasks,
            conversation_id: &conv_id,
            profile_id: &profile,
            model: model.as_deref(),
            limit_tokens: limit,
            guards: &GLOBAL,
            prompts: &prompts,
        };
        match run_once(&ctx, true).await {
            Ok(RunOutcome::Failed(reason)) => eprintln!("[summary] 자동 요약 실패({reason}) — 메시지가 더 쌓인 뒤에 다시"),
            Ok(_) => {}
            Err(e) => eprintln!("[summary] 자동 요약을 시작하지 못함: {e}"),
        }
    });
}
