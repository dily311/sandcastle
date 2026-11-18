//! 메모리 임베딩 검색 커맨드 (CLAUDE.md 3-C, ADR-015). 유료 호출(색인·질의 임베딩)은 TaskManager 작업 안에서 — 실행 주체는 늘 사용자.
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{
    error::{AppError, Result},
    models::settings::{MemorySetting, PluginSetting},
    service::{
        ai::AiService,
        memory::{
            self,
            index::{self, IndexPreview, IndexResult},
            search::{self, SearchRequest, SearchResult},
            store::MemoryStore,
            MemoryScope, MemoryStatus,
        },
    },
    state::AppState,
    sys::{plugins, settings::SettingsState},
    task::{CreateTaskRequest, TaskManager, TaskOwner, TaskType},
};

fn cfg_of(plugin: &SettingsState<PluginSetting>) -> Result<MemorySetting> {
    plugin
        .0
        .lock()
        .map(|s| s.memory.clone())
        .map_err(|_| AppError::internal("settings lock poisoned"))
}

/// 설정의 임베딩 프로필 (없거나 지워졌으면 검증 오류)
async fn profile_of(ai: &AiService, cfg: &MemorySetting) -> Result<crate::models::ai_service::ApiProfiles> {
    let id = memory::check_config(cfg)?;
    ai.load_profile(&id).await
}

/// 상태 보기: 플러그인이 꺼져 있어도 부를 수 있다(지우기 화면용). 파일이 없으면 만들지 않는다
#[tauri::command]
#[specta::specta]
pub async fn get_memory_status(
    ai: State<'_, Arc<AiService>>,
    store: State<'_, Arc<MemoryStore>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
) -> Result<MemoryStatus> {
    let cfg = cfg_of(&plugin)?;
    let key = match profile_of(&ai, &cfg).await {
        Ok(p) => Some(memory::model_key(&p, &cfg)),
        Err(_) => None,
    };
    memory::status(&store, key.as_deref()).await
}

/// 색인 미리보기: 요금 없음 (공급자를 부르지 않음)
#[tauri::command]
#[specta::specta]
pub async fn preview_memory_index(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    store: State<'_, Arc<MemoryStore>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    scope: MemoryScope,
) -> Result<IndexPreview> {
    plugins::require(&app, plugins::id::MEMORY)?;
    let cfg = cfg_of(&plugin)?;
    let profile = profile_of(&ai, &cfg).await?;
    index::preview(&state.db, &store, &profile, &cfg, &scope).await
}

#[allow(clippy::too_many_arguments)]
/// 색인 실행: 사용자가 미리보기를 보고 누른 것만. 상한을 넘는 계획은 `allow_over_cap`(두 번째 확인)이 있어야 돈다
#[tauri::command]
#[specta::specta]
pub async fn run_memory_index(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    tasks: State<'_, Arc<TaskManager>>,
    store: State<'_, Arc<MemoryStore>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    scope: MemoryScope,
    allow_over_cap: bool,
) -> Result<IndexResult> {
    plugins::require(&app, plugins::id::MEMORY)?;
    let cfg = cfg_of(&plugin)?;
    let profile = profile_of(&ai, &cfg).await?;
    let label = if scope.conversation_id.is_some() {
        "메모리 색인 · 이 대화"
    } else {
        "메모리 색인 · 전체"
    };
    let job = CreateTaskRequest::inline(TaskType::Embedding, TaskOwner::System, memory::FEATURE, label, false);
    tasks
        .run_inline(job, |cancel| async move {
            index::run(&state.db, &store, &ai, &profile, &cfg, &scope, allow_over_cap, &cancel).await
        })
        .await
}

/// 뜻으로 찾기: 질의 임베딩 한 번(작업 하나)
#[tauri::command]
#[specta::specta]
pub async fn search_memory(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    tasks: State<'_, Arc<TaskManager>>,
    store: State<'_, Arc<MemoryStore>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    request: SearchRequest,
) -> Result<SearchResult> {
    plugins::require(&app, plugins::id::MEMORY)?;
    let cfg = cfg_of(&plugin)?;
    let profile = profile_of(&ai, &cfg).await?;
    let job = CreateTaskRequest::inline(TaskType::Embedding, TaskOwner::System, memory::FEATURE, "메모리 검색", false);
    tasks
        .run_inline(job, |cancel| async move {
            search::search(&state.db, &store, &ai, &profile, &cfg, &request, &cancel).await
        })
        .await
}

/// 정리: "other" 다른 모델로 색인된 것 / "orphans" 없어진 대화·가리키지 않는 벡터 / "all" 메모리 파일 전부.
/// 플러그인이 꺼져 있어도 된다 — 데이터를 지우는 길은 늘 열려 있어야 함. 지운 chunk 수(all은 0)
#[tauri::command]
#[specta::specta]
pub async fn purge_memory(
    ai: State<'_, Arc<AiService>>,
    state: State<'_, AppState>,
    store: State<'_, Arc<MemoryStore>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    kind: String,
) -> Result<u64> {
    match kind.as_str() {
        "all" => store.wipe().await.map(|_| 0),
        "other" | "orphans" => {
            let cfg = cfg_of(&plugin)?;
            let key = match profile_of(&ai, &cfg).await {
                Ok(p) => Some(memory::model_key(&p, &cfg)),
                Err(_) => None,
            };
            memory::purge(&store, &state.db, key.as_deref(), kind == "other").await
        }
        other => Err(AppError::validation(format!("알 수 없는 정리 종류: {other}"), None)),
    }
}
