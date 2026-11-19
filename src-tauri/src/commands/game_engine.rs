//! 호스트 웹뷰만 호출하는 command. game_id는 인자로 받지 않는다:
//! AppState.active_archive에서 Rust가 직접 읽는다.
//!
//! 세션(game_player_sessions)=책 한 권, 세이브(game_saves)=그 책의 책갈피(일지 seq 위치 + 스냅샷).
//! 화면 흐름: 세션 목록(list_sessions) → continue_game(session_id)로 그 책을 이어서 플레이.
//! 오래된 책갈피 불러오기는 load_save가 분기(game::session::fork_session)한다.
//!
//! ⚠ AppState에 active_runtime: tokio::sync::Mutex<Option<GameRuntime>>가 필요하다.
//! std Mutex가 아닌 이유: "락 잡고 → dispatch → 저장(async)"을 한 번에 원자적으로 하려면
//! 가드를 .await 너머로 들고 갈 수 있어야 한다.
use crate::db::game::{
    get_all_game_conversations, get_all_game_player_sessions, get_all_game_save, get_game_journal_lookup, get_game_messages_lookup, get_game_player_sessions,
    get_game_save, upsert_game_save,
};
use crate::error::AppError;
use crate::game::guard::{check_slot, current_game_id};
use crate::game::runtime::{GameCommand, GameRepository, GameResponse, GameRuntime, SqlxGameRepository};
use crate::game::transcript::{self, ContinueGameResponse, TranscriptPage};
use crate::models::game::{GameConversations, GameJournal, GameMessages, GamePlayerSessions, GameSave, GameSaveInput};
use crate::models::settings::AppSetting;
use crate::state::AppState;
use crate::sys::settings::SettingsState;
use tauri::{ipc::Channel, AppHandle, State};

const MAX_SAVE_BYTES: usize = 256 * 1024;

/// 새 게임(새 책) 시작. 세션을 만들어 바로 저장해서 세션 목록에 보이게 하고,
/// GameRuntime을 active_runtime에 넣어둔다.
#[tauri::command]
#[specta::specta]
pub async fn start_new_game(state: State<'_, AppState>, settings: State<'_, SettingsState<AppSetting>>) -> Result<GameResponse, AppError> {
    let game_id = current_game_id(&state)?;

    let definition = crate::game::guard::current_definition(&state)?; // open_game이 올려둔 정의
                                                                      // 데이터가 어긋나 있으면(없는 씬을 가리키는 선택지 등) 시작 전에 막는다. 경고는 validate_game으로 본다.
    let issues = crate::game::validate::validate(&definition);
    if let Some(msg) = crate::game::validate::errors_summary(&issues) {
        let errors: Vec<_> = issues.iter().filter(|i| i.severity == crate::game::validate::Severity::Error).collect();
        return Err(AppError::game_data_invalid(
            format!("게임 데이터에 문제가 있어 시작할 수 없습니다: {msg}"),
            serde_json::to_value(errors).ok(),
        ));
    }
    let session_id = uuid::Uuid::new_v4().to_string();
    // 지금 선택돼 있는 페르소나를 이 세션에 고정한다. 세션 중에 설정에서 바꿔도 이 세션엔 반영 안 됨.
    let player_id = crate::game::guard::selected_persona_id(&settings)?;
    let mut runtime = GameRuntime::from_new_game(definition, session_id.clone(), player_id).map_err(|e| AppError::game_data_invalid(e.to_string(), None))?;

    // 첫 턴: GameStarted를 흘려서 NPC의 처음 위치도 굴린다. record가 세션 행을 먼저 만들고
    // 일지/NPC 상태를 한 트랜잭션으로 쓴다.
    let events = runtime.start();
    crate::game::journal::record(&state.db, &mut runtime, &game_id, None, &events).await?;

    let response = GameResponse {
        events,
        state: runtime.render_state(),
    };
    *state.active_runtime.lock().await = Some(runtime);
    Ok(response)
}

/// 화면에 그릴 정적 데이터(맵/위치/캐릭터/씬/변수 정의 등)를 돌려준다. 플레이 중에만 부를 수 있고,
/// 이미 메모리에 있는 정의에서 만들어서 DB를 안 읽는다. 어떤 필드를 주고/안 주는지는 runtime/view.rs 참고.
/// 현재 값(위치, 시간, 변수 값 등)은 RenderState에 있다.
#[tauri::command]
#[specta::specta]
pub fn get_game_view(state: State<'_, AppState>) -> Result<serde_json::Value, AppError> {
    // 플레이를 시작하기 전에도 부를 수 있다 (open_game의 응답에도 같은 값이 들어 있다).
    Ok(crate::game::guard::current_definition(&state)?.view())
}

/// 게임 정의의 일관성 검사 결과 (Error/Warning). 임포트 직후나 편집기에서 보여주기 위한 것.
#[tauri::command]
#[specta::specta]
pub async fn validate_game(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<crate::game::validate::Issue>, AppError> {
    use tauri::Manager;
    let game_id = current_game_id(&state)?;
    let repo = SqlxGameRepository::new(&state.db);
    let definition = repo.load_definition(&game_id).await.map_err(|e| AppError::database(e.to_string()))?;
    let game_dir = app.path().app_data_dir()?.join("games").join(&game_id);
    Ok(crate::game::validate::diagnose(&definition, Some(game_dir.as_path())))
}

/// 이 게임의 세션(책) 목록. 세션 목록 화면용.
#[tauri::command]
#[specta::specta]
pub async fn list_sessions(state: State<'_, AppState>) -> Result<Vec<GamePlayerSessions>, AppError> {
    let game_id = current_game_id(&state)?;
    Ok(get_all_game_player_sessions(&state.db, &game_id).await?)
}

/// 이 게임의 세션 삭제용
#[tauri::command]
#[specta::specta]
pub async fn delete_sessions(app: AppHandle, state: State<'_, AppState>, session_id: String) -> Result<crate::service::data::trash::TrashItem, AppError> {
    crate::commands::data::move_to_trash(&app, &state, crate::service::data::trash::TrashKind::GameSession, &session_id).await
}

/// 세션이 이 게임의 것인지 확인하고 돌려준다 (다른 게임의 세션 id를 넣는 위조 방지).
async fn owned_session(state: &AppState, game_id: &str, session_id: &str) -> Result<GamePlayerSessions, AppError> {
    let session = get_game_player_sessions(&state.db, session_id).await?;
    if session.game_id != game_id {
        return Err(AppError::validation("이 게임의 세션이 아닙니다", None));
    }
    Ok(session)
}

/// 세션을 런타임으로 불러와 active_runtime에 올린다.
const DEFAULT_HISTORY: usize = 50;
const MAX_HISTORY: usize = 200;

fn history_limit(requested: Option<u32>) -> usize {
    requested.map_or(DEFAULT_HISTORY, |n| (n as usize).clamp(1, MAX_HISTORY))
}

async fn open_session(state: &AppState, _game_id: &str, session_id: &str, history: usize) -> Result<ContinueGameResponse, AppError> {
    let repo = SqlxGameRepository::new(&state.db);
    let definition = crate::game::guard::current_definition(state)?;
    let runtime = GameRuntime::load(&repo, definition, session_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    // 이어하기는 GameStarted가 아니다 — 새로 생긴 일이 없으니 events는 빈 배열이다.
    // 대신 이전에 나온 대사를 history로 같이 준다 (일지에서 읽는다). 지금 화면의 모습은 state.stage가 복원한다.
    let history = transcript::transcript_page(&state.db, session_id, runtime.state().head_seq, None, history).await?;
    let response = ContinueGameResponse {
        events: vec![],
        state: runtime.render_state(),
        history,
    };
    *state.active_runtime.lock().await = Some(runtime);
    Ok(response)
}

/// 세션(책)을 그대로 이어서 플레이한다. 위치/시간/날씨/씬은 세션 테이블에서 복원된다.
#[tauri::command]
#[specta::specta]
pub async fn continue_game(state: State<'_, AppState>, session_id: String, option_history_limit: Option<u32>) -> Result<ContinueGameResponse, AppError> {
    let game_id = current_game_id(&state)?;
    owned_session(&state, &game_id, &session_id).await?;
    open_session(&state, &game_id, &session_id, history_limit(option_history_limit)).await
}

/// 이전 대사 기록을 더 읽는다 (이어하기 응답의 history보다 오래된 것). beforeSeq에 history.oldestSeq를 넘기면
/// 그보다 오래된 limit개를 돌려준다. 연출까지 전부 재생하려면 read_journal을 쓴다.
#[tauri::command]
#[specta::specta]
pub async fn read_transcript(state: State<'_, AppState>, session_id: String, before_seq: Option<u32>, limit: Option<u32>) -> Result<TranscriptPage, AppError> {
    let game_id = current_game_id(&state)?;
    let session = owned_session(&state, &game_id, &session_id).await?;
    transcript::transcript_page(&state.db, &session_id, session.head_seq, before_seq, history_limit(limit)).await
}

/// 지금 씬 또는 지금 대화의 기록 ("지난 대사" 창). 씬이 열려 있으면 그 씬이 마지막으로 열린 뒤의 대본과 씬 대화,
/// 씬 밖에서 NPC와 대화 중이면 그 대화를 시작한 뒤의 메시지. 둘 다 아니면 최근 기록(limit개).
#[tauri::command]
#[specta::specta]
pub async fn read_current_transcript(state: State<'_, AppState>, limit: Option<u32>) -> Result<TranscriptPage, AppError> {
    let game_id = current_game_id(&state)?;
    let (session_id, head_seq, scene_id, npc_id) = {
        let guard = state.active_runtime.lock().await;
        let st = guard.as_ref().ok_or_else(AppError::game_no_session)?.state();
        (st.session_id.clone(), st.head_seq, st.current_scene_id.clone(), st.active_npc_id.clone())
    };
    let limit = history_limit(limit);
    let conversations = || async {
        get_all_game_conversations(&state.db, &game_id)
            .await
            .map(|all| all.into_iter().filter(|c| c.session_id == session_id).collect::<Vec<_>>())
    };
    let scope = if let Some(scene_id) = scene_id {
        let ids = conversations().await?.into_iter().filter(|c| c.branch_key == scene_id).map(|c| c.id).collect();
        transcript::CurrentScope::Scene {
            scene_id,
            conversation_ids: ids,
        }
    } else if let Some(character_id) = npc_id {
        let key = crate::game::npc::participants_key(&[character_id.as_str()]);
        let conversation_id = conversations()
            .await?
            .into_iter()
            .find(|c| c.branch_key == crate::game::npc::DEFAULT_BRANCH_KEY && c.participants_key == key)
            .map(|c| c.id);
        transcript::CurrentScope::Talk { character_id, conversation_id }
    } else {
        return transcript::transcript_page(&state.db, &session_id, head_seq, None, limit).await;
    };
    transcript::current_page(&state.db, &session_id, head_seq, &scope, limit).await
}

/// 분기할 수 있는 시점 목록 (최신 → 오래된 순). 턴이 끝날 때마다 하나씩 생긴다.
#[tauri::command]
#[specta::specta]
pub async fn list_turn_points(state: State<'_, AppState>, session_id: String, limit: Option<u32>) -> Result<Vec<transcript::TurnPoint>, AppError> {
    let game_id = current_game_id(&state)?;
    let session = owned_session(&state, &game_id, &session_id).await?;
    transcript::turn_points(&state.db, &session_id, session.head_seq, limit.map_or(100, |n| (n as usize).clamp(1, 500))).await
}

/// 시점(seq)에서 분기해 새 세션으로 이어서 플레이한다. 원래 세션은 그대로 남는다.
/// 응답의 state.sessionId가 새 세션이다.
#[tauri::command]
#[specta::specta]
pub async fn fork_at(state: State<'_, AppState>, session_id: String, seq: u32, option_history_limit: Option<u32>) -> Result<ContinueGameResponse, AppError> {
    let game_id = current_game_id(&state)?;
    owned_session(&state, &game_id, &session_id).await?;
    let target = crate::game::session::fork_at_seq(&state.db, &game_id, &session_id, seq).await?;
    open_session(&state, &game_id, &target, history_limit(option_history_limit)).await
}

/// 책갈피를 불러와서 거기서부터 이어서 플레이한다.
///  - 그 책갈피가 세션의 맨 끝(이후 진행 없음)이면 같은 세션을 그대로 이어간다.
///  - 이후 기록이 있으면 그 지점까지를 복사한 새 세션으로 분기한다 (원래 기록은 보존).
/// 응답의 state.sessionId가 실제로 열린 세션이다 (분기면 새 세션 id).
#[tauri::command]
#[specta::specta]
pub async fn load_save(state: State<'_, AppState>, session_id: String, slot: i64, option_history_limit: Option<u32>) -> Result<ContinueGameResponse, AppError> {
    let game_id = current_game_id(&state)?;
    check_slot(slot)?;
    let session = owned_session(&state, &game_id, &session_id).await?;
    let save = get_game_save(&state.db, &session_id, slot).await?; // 없으면 NotFound

    let target = if save.seq >= session.head_seq {
        session_id.clone()
    } else {
        crate::game::session::fork_session(&state.db, &game_id, &session_id, slot).await?
    };
    open_session(&state, &game_id, &target, history_limit(option_history_limit)).await
}

/// 일지 읽기 전용 재생용: 세션의 [from_seq, to_seq] 구간. AI를 다시 호출하지 않는다.
#[tauri::command]
#[specta::specta]
pub async fn read_journal(state: State<'_, AppState>, session_id: String, from_seq: Option<u32>, to_seq: u32) -> Result<Vec<GameJournal>, AppError> {
    const MAX_RANGE: u32 = 1000; // 한 번에 읽는 양 상한 (큰 세션을 통째로 끌어오지 않게)
    let game_id = current_game_id(&state)?;
    owned_session(&state, &game_id, &session_id).await?;
    if to_seq.saturating_sub(from_seq.unwrap_or(0)) > MAX_RANGE {
        return Err(AppError::validation(format!("한 번에 {MAX_RANGE}개까지만 읽을 수 있습니다"), None));
    }
    Ok(get_game_journal_lookup(&state.db, &session_id, from_seq, to_seq).await?)
}

/// 세션의 대화 목록 (다시 보기에서 메시지를 읽을 대화를 고르기 위함).
#[tauri::command]
#[specta::specta]
pub async fn read_conversations(state: State<'_, AppState>, session_id: String) -> Result<Vec<GameConversations>, AppError> {
    let game_id = current_game_id(&state)?;
    owned_session(&state, &game_id, &session_id).await?;
    let all = get_all_game_conversations(&state.db, &game_id).await?;
    Ok(all.into_iter().filter(|c| c.session_id == session_id).collect())
}

/// 대화의 [from_seq, to_seq] 구간 메시지. meta에는 그때 나온 선택지와 적용된 변수 변경이 들어 있다.
#[tauri::command]
#[specta::specta]
pub async fn read_messages(state: State<'_, AppState>, conversation_id: String, from_seq: Option<u32>, to_seq: u32) -> Result<Vec<GameMessages>, AppError> {
    const MAX_RANGE: u32 = 1000;
    let game_id = current_game_id(&state)?;
    let all = get_all_game_conversations(&state.db, &game_id).await?;
    if !all.iter().any(|c| c.id == conversation_id) {
        return Err(AppError::not_found("이 게임의 대화가 아닙니다"));
    }
    if to_seq.saturating_sub(from_seq.unwrap_or(0)) > MAX_RANGE {
        return Err(AppError::validation(format!("한 번에 {MAX_RANGE}개까지만 읽을 수 있습니다"), None));
    }
    Ok(get_game_messages_lookup(&state.db, &conversation_id, from_seq, to_seq).await?)
}

/// 게임 엔진에 커맨드 하나를 보낸다. iframe SDK의 game.move() 등이 여기로 온다.
#[tauri::command]
#[specta::specta]
pub async fn dispatch_command(state: State<'_, AppState>, command: GameCommand) -> Result<GameResponse, AppError> {
    let game_id = current_game_id(&state)?;
    let mut guard = state.active_runtime.lock().await;
    let runtime = guard.as_mut().ok_or_else(AppError::game_no_session)?;

    // 게임이 쓰지 않는 기능(capabilities)의 커맨드는 GAME_ACTION_BLOCKED(details.reason = "capabilityDisabled")로 거절된다.
    let response = runtime.dispatch(command.clone())?;

    // 일지/이벤트 로그/세션 저장을 한꺼번에 (npc_say의 AI 변수 변경도 같은 함수를 쓴다).
    crate::game::journal::record(&state.db, runtime, &game_id, Some(&command), &response.events).await?;

    Ok(response)
}

/// 책갈피 저장. 지금 일지 위치(head_seq)와 상태 스냅샷을 현재 세션의 slot에 남긴다.
#[tauri::command]
#[specta::specta]
pub async fn save_write(state: State<'_, AppState>, slot: i64, data: serde_json::Value) -> Result<(), AppError> {
    let game_id = current_game_id(&state)?;
    check_slot(slot)?;

    let guard = state.active_runtime.lock().await;
    let runtime = guard.as_ref().ok_or_else(AppError::game_no_session)?;
    let s = runtime.state();

    // engine은 참고용 스냅샷 — 슬롯 목록 UI에 "마지막 저장 시점이 어땠는지" 보여주는 용도.
    // 게임(iframe)이 준 데이터는 custom에만 담는다.
    let snapshot_value = serde_json::json!({ "engine": s, "custom": data });
    let snapshot_str = serde_json::to_string(&snapshot_value)?;
    if snapshot_str.len() > MAX_SAVE_BYTES {
        return Err(AppError::validation("세이브가 256KB를 초과했습니다", None));
    }

    let input = GameSaveInput {
        name: None,
        seq: s.head_seq,
        session_id: s.session_id.clone(),
        slot_number: slot,
        current_scene_id: s.current_scene_id.clone(),
        current_map_id: Some(s.map_id.clone()),
        current_zone_id: s.zone_id.clone(),
        current_location_id: s.location_id.clone(),
        current_weather_id: Some(s.weather_id.clone()),
        game_time: s.game_time,
        state_snapshot: snapshot_value,
    };
    upsert_game_save(&state.db, &game_id, &input, &snapshot_str).await?;
    Ok(())
}

/// 현재 세션의 slot 책갈피에서 게임이 저장해둔 custom 데이터만 돌려준다 (engine 상태는 노출 안 함).
#[tauri::command]
#[specta::specta]
pub async fn save_read(state: State<'_, AppState>, slot: i64) -> Result<Option<serde_json::Value>, AppError> {
    check_slot(slot)?;
    let session_id = {
        let guard = state.active_runtime.lock().await;
        guard.as_ref().ok_or_else(AppError::game_no_session)?.state().session_id.clone()
    };
    let save: GameSave = match get_game_save(&state.db, &session_id, slot).await {
        Ok(s) => s,
        Err(sqlx::Error::RowNotFound) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    Ok(save.state_snapshot.get("custom").cloned())
}

/// 이 게임의 모든 책갈피(여러 세션에 걸쳐). 책갈피 화면용.
#[tauri::command]
#[specta::specta]
pub async fn save_list(state: State<'_, AppState>) -> Result<Vec<GameSave>, AppError> {
    let game_id = current_game_id(&state)?;
    Ok(get_all_game_save(&state.db, &game_id).await?)
}

/// 얇은 wrapper — 실제 로직은 game::npc::say. 여기서는 Tauri가 주입하는
/// AppHandle/State들을 받아서 그대로 넘기기만 한다.
#[tauri::command]
#[specta::specta]
pub async fn npc_say(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: State<'_, SettingsState<AppSetting>>,
    character_id: String,
    input: String,
    on_event: Channel<crate::models::ai_service::StreamPayload>,
) -> Result<crate::game::npc::NpcReply, AppError> {
    // on_event: 기다리는 동안 보여줄 미리보기(태그를 걸러낸 글). 최종 대사·연출은 반환값
    crate::game::npc::say(app, &state, &settings, character_id, input, Some(on_event)).await
}

/// 씬 입력: 씬이 열려 있는 동안 AI가 서술자로 장면을 이어간다. 선택지의 ChoiceMade 이벤트에 continuesScene=true가
/// 있으면 그 선택지 문구를 input으로 이어서 부르고, 자유 입력도 같은 경로다.
/// 씬이 열려 있지 않거나 엔딩 씬이거나 aiContinue가 꺼진 씬이면 에러다 (RenderState.sceneChat으로 미리 알 수 있다).
#[tauri::command]
#[specta::specta]
pub async fn scene_say(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: State<'_, SettingsState<AppSetting>>,
    input: String,
    on_event: Channel<crate::models::ai_service::StreamPayload>,
) -> Result<crate::game::npc::NpcReply, AppError> {
    crate::game::npc::scene_say(app, &state, &settings, input, Some(on_event)).await
}

// =======================================
// 세션별 설정 · 컨텍스트 (오른쪽 패널)
// =======================================

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameSessionOptions {
    pub api_profile_id: Option<String>,
    pub prompt_preset_id: Option<String>,
    /// 이 플레이의 모델 (None = API의 기본 모델, ADR-020)
    pub model: Option<String>,
    pub player_id: String,
}

/// 세션의 AI 설정과 플레이어 페르소나
#[tauri::command]
#[specta::specta]
pub async fn get_game_session_options(state: State<'_, AppState>, session_id: String) -> Result<GameSessionOptions, AppError> {
    let session = get_game_player_sessions(&state.db, &session_id).await?;
    let (api_profile_id, prompt_preset_id) = crate::db::game::get_game_session_options(&state.db, &session_id).await?;
    let model = crate::db::game::get_game_session_model(&state.db, &session_id).await?;
    Ok(GameSessionOptions {
        api_profile_id,
        prompt_preset_id,
        model,
        player_id: session.player_id,
    })
}

/// 세션의 AI 설정·플레이어 페르소나 바꾸기. 비운 값은 앱 기본값.
/// 플레이어를 바꿨고 지금 열린 세션이면 런타임을 다시 불러와 바로 반영한다.
#[tauri::command]
#[specta::specta]
pub async fn set_game_session_options(
    state: State<'_, AppState>,
    session_id: String,
    api_profile_id: Option<String>,
    prompt_preset_id: Option<String>,
    player_id: Option<String>,
    model: Option<String>,
) -> Result<(), AppError> {
    crate::db::game::set_game_session_options(&state.db, &session_id, api_profile_id.as_deref(), prompt_preset_id.as_deref(), model.as_deref()).await?;
    if let Some(player) = player_id.filter(|s| !s.is_empty()) {
        let changed = {
            let guard = state.active_runtime.lock().await;
            guard
                .as_ref()
                .map(|r| r.state().session_id == session_id && r.state().player_id != player)
                .unwrap_or(false)
        };
        crate::db::game::set_game_session_player(&state.db, &session_id, &player).await?;
        if changed {
            let game_id = current_game_id(&state)?;
            open_session(&state, &game_id, &session_id, DEFAULT_HISTORY).await?;
        }
    }
    Ok(())
}

/// 게임 세션의 컨텍스트 사용량: 이번에 앱을 켠 뒤 마지막으로 보낸 프롬프트 기준
#[tauri::command]
#[specta::specta]
pub async fn game_context_usage(
    state: State<'_, AppState>,
    settings: State<'_, SettingsState<AppSetting>>,
    session_id: String,
) -> Result<crate::commands::chat::ContextUsage, AppError> {
    use crate::models::ai_service::PromptRole;
    use crate::service::{
        chat::chat_manager::{current_conv_request, LAST_GAME_PROMPT},
        prompt::prompt_manager::{chat_options_from, context_limit_from, PromptManager, DEFAULT_CONTEXT_LIMIT as CONTEXT_LIMIT},
    };
    let (_, preset) = crate::db::game::get_game_session_options(&state.db, &session_id).await?;
    let fallback = current_conv_request(&settings).map(|r| r.preset_id).unwrap_or_default();
    let preset_id = preset.filter(|s| !s.is_empty()).or(Some(fallback).filter(|s| !s.is_empty()));
    let loaded = PromptManager::load_preset(&state.db, preset_id.as_deref())
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    let limit = loaded.as_ref().and_then(|p| context_limit_from(&p.preset)).unwrap_or(CONTEXT_LIMIT);
    let reserved = loaded.as_ref().and_then(|p| chat_options_from(&p.preset).max_tokens).unwrap_or(0);
    let prompt = LAST_GAME_PROMPT.lock().ok().and_then(|m| m.get(&session_id).cloned());
    let (mut system, mut history, mut history_count) = (0u32, 0u32, 0u32);
    if let Some(p) = &prompt {
        for m in &p.messages {
            let t = m.token.max(0) as u32;
            if m.role == PromptRole::System {
                system += t;
            } else {
                history += t;
                history_count += 1;
            }
        }
    }
    Ok(crate::commands::chat::ContextUsage {
        used: prompt.map(|_| system + history),
        system,
        history,
        history_count,
        limit,
        reserved,
        preset_name: loaded.as_ref().map(|p| p.preset.name.clone()),
        preset_id,
        // 게임 대사에는 플러그인 프롬프트 훅을 쓰지 않는다
        plugins: vec![],
        // 자동 회상은 채팅에서만 (게임 대사에는 쓰지 않음)
        memory_recall: None,
        summary: None,
    })
}

/// 게임 메시지 고치기 (AI 응답은 줄로 다시 나눠 화면·지난 대사에 반영)
#[tauri::command]
#[specta::specta]
pub async fn edit_game_message(state: State<'_, AppState>, message_id: String, content: String) -> Result<crate::game::npc::EditedGameMessage, AppError> {
    crate::game::npc::edit_message(&state, message_id, content).await
}

/// 마지막 AI 응답 다시 만들기 (그 응답이 바꾼 변수·관계는 되돌림)
#[tauri::command]
#[specta::specta]
pub async fn regenerate_game_reply(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: State<'_, SettingsState<AppSetting>>,
    message_id: String,
) -> Result<crate::game::npc::NpcReply, AppError> {
    crate::game::npc::regenerate_reply(app, &state, &settings, message_id).await
}

/// StoryPlan(각본가) 갱신. 체크포인트(RenderState.storyPlanDue)일 때 부르면 AI가 새 플랜을 세워 저장한다.
/// StoryPlan이 꺼진 게임이거나 체크포인트가 아니면(force가 아닌 한) AI를 부르지 않고 null.
/// 플랜은 조언일 뿐 상태를 바꾸지 않는다. 제안을 실제로 올리려면 stage_story_suggestion.
#[tauri::command]
#[specta::specta]
pub async fn refresh_story_plan(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: State<'_, SettingsState<AppSetting>>,
    tasks: State<'_, std::sync::Arc<crate::task::TaskManager>>,
    force: Option<bool>,
    auto: Option<bool>,
) -> Result<Option<crate::game::story_plan::StoryPlanRecord>, AppError> {
    use tauri::Manager;
    let game_id = current_game_id(&state)?;
    let profile_id = settings
        .0
        .lock()
        .map_err(|_| AppError::internal("settings lock poisoned"))?
        .selected_api_profile
        .clone()
        .unwrap_or_default();
    let ai = app
        .try_state::<std::sync::Arc<crate::service::ai::AiService>>()
        .ok_or_else(|| AppError::internal("AiService가 초기화되지 않았습니다"))?
        .inner()
        .clone();
    // 유료 호출은 TaskManager 작업 안에서 (기능 id·실행 주체·이름이 기록된다). 자동 호출(auto)은 화면의 자동 갱신 스위치가 켜졌을 때만 온다.
    let job = crate::task::CreateTaskRequest::inline(
        crate::task::TaskType::GameDialogue,
        crate::task::TaskOwner::Game,
        "game.storyplan",
        "각본가",
        auto.unwrap_or(false),
    )
    .owner_ref(&game_id);
    let tasks = tasks.inner().clone();
    crate::game::story_plan::refresh(state.inner(), &game_id, profile_id, force.unwrap_or(false), |request| async move {
        tasks.run_inline(job, |_| async move { Ok(ai.generate_once(request).await?.content) }).await
    })
    .await
}

/// 지금 세션의 가장 최근 플랜 (없으면 null). 호스트 UI용 — 게임 iframe에 주면 스포일러가 된다.
#[tauri::command]
#[specta::specta]
pub async fn get_story_plan(state: State<'_, AppState>) -> Result<Option<crate::game::story_plan::StoryPlanRecord>, AppError> {
    current_game_id(&state)?;
    let (session_id, head_seq) = {
        let guard = state.active_runtime.lock().await;
        let runtime = guard.as_ref().ok_or_else(|| AppError::not_found("진행 중인 세션이 없습니다"))?;
        (runtime.state().session_id.clone(), runtime.state().head_seq)
    };
    crate::game::story_plan::latest(&state.db, &session_id, head_seq).await
}

/// 최근 플랜의 제안(kind = "event" | "scene", targetId)을 무대에 올린다. fire_event/enter_scene과 같은 검증을 다시 하고,
/// 막히면 GAME_ACTION_BLOCKED(details.reason: notSuggested, notPlanUsable, conditionNotMet, alreadyQueued, alreadyStaged, storyPlanDisabled).
/// 응답은 dispatch_command와 같은 GameResponse다.
#[tauri::command]
#[specta::specta]
pub async fn stage_story_suggestion(state: State<'_, AppState>, kind: String, target_id: String) -> Result<GameResponse, AppError> {
    let game_id = current_game_id(&state)?;
    crate::game::story_plan::stage_suggestion(state.inner(), &game_id, &kind, &target_id).await
}
