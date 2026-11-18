//! 대필 커맨드 (CLAUDE.md 5-D, ADR-018). 사용자가 펜 버튼을 누른 것이라 `run_by = user`. 결과는 저장하지 않고 입력칸에 채울 글로 돌려준다.
use std::{
    collections::HashSet,
    sync::{Arc, LazyLock, Mutex},
};

use tauri::{ipc::Channel, AppHandle, State};

use crate::{
    commands::chat::into_app_error,
    error::{AppError, Result},
    models::{
        ai_service::StreamPayload,
        settings::{AppSetting, PluginSetting},
    },
    service::chat::{chat_manager::ChatManager, ghost},
    state::AppState,
    sys::{plugins, settings::SettingsState},
    task::{CreateTaskRequest, TaskManager, TaskOwner, TaskStatus, TaskType},
};

pub const FEATURE: &str = plugins::id::GHOSTWRITE;

/// 한 대화에서 대필은 한 번에 하나
static RUNNING: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

struct Guard(String);
impl Guard {
    fn take(id: &str) -> Option<Self> {
        let mut g = RUNNING.lock().ok()?;
        g.insert(id.to_string()).then(|| Self(id.to_string()))
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        if let Ok(mut g) = RUNNING.lock() {
            g.remove(&self.0);
        }
    }
}

/// 초안(없을 수 있음)으로 사용자의 다음 대사를 쓴다. 쓰는 동안 글 조각을 `on_event`로 흘려보내고, 끝나면 정리한 글을 돌려준다.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
#[specta::specta]
pub async fn ghostwrite(
    app: AppHandle,
    state: State<'_, AppState>,
    tasks: State<'_, Arc<TaskManager>>,
    settings: State<'_, SettingsState<AppSetting>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    conversation_id: String,
    brief: Option<String>,
    on_event: Channel<StreamPayload>,
) -> Result<String> {
    plugins::require(&app, plugins::id::GHOSTWRITE)?;
    let cfg = plugin
        .0
        .lock()
        .map(|s| s.ghost.clone())
        .map_err(|_| AppError::internal("settings lock poisoned"))?;
    let brief = brief.as_deref().and_then(ghost::clean_brief);
    let Some(_guard) = Guard::take(&conversation_id) else {
        return Err(AppError::validation("이미 대필 중", None));
    };
    let job = CreateTaskRequest::inline(TaskType::Ghostwrite, TaskOwner::Chat, FEATURE, "대필", false).owner_ref(&conversation_id);
    tasks
        .run_inline(job, |cancel| async move {
            ChatManager
                .ghost(&app, &state.db, &settings, &conversation_id, brief, &cfg, Some(on_event), &cancel)
                .await
                .map_err(into_app_error)
        })
        .await
}

/// 진행 중인 대필 멈추기 (받은 데까지는 화면이 이미 입력칸에 가지고 있다). 멈출 게 있었으면 true
#[tauri::command]
#[specta::specta]
pub async fn ghostwrite_cancel(tasks: State<'_, Arc<TaskManager>>, conversation_id: String) -> Result<bool> {
    let running = tasks
        .list_by_owner_ref(&conversation_id)
        .await?
        .into_iter()
        .find(|t| t.task_type == TaskType::Ghostwrite && matches!(t.status, TaskStatus::Pending | TaskStatus::Running));
    match running {
        Some(t) => tasks.cancel(&t.id).await.map(|_| true),
        None => Ok(false),
    }
}
