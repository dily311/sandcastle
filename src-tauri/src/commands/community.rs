//! 커뮤니티 플러그인 커맨드 (CLAUDE.md 6-D-1, ADR-012). 코어 플러그인 `community`(기본 꺼짐)가 켜져 있을 때만.
//! 파일 고르기는 화면이 한다(Android의 content 주소는 stage_picked_file로 먼저 바꿔서 넘김).
use std::{path::Path, sync::Arc};

use serde_json::Value;
use tauri::{AppHandle, Emitter, State};

use crate::{
    commands::{character_image::app_data_dir, data::move_to_trash},
    db::chat_ui,
    error::{AppError, Result},
    models::{conversation::Message, settings::AppSetting},
    service::{
        ai::AiService,
        community::{
            self, ai as plugin_ai,
            postprocess::{self, PostProcessResult},
            prompt_hook::{self, PluginPromptAnswer},
            runtime::{self, PluginLogEntry, RuntimeStart},
            PluginPreview, UserPlugin,
        },
        data::trash::{TrashItem, TrashKind},
        plugins as tools,
    },
    state::AppState,
    sys::{
        plugins::{id, require},
        settings::SettingsState,
    },
    task::TaskManager,
};

/// 설치·켜기·지우기 뒤 — 화면의 플러그인 호스트가 다시 시작한다
pub const CHANGED_EVENT: &str = "user-plugins-changed";
/// 오류가 많아 멈춤 — 호스트가 그 iframe을 내린다
pub const SUSPENDED_EVENT: &str = "user-plugin-suspended";

fn changed(app: &AppHandle) {
    let _ = app.emit(CHANGED_EVENT, ());
}

/// 설치된 플러그인 목록
#[tauri::command]
#[specta::specta]
pub async fn list_user_plugins(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<UserPlugin>> {
    require(&app, id::COMMUNITY)?;
    community::list(&state.db, &app_data_dir(&app)?).await
}

/// zip 파일이나 폴더를 읽어 미리보기 (아무것도 쓰지 않음)
#[tauri::command]
#[specta::specta]
pub async fn preview_user_plugin(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<PluginPreview> {
    require(&app, id::COMMUNITY)?;
    community::preview(&state.db, &app_data_dir(&app)?, Path::new(&path)).await
}

/// 미리보기에서 확인한 것을 설치 (hash가 다르면 거절). 새로 설치한 플러그인은 꺼져 있다
#[tauri::command]
#[specta::specta]
pub async fn install_user_plugin(app: AppHandle, state: State<'_, AppState>, path: String, hash: String) -> Result<UserPlugin> {
    require(&app, id::COMMUNITY)?;
    let p = community::install(&state.db, &app_data_dir(&app)?, Path::new(&path), &hash).await?;
    changed(&app);
    Ok(p)
}

/// 켜기(권한 전부에 동의한 목록과 함께) · 끄기
#[tauri::command]
#[specta::specta]
pub async fn set_user_plugin_enabled(app: AppHandle, state: State<'_, AppState>, plugin_id: String, enabled: bool, grant: Vec<String>) -> Result<UserPlugin> {
    require(&app, id::COMMUNITY)?;
    let p = community::set_enabled(&state.db, &app_data_dir(&app)?, &plugin_id, enabled, grant).await?;
    changed(&app);
    Ok(p)
}

/// 삭제 = 휴지통으로 (되살리면 꺼진 채)
#[tauri::command]
#[specta::specta]
pub async fn uninstall_user_plugin(app: AppHandle, state: State<'_, AppState>, plugin_id: String) -> Result<TrashItem> {
    require(&app, id::COMMUNITY)?;
    community::prepare_uninstall(&state.db, &app_data_dir(&app)?, &plugin_id).await?;
    let item = move_to_trash(&app, &state, TrashKind::UserPlugin, &plugin_id).await?;
    changed(&app);
    Ok(item)
}

// ---------------------------------------------------------------------------
// 런타임 (6-D-2)
// ---------------------------------------------------------------------------

/// 화면의 플러그인 호스트가 시작할 때: 켜진 플러그인을 도는 목록에 올리고 iframe 주소를 돌려준다
#[tauri::command]
#[specta::specta]
pub async fn start_user_plugins(app: AppHandle, state: State<'_, AppState>) -> Result<RuntimeStart> {
    if require(&app, id::COMMUNITY).is_err() {
        runtime::stop_all();
        return Err(AppError::validation("커뮤니티 플러그인이 꺼져 있음", None));
    }
    let base = app_data_dir(&app)?;
    let list = community::list(&state.db, &base).await?;
    Ok(runtime::start(&list, &base))
}

/// 호스트를 내릴 때 (코어 플러그인을 끔 등)
#[tauri::command]
#[specta::specta]
pub fn stop_user_plugins() {
    runtime::stop_all();
}

/// 플러그인 iframe의 호출. 화면이 보낸 창을 확인해 플러그인 id를 붙여 넘긴다. 권한은 여기서 다시 본다.
/// `by`: 화면의 호스트가 정한 실행 주체 — "user"(사용자가 누른 명령·확인 창에서 허락) / 그 밖은 auto. 플러그인은 정할 수 없다 (6-D-7)
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn call_user_plugin(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    tasks: State<'_, Arc<TaskManager>>,
    settings: State<'_, SettingsState<AppSetting>>,
    plugin_id: String,
    method: String,
    args: Value,
    by: Option<String>,
) -> Result<Value> {
    require(&app, id::COMMUNITY)?;
    let r = if method == "ai.generate" {
        generate_ai(&state, &ai, &tasks, &settings, &plugin_id, &args, by.as_deref() != Some("user")).await
    } else {
        community::call(&state.db, &plugin_id, &method, &args).await
    };
    if r.is_ok() && method.ends_with(".propose") {
        crate::commands::proposals::changed(&app);
    }
    if runtime::is_suspended(&plugin_id) {
        let _ = app.emit(SUSPENDED_EVENT, &plugin_id);
    }
    r
}

/// `ai.generate`: 권한·호출 수 확인 → 인자 검사 → 자동 실행 스위치·동시 하나·분당 상한 → 작업으로 실행. 실패는 플러그인 기록에 남는다
async fn generate_ai(
    state: &AppState,
    ai: &AiService,
    tasks: &TaskManager,
    settings: &SettingsState<AppSetting>,
    plugin_id: &str,
    args: &Value,
    auto: bool,
) -> Result<Value> {
    let run = async {
        runtime::check_call(plugin_id, "ai.generate")?;
        let call = plugin_ai::parse(args)?;
        let _guard = plugin_ai::admit(&state.db, plugin_id, auto).await?;
        let profile = settings
            .0
            .lock()
            .map_err(|_| AppError::internal("설정 잠금 실패"))?
            .selected_api_profile
            .clone()
            .unwrap_or_default();
        let profile = tools::pick_profile(&None, &profile)?;
        let name = runtime::name_of(plugin_id).unwrap_or_else(|| plugin_id.to_string());
        plugin_ai::generate(ai, tasks, plugin_id, &name, profile, auto, call).await
    };
    let r = run.await;
    if let Err(e) = &r {
        runtime::record_error(plugin_id, &format!("ai.generate: {e}"));
    }
    r
}

/// 누른 명령 밖에서도 플러그인이 AI를 부르게 할지 (기본 꺼짐, 요금이 들 수 있음)
#[tauri::command]
#[specta::specta]
pub async fn set_user_plugin_ai_auto(app: AppHandle, state: State<'_, AppState>, plugin_id: String, enabled: bool) -> Result<UserPlugin> {
    require(&app, id::COMMUNITY)?;
    plugin_ai::set_auto(&state.db, &plugin_id, enabled).await?;
    changed(&app);
    community::get(&state.db, &app_data_dir(&app)?, &plugin_id).await
}

/// 플러그인 기록 (최근 것, 메모리 — 앱을 다시 켜면 비어 있음)
#[tauri::command]
#[specta::specta]
pub fn get_user_plugin_log(plugin_id: String) -> Vec<PluginLogEntry> {
    runtime::log_of(&plugin_id)
}

// ---------------------------------------------------------------------------
// 메시지 후처리 (6-D-4)
// ---------------------------------------------------------------------------

/// 후처리 방식: notes(표시만) / fix(자동 고침)
#[tauri::command]
#[specta::specta]
pub async fn set_user_plugin_process_mode(app: AppHandle, state: State<'_, AppState>, plugin_id: String, mode: String) -> Result<UserPlugin> {
    require(&app, id::COMMUNITY)?;
    postprocess::set_mode(&state.db, &plugin_id, &mode).await?;
    changed(&app);
    community::get(&state.db, &app_data_dir(&app)?, &plugin_id).await
}

/// 화면의 호스트가 플러그인들을 부른 결과를 저장 (원문은 meta에). 그 사이 메시지가 바뀌었으면 거절
#[tauri::command]
#[specta::specta]
pub async fn apply_plugin_postprocess(
    app: AppHandle,
    state: State<'_, AppState>,
    message_id: String,
    expected: String,
    content: String,
    results: Vec<PostProcessResult>,
) -> Result<Message> {
    require(&app, id::COMMUNITY)?;
    postprocess::apply(&state.db, &message_id, &expected, &content, &results).await?;
    Ok(chat_ui::get_message(&state.db, &message_id).await?)
}

/// 플러그인이 고친 글을 원문으로 (그 뒤 또 고쳤으면 거절). 코어 플러그인이 꺼져 있어도 된다 — 되돌리기는 언제나
#[tauri::command]
#[specta::specta]
pub async fn revert_plugin_postprocess(state: State<'_, AppState>, message_id: String) -> Result<Message> {
    postprocess::revert(&state.db, &message_id).await?;
    Ok(chat_ui::get_message(&state.db, &message_id).await?)
}

// ---------------------------------------------------------------------------
// 프롬프트 훅 (6-D-5)
// ---------------------------------------------------------------------------

/// 화면의 호스트가 프롬프트 요청(user-plugin-prompt)에 답한다. 늦게 와서 기다리는 요청이 없으면 false
#[tauri::command]
#[specta::specta]
pub fn answer_plugin_prompt(request_id: String, answers: Vec<PluginPromptAnswer>) -> bool {
    prompt_hook::answer(&request_id, answers)
}
