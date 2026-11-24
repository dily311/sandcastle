//! 채팅 흐름: 컨텍스트 수집(ContextManager) → 프롬프트 조립(PromptManager) → AI 호출(AiService)
//!
//! 대화 화면의 한 차례(turn/peek)는 ChatStream 작업(chat_task.rs)의 Executor 안에서 실행된다.
//! 커맨드는 작업을 제출하고 결과를 기다릴 뿐이고, ChatManager는 작업 상태를 바꾸지 않는다.
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use anyhow::Result;
use tauri::{ipc::Channel, AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use serde_json::{json, Map, Value};

use crate::{
    db::{
        chat_ui,
        conversation::{get_character, get_conversation, get_persona, resolve_conv_persona},
    },
    error::AppError,
    models::{
        ai_service::{ChatContext, ContextRequest, DialogueDoneEvent, GenerationType, PromptMessage, PromptRequest, StreamPayload},
        character_image::CharacterImage,
        conversation::{ChatTurn, ConversationMode, Message, MessageInput},
        regex::{RegexPlacement, RegexTarget},
        settings::AppSetting,
    },
    service::{
        ai::{AiRequest, AiResponse, AiService},
        chat::{
            chat_images::{image_rules, sanitize_cg_tags, strip_cg_tags, FEATURE_KEY as IMAGES_FEATURE},
            context_manager::{is_hidden, ContextManager},
            phone::{self as phone_sides, Side},
            regex_processor::{RegexMacros, RegexProcessor},
        },
        prompt::PromptManager,
    },
    state::AppState,
    sys::settings::SettingsState,
};

const EVENT_DIALOGUE_DONE: &str = "chat:dialogue:done";

use crate::service::prompt::prompt_manager::{DEFAULT_CONTEXT_LIMIT, DEFAULT_WORLD_INFO_RATIO, HISTORY_FETCH_LIMIT};

pub struct ChatRequest {
    api_profile_id: String,
    /// 폰의 기본 API를 따라갈 때 폰 설정의 모델 (ADR-020)
    model: Option<String>,
    pub(crate) preset_id: String,
    phone_mode_prompt: String,
    /// 전역 추론 강도 (5-E-29-3): 자물쇠를 풀어 둔 대화가 쓴다
    reasoning_effort: Option<String>,
}

/// AI가 읽고 바꿀 대화 변수 정의 (5-F-4-4, ADR-029): 상태창·변수 플러그인과 그 안의 "AI 변수" 스위치가 켜졌고
/// 캐릭터에 AI에게 보이는 정의가 있을 때만. 아니면 빈 목록 (프롬프트·태그 읽기 모두 하지 않음)
async fn ai_var_defs(app: &AppHandle, db: &crate::db::conn::DbPool, character_id: &str) -> Vec<crate::models::conversation::VarDef> {
    if !crate::sys::plugins::is_enabled(app, crate::sys::plugins::id::STATUS) {
        return vec![];
    }
    let on = app
        .try_state::<SettingsState<crate::models::settings::PluginSetting>>()
        .and_then(|s| s.0.lock().ok().map(|g| g.status.ai_vars))
        .unwrap_or(false);
    if !on || character_id.is_empty() {
        return vec![];
    }
    let defs = crate::db::conversation::get_character_var_defs(db, character_id).await.unwrap_or_default();
    if super::vars::any_visible(&defs) {
        defs
    } else {
        vec![]
    }
}

/// 전역 변수 ({{getglobalvar::x}}, ADR-029): 슬래시 `/setglobalvar`가 쓰는 플러그인 설정 `quickReply.globals`
fn with_global_vars(app: &AppHandle, ctx: &mut crate::models::ai_service::ChatContext) {
    let globals = app
        .try_state::<SettingsState<crate::models::settings::PluginSetting>>()
        .and_then(|s| s.0.lock().ok().map(|g| g.quick_reply.globals.clone()))
        .unwrap_or_default();
    if let serde_json::Value::Object(m) = &mut ctx.extra_data {
        m.insert("globalVariables".into(), serde_json::json!(globals));
    }
}

/// 폰 대화면 폰 플러그인의 기본 API·프리셋을 앱 기본값보다 앞에 둔다 (대화에 고른 값이 가장 먼저)
fn phone_defaults(app: &AppHandle, mode: &str, req: &mut ChatRequest) {
    if mode != "phone" {
        return;
    }
    let Some(state) = app.try_state::<SettingsState<crate::models::settings::PluginSetting>>() else {
        return;
    };
    let Ok(p) = state.0.lock().map(|s| s.phone.clone()) else { return };
    if let Some(id) = p.api_profile_id.filter(|s| !s.is_empty()) {
        req.api_profile_id = id;
        req.model = Some(p.model.trim().to_string()).filter(|m| !m.is_empty());
    }
    if let Some(id) = p.preset_id.filter(|s| !s.is_empty()) {
        req.preset_id = id;
    }
}

/// 대화에서 고른 모델 (ADR-020). 대화에 API 프로필이 정해져 있을 때만 쓴다 — 앱 기본 API를 따라가는 대화에는 모델 이름이 딸려 있지 않다
/// 읽기 모드인 대화는 AI 응답을 만들지 않는다 — 보내기·다시 생성·이어 쓰기·스와이프·대필·/gen (5-E-6-2, ADR-031)
pub(crate) fn reject_read_mode(conv: &crate::models::conversation::Conversation) -> std::result::Result<(), AppError> {
    if conv.read_mode {
        return Err(AppError::validation("읽기 모드인 대화 — 채팅 모드로 바꾼 뒤 생성할 수 있음", None));
    }
    Ok(())
}

/// 추론 강도 (ADR-028): 대화에 값이 있으면(자물쇠) 그것 — "auto"로 잠근 대화도 전역을 따르지 않는다 —, 없으면 전역.
/// "auto"·빈 값 = 프리셋·공급자 기본 (None)
fn conv_effort(meta: Option<&Value>, global: Option<&str>) -> Option<String> {
    let locked = meta.and_then(|m| m.get("reasoningEffort")).and_then(Value::as_str).map(str::trim);
    locked.or(global.map(str::trim)).filter(|s| !s.is_empty() && *s != "auto").map(str::to_string)
}

fn conv_model(conv: &crate::models::conversation::Conversation) -> Option<String> {
    conv.api_profile_id.as_deref().filter(|s| !s.is_empty())?;
    conv.model.clone().filter(|m| !m.trim().is_empty())
}

/// 대화에 API가 없어 폰·앱의 기본 API를 따라갈 때, 폰 설정의 모델 (ADR-020)
fn fallback_model(conv: &crate::models::conversation::Conversation, model: &Option<String>) -> Option<String> {
    if conv.api_profile_id.as_deref().is_some_and(|s| !s.is_empty()) {
        return None;
    }
    model.clone()
}

pub fn current_conv_request(state: &SettingsState<AppSetting>) -> Result<ChatRequest> {
    let guard = state.0.lock().map_err(|_| AppError::internal("settings lock poisoned"))?;
    Ok(ChatRequest {
        api_profile_id: guard.selected_api_profile.clone().unwrap_or_default(),
        // 전역 모델 (5-E-29-3). 폰은 phone_defaults가 폰 설정의 것으로 바꾼다
        model: guard.selected_model.clone().filter(|m| !m.trim().is_empty()),
        preset_id: guard.selected_preset.clone(),
        phone_mode_prompt: guard.phone_mode_prompt.clone(),
        reasoning_effort: guard.reasoning_effort.clone(),
    })
}

/// 프롬프트에 들어가기 전 정규식 적용. 치환된 이번 입력을 돌려준다.
/// - 이번 입력(깊이 0): 저장용 + 프롬프트용 (저장 시 append_message가 적용한 것과 같은 결과)
/// - 대화 기록: 이미 저장용이 적용된 상태라 프롬프트용만 (깊이 = 최근부터 1, 2, …)
/// - 월드인포: 저장되지 않으므로 둘 다
fn apply_prompt_regex(regex: &RegexProcessor, context: &mut ChatContext, message: &str, message_targets: &[RegexTarget]) -> String {
    if regex.is_empty() {
        return message.to_string();
    }
    let both = [RegexTarget::Stored, RegexTarget::Prompt];
    let n = context.history.len();
    for (i, item) in context.history.iter_mut().enumerate() {
        if let Some(placement) = RegexPlacement::for_role(&item.role) {
            item.content = regex.apply(&item.content, placement, &[RegexTarget::Prompt], Some(n - i));
        }
    }
    for entry in context.world_info.iter_mut() {
        entry.content = regex.apply(&entry.content, RegexPlacement::WorldInfo, &both, None);
    }
    regex.apply(message, RegexPlacement::UserInput, message_targets, Some(0))
}

/// 캐릭터 이미지 코어 플러그인 (sys::plugins)
fn images_feature_on(app: &AppHandle) -> bool {
    crate::sys::plugins::is_enabled(app, IMAGES_FEATURE)
}

async fn load_enabled_images(app: &AppHandle, db: &crate::db::conn::DbPool, character_id: &str) -> Result<Vec<CharacterImage>> {
    if character_id.is_empty() {
        return Ok(vec![]);
    }
    let base = crate::commands::character_image::app_data_dir(app)?;
    let rows = crate::db::character_image::get_character_images(db, character_id, true).await?;
    // 캐릭터가 판을 골랐으면 그 판과 공통 그림만 (5-E-21)
    let version = crate::db::character_image::get_asset_version(db, character_id).await?;
    Ok(rows
        .into_iter()
        .filter(|r| in_version(&r.folder, version.as_deref()))
        .map(|r| CharacterImage::from_row(r, &base))
        .collect())
}

/// 고른 판(None = 전체)에서 이 그림을 쓰는가: 공통(폴더 없음)은 늘 쓴다
fn in_version(folder: &str, version: Option<&str>) -> bool {
    match version {
        None => true,
        Some(v) => folder.is_empty() || folder.eq_ignore_ascii_case(v),
    }
}

pub struct ChatManager;

impl ChatManager {
    /// 게임 대화: 컨텍스트는 GameRuntime(game::npc)이 만들어 넘긴다.
    pub async fn game_chat(
        &self,
        app: AppHandle,
        state: &State<'_, AppState>,
        settings: &State<'_, SettingsState<AppSetting>>,
        _game_conversation_id: String,
        message: String,
        context: ChatContext,
        on_delta: Option<&mut (dyn FnMut(&str) + Send)>,
        target: GameChatTarget,
    ) -> Result<String> {
        let chat_req = current_conv_request(settings)?;
        let game_model = target
            .api_profile_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .and(target.model.clone())
            .filter(|m| !m.trim().is_empty());
        let profile = target.api_profile_id.filter(|s| !s.is_empty()).unwrap_or(chat_req.api_profile_id);
        let preset = target.prompt_preset_id.filter(|s| !s.is_empty()).unwrap_or(chat_req.preset_id);
        // 스트리밍: 받은 조각은 호출자(game::npc)가 태그를 걸러 화면으로 보낸다
        let streaming = on_delta.is_some();
        let mut noop = |_: &str| {};
        let sink: &mut (dyn FnMut(&str) + Send) = match on_delta {
            Some(f) => f,
            None => &mut noop,
        };
        // 작업으로 실행: 실행 기록·사용량에 남고, 실행 중 목록에서 멈출 수 있다
        let tasks = app
            .try_state::<Arc<crate::task::TaskManager>>()
            .ok_or_else(|| AppError::internal("TaskManager가 초기화되지 않았습니다"))?
            .inner()
            .clone();
        let job = crate::task::CreateTaskRequest::inline(
            crate::task::TaskType::GameDialogue,
            crate::task::TaskOwner::Game,
            crate::sys::plugins::id::GAME,
            "게임 대사",
            false,
        )
        .owner_ref(target.session_id.clone());
        let (response, prompt) = tasks
            .run_inline(job, |cancel| async move {
                self.generate_with(
                    &app,
                    &state.db,
                    profile,
                    game_model,
                    None,
                    Some(preset),
                    context,
                    message,
                    GenerationType::Normal,
                    &cancel,
                    streaming,
                    sink,
                )
                .await
                .map_err(crate::commands::chat::into_app_error)
            })
            .await?;
        if let Ok(mut map) = LAST_GAME_PROMPT.lock() {
            map.insert(target.session_id, prompt);
        }
        Ok(response.content)
    }

    /// 프롬프트를 만들고 AI를 부른다. 보낸 프롬프트도 함께 돌려준다 ("보낸 컨텍스트 보기").
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn generate_with(
        &self,
        app: &AppHandle,
        db: &crate::db::conn::DbPool,
        profile_id: String,
        model: Option<String>,
        effort: Option<String>,
        preset_id: Option<String>,
        context: ChatContext,
        message: String,
        generation_type: GenerationType,
        cancel: &CancellationToken,
        stream: bool,
        on_delta: &mut (dyn FnMut(&str) + Send),
    ) -> Result<(AiResponse, PromptMessage)> {
        let mut prompt = PromptManager
            .build_with_db(
                db,
                PromptRequest {
                    context,
                    user_message: message,
                    preset_id,
                    context_limit: DEFAULT_CONTEXT_LIMIT,
                    world_info_ratio: DEFAULT_WORLD_INFO_RATIO,
                    generation_type,
                },
            )
            .await?;
        prompt.chat_options.stream = stream;
        // 대화에서 고른 추론 강도가 프리셋 값보다 앞선다 (5-E-7). 모델이 받지 않는 값이면 AiService가 거른다
        if let Some(e) = effort {
            prompt.chat_options.reasoning_effort = Some(e);
        }

        let ai = app
            .try_state::<Arc<AiService>>()
            .ok_or_else(|| AppError::internal("AiService가 초기화되지 않았습니다"))?
            .inner()
            .clone();

        let response = ai
            .generate(
                AiRequest {
                    profile_id,
                    model,
                    prompt: prompt.clone(),
                },
                cancel,
                |d| on_delta(d),
            )
            .await?;
        Ok((response, prompt))
    }
}

// =======================================
// 대화 화면의 한 차례 (보내기 / 이어쓰기 / 다시 생성)
// =======================================

/// 게임 대화를 어느 세션 설정으로 부를지 (비운 값은 앱 기본값)
pub struct GameChatTarget {
    pub session_id: String,
    pub api_profile_id: Option<String>,
    pub prompt_preset_id: Option<String>,
    /// 이 플레이에서 고른 모델 (ADR-020). 플레이에 API가 정해져 있을 때만 쓴다
    pub model: Option<String>,
}

/// 게임 세션별 마지막으로 보낸 프롬프트 (컨텍스트 사용량 표시용, 앱을 끄면 사라짐)
pub(crate) static LAST_GAME_PROMPT: LazyLock<Mutex<HashMap<String, PromptMessage>>> = LazyLock::new(Default::default);

/// 한 차례의 종류
pub enum TurnKind {
    /// 사용자 입력을 저장하고 응답을 만든다. 입력이 비었으면 저장 없이 이어서 응답 (이어쓰기)
    /// 두 번째 값은 붙인 이미지 경로 (글 없이 이미지만 보내도 됨)
    /// 세 번째 값: 대필을 거친 글이면 true (사용자 메시지 meta.ghost)
    Send(String, Vec<String>, bool),
    /// 마지막 AI 메시지를 다시 만든다. `swipe`면 스와이프로 쌓고, 아니면 스와이프를 버리고 바꾼다.
    /// `extra`: 이번 생성에만 붙이는 지시 ("지시와 함께 다시 생성")
    /// `auto`: 자동 재생성(3-B-4)이 낸 것이면 묶은 메시지·시도 횟수 — 그 메시지가 바뀌었으면 만들지 않고, 멈추면 받은 글을 버린다
    Regenerate {
        swipe: bool,
        extra: Option<String>,
        auto: Option<super::rewrite::AutoCtx>,
    },
    /// 이미 저장된 마지막 사용자 메시지에 응답한다 (보내기 작업을 다시 시도할 때: 입력은 첫 시도에 저장됨)
    Reply,
    /// 마지막 AI 메시지에 이어 쓴다 ("이어서 응답 받기", 5-E-26). 빈 보내기(`Send("")`)는 이어 쓰지 않고 새 AI 메시지를 만든다
    Continue,
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn meta_obj(meta: &Option<Value>) -> Map<String, Value> {
    meta.as_ref().and_then(Value::as_object).cloned().unwrap_or_default()
}

/// 다시 생성(스와이프): 기존 내용을 첫 스와이프로 보존하고 새 응답을 뒤에 쌓는다
fn push_swipe(old: &Message, content: &str, info: &Value, images: &Value) -> Value {
    let mut m = meta_obj(&old.meta);
    let old_lint = m.remove("lint"); // 새 응답의 linter 결과는 저장한 뒤 따로 넣는다 (원본의 것은 스와이프에 보관)
    let old_vars = m.remove("varChanges"); // AI 변수 몫도 스와이프마다 (5-F-4-4)
    m.remove("rewriteNote");
    let mut swipes = m.get("swipes").and_then(Value::as_array).cloned().unwrap_or_default();
    if swipes.is_empty() {
        let mut first = Map::new();
        if let Some(i) = m.get("info") {
            first.insert("info".into(), i.clone());
        }
        if let Some(i) = m.get("images") {
            first.insert("images".into(), i.clone());
        }
        if let Some(l) = old_lint {
            first.insert("lint".into(), l);
        }
        if let Some(v) = old_vars {
            first.insert("varChanges".into(), v);
        }
        swipes.push(json!({ "content": old.content, "meta": first, "createdAt": old.created_at, "updatedAt": old.updated_at }));
    }
    let t = now_ms();
    swipes.push(json!({ "content": content, "meta": { "info": info, "images": images }, "createdAt": t, "updatedAt": t }));
    m.insert("swipeId".into(), json!(swipes.len() - 1));
    m.insert("swipes".into(), Value::Array(swipes));
    m.insert("info".into(), info.clone());
    m.insert("images".into(), images.clone());
    Value::Object(m)
}

/// 이어쓰기로 받은 글을 원래 글 뒤에 붙인다. 문장이 끝난 자리(마침표·따옴표 등) 바로 뒤에 공백 없이 이어 오면 한 칸 띄운다.
/// 단어 중간에서 끊긴 글은 그대로 붙인다 (공급자는 보통 필요한 공백을 스스로 넣는다)
fn join_continue(old: &str, add: &str) -> String {
    let ends_sentence = old
        .chars()
        .last()
        .is_some_and(|c| matches!(c, '.' | '!' | '?' | '…' | '"' | '”' | '\'' | '’' | '」' | '』' | ')' | '*' | '~'));
    let starts_bare = add.chars().next().is_some_and(|c| !c.is_whitespace());
    if ends_sentence && starts_bare {
        format!("{old} {add}")
    } else {
        format!("{old}{add}")
    }
}

/// 이어쓰기: 마지막 AI 메시지 뒤에 붙인 글을 지금 보고 있는 스와이프에도 반영한다.
/// 생성 정보는 이번 것으로, 쓴 그림은 앞의 것 뒤에 더한다. 앞 조각 기준이던 linter 결과는 위치가 맞지 않아 지운다
fn continue_meta(old: &Message, joined: &str, info: &Value, added: &[crate::models::character_image::ChatImageRef]) -> Value {
    let mut m = meta_obj(&old.meta);
    m.remove("lint");
    m.remove("rewriteNote");
    let mut images = m.get("images").and_then(Value::as_array).cloned().unwrap_or_default();
    images.extend(added.iter().filter_map(|r| serde_json::to_value(r).ok()));
    let images = Value::Array(images);
    let sid = m.get("swipeId").and_then(Value::as_u64);
    if let Some(s) = sid.and_then(|i| m.get_mut("swipes").and_then(Value::as_array_mut).and_then(|a| a.get_mut(i as usize))) {
        s["content"] = json!(joined);
        s["updatedAt"] = json!(now_ms());
        if !s["meta"].is_object() {
            s["meta"] = json!({});
        }
        s["meta"]["info"] = info.clone();
        s["meta"]["images"] = images.clone();
        if let Some(sm) = s["meta"].as_object_mut() {
            sm.remove("lint");
        }
    }
    m.insert("info".into(), info.clone());
    m.insert("images".into(), images);
    Value::Object(m)
}

/// 다시 생성(처음부터): 스와이프를 버리고 내용만 바꾼다. 숨김·챕터 같은 표시는 남긴다
fn replace_meta(old: &Message, info: &Value, images: &Value) -> Value {
    let mut m = meta_obj(&old.meta);
    m.remove("swipes");
    m.remove("swipeId");
    m.remove("reasoning");
    m.remove("lint");
    m.remove("rewriteNote");
    m.remove("varChanges");
    m.insert("info".into(), info.clone());
    m.insert("images".into(), images.clone());
    Value::Object(m)
}

/// 지시 글 정리: 앞뒤 공백을 자르고 비면 없음, 1000자까지
pub fn clean_extra(e: &str) -> Option<String> {
    let t: String = e.trim().chars().take(MAX_EXTRA_CHARS).collect();
    (!t.is_empty()).then_some(t)
}

pub const MAX_EXTRA_CHARS: usize = 1000;

/// 조립된 프롬프트를 시스템(규칙·캐릭터·프리셋·월드인포·노트 등)과 실제 대화 기록으로 나눠 센다.
pub(crate) fn count_prompt_usage(p: &PromptMessage) -> (u32, u32, u32) {
    use crate::models::ai_service::PromptRole;
    let (mut system, mut history, mut history_count) = (0u32, 0u32, 0u32);
    for m in &p.messages {
        let t = m.token.max(0) as u32;
        let is_chat = match m.source.as_ref().map(|s| s.kind.as_str()) {
            Some("chat" | "user_input" | "recall") => true,
            Some(_) => false,
            None => m.role != PromptRole::System,
        };
        if is_chat {
            history += t;
            history_count += 1;
        } else {
            system += t;
        }
    }
    (system, history, history_count)
}

struct EnrichedContext {
    images: Vec<CharacterImage>,
    auto_image: bool,
    summary_use: Option<crate::service::summary::SummaryUse>,
}

#[allow(clippy::too_many_arguments)]
async fn enrich_context(
    app: Option<&AppHandle>,
    db: &crate::db::conn::DbPool,
    settings: &SettingsState<AppSetting>,
    conv: &crate::models::conversation::Conversation,
    fallback: &ChatRequest,
    other: &Side,
    preset_prompts: &crate::service::app_prompts::PresetPrompts,
    var_defs: &[crate::models::conversation::VarDef],
    base_vars: &Map<String, Value>,
    context: &mut ChatContext,
) -> Result<EnrichedContext> {
    // 대화별 메모·작가 노트 (오른쪽 패널)
    crate::service::chat::notes::apply(context, conv.meta.as_ref());

    // 대화 요약 (3-D, ADR-016): 켜져 있고 이 대화에서 끄지 않았으면 시스템 규칙 끝에 (history에 넣으면 토큰 한도에서 가장 먼저 잘림)
    let summary_on = app.is_some_and(|a| crate::sys::plugins::is_enabled(a, crate::sys::plugins::id::SUMMARY));
    let summary_use = if summary_on && !crate::service::summary::disabled_for(conv.meta.as_ref()) {
        match crate::service::summary::block_for(db, &conv.id).await {
            Some((block, used)) => {
                context.system_rules.push(block);
                Some(used)
            }
            None => None,
        }
    } else {
        None
    };

    if let Some(prompt) = ConversationMode::parse_or_default(&conv.mode).system_prompt(&fallback.phone_mode_prompt) {
        context.system_rules.push(prompt);
    }

    let images_on = conv.images_enabled && app.is_some_and(images_feature_on);
    let images = match (app, other.character_id()) {
        (Some(a), Some(cid)) if images_on => load_enabled_images(a, db, cid).await?,
        _ => vec![],
    };
    let group_cg = settings.0.lock().map(|s| s.cg_group_variants).unwrap_or(false);
    let images = if group_cg { super::chat_images::group_variants(images) } else { images };
    match image_rules(&images, preset_prompts) {
        Some(rules) => {
            context.image_prompt = Some(rules);
            context.image_keys = super::chat_images::image_keys(&images);
        }
        None => {
            for item in context.history.iter_mut() {
                item.content = strip_cg_tags(&item.content);
            }
        }
    }

    // 유틸 지시문은 프리셋 블록의 매크로 자리에만 들어간다 (ADR-030 추가: {{auto_image}}·{{multi_char}}·{{vars}})
    // 이미지 자동 생성: 글쓰기 AI에게 응답 끝 <image> 그림 프롬프트를 쓰게 한다 (기본 꺼짐)
    let auto_image = app.is_some_and(|a| crate::sys::plugins::is_enabled(a, crate::sys::plugins::id::IMAGE_AUTO));
    if auto_image {
        context
            .util_prompts
            .insert("auto_image".into(), crate::service::app_prompts::auto_image(preset_prompts));
    }
    // AI 변수 (5-F-4-4): 보이는 변수의 지금 값과 <updateVar> 쓰는 법 (스위치가 켜졌을 때만 — 응답마다 토큰이 늚)
    if let Some(rule) = super::vars::prompt_rule(var_defs, base_vars, preset_prompts) {
        context.util_prompts.insert("vars".into(), rule);
    }

    Ok(EnrichedContext {
        images,
        auto_image,
        summary_use,
    })
}

/// 지금 대화·설정 상태로 컨텍스트 사용량을 계산한다 (외부 호출·저장 부작용 없음).
pub async fn compute_context_usage(
    app: Option<&AppHandle>,
    db: &crate::db::conn::DbPool,
    settings: &SettingsState<AppSetting>,
    conversation_id: &str,
) -> std::result::Result<crate::commands::chat::ContextUsage, AppError> {
    use crate::error::ErrorCode;
    use crate::service::prompt::prompt_manager::{chat_options_from, context_limit_from};

    let conv = get_conversation(db, conversation_id).await?;
    let mut fallback = current_conv_request(settings)
        .map_err(|e| e.downcast::<AppError>().unwrap_or_else(|e| AppError::internal(e.to_string())))?;
    if let Some(a) = app {
        phone_defaults(a, &conv.mode, &mut fallback);
    }
    let character = if conv.character_id.is_empty() {
        None
    } else {
        get_character(db, &conv.character_id).await.ok()
    };
    let char_default = character.as_ref().and_then(|c| c.default_prompt_preset_id.clone());
    let preset_id = conv
        .prompt_preset_id
        .clone()
        .filter(|s| !s.is_empty())
        .or(char_default.filter(|s| !s.is_empty()))
        .or(Some(fallback.preset_id.clone()).filter(|s| !s.is_empty()));
    let preset = PromptManager::load_preset(db, preset_id.as_deref()).await?;
    let limit = preset.as_ref().and_then(|p| context_limit_from(&p.preset)).unwrap_or(DEFAULT_CONTEXT_LIMIT);
    let reserved = preset.as_ref().and_then(|p| chat_options_from(&p.preset).max_tokens).unwrap_or(0);
    let preset_name = preset.as_ref().map(|p| p.preset.name.clone());

    let Some(character) = character else {
        return Ok(crate::commands::chat::ContextUsage {
            used: None,
            system: 0,
            history: 0,
            history_count: 0,
            limit,
            reserved,
            preset_id,
            preset_name,
            plugins: vec![],
            memory_recall: None,
            summary: None,
        });
    };

    let persona_id = resolve_conv_persona(db, &conv.persona_id, &conv.character_id).await;
    let persona = get_persona(db, &persona_id).await.unwrap_or_else(|_| crate::models::conversation::Persona {
        id: persona_id.clone(),
        name: "User".into(),
        avatar: None,
        avatar_fit: None,
        description: String::new(),
        world_info_ids: vec![],
        tags: vec![],
        color: String::new(),
        is_system_default: false,
        created_at: 0,
        updated_at: 0,
    });
    let (me, other, char_phone) = phone_sides::sides(db, conv.meta.as_ref(), &character, &persona).await?;

    let mut context = match ContextManager
        .build_context(
            db,
            ContextRequest {
                conversation_id: conversation_id.to_string(),
                character_id: conv.character_id.clone(),
                persona_id: persona.id.clone(),
                history_limit: HISTORY_FETCH_LIMIT,
            },
        )
        .await
    {
        Ok(ctx) => ctx,
        Err(_) => {
            let messages = crate::db::conversation::get_messages(db, conversation_id, HISTORY_FETCH_LIMIT, None)
                .await
                .unwrap_or_default();
            let world_info =
                crate::db::conversation::get_all_conv_worldinfo_book_with_entries(db, conversation_id, &conv.character_id, &persona.id)
                    .await
                    .unwrap_or_default();
            let variables = conv.meta.as_ref().and_then(|m| m.get("variables")).cloned().unwrap_or_else(|| json!({}));
            ChatContext {
                char_name: Some(character.name.clone()),
                user_name: Some(persona.name.clone()),
                character_info: Some(character.description.clone()),
                example_dialogue: character.example_dialogue.clone(),
                main_prompt: character.system_prompt_override.clone().filter(|s| !s.trim().is_empty()),
                persona_info: persona.description.clone(),
                world_info,
                history: messages
                    .into_iter()
                    .filter(|m| !is_hidden(m.meta.as_ref()))
                    .map(|m| crate::models::ai_service::HistoryItem {
                        images: super::context_manager::attachments_of(m.meta.as_ref()),
                        role: m.role,
                        content: m.content,
                        source: None,
                    })
                    .collect(),
                extra_data: json!({ "variables": variables }),
                ..Default::default()
            }
        }
    };
    if let Some(a) = app {
        with_global_vars(a, &mut context);
    }
    if char_phone {
        phone_sides::apply_sides(&mut context, &me, &other);
    }

    let preset_prompts = crate::service::app_prompts::PresetPrompts::load(db, preset_id.as_deref().unwrap_or_default()).await;
    let var_defs = match app {
        Some(a) => ai_var_defs(a, db, &conv.character_id).await,
        None => vec![],
    };
    let base_vars = super::vars::current_vars(conv.meta.as_ref());
    let EnrichedContext { summary_use, .. } = enrich_context(
        app,
        db,
        settings,
        &conv,
        &fallback,
        &other,
        &preset_prompts,
        &var_defs,
        &base_vars,
        &mut context,
    )
    .await
    .map_err(|e| e.downcast::<AppError>().unwrap_or_else(|e| AppError::internal(e.to_string())))?;

    let regex = RegexProcessor::load(
        db,
        Some(&conv.character_id),
        preset_id.as_deref(),
        RegexMacros {
            char_name: other.name.clone(),
            user_name: me.name.clone(),
        },
    )
    .await?;
    let _ = apply_prompt_regex(&regex, &mut context, "", &[RegexTarget::Prompt]);

    let (used, system, history, history_count) = match PromptManager.build(
        PromptRequest {
            context,
            user_message: String::new(),
            preset_id: preset_id.clone(),
            context_limit: DEFAULT_CONTEXT_LIMIT,
            world_info_ratio: DEFAULT_WORLD_INFO_RATIO,
            generation_type: GenerationType::Normal,
        },
        preset.as_ref(),
    ) {
        Ok(prompt) => {
            let (s, h, c) = count_prompt_usage(&prompt);
            (Some(s + h), s, h, c)
        }
        Err(AppError::Validation {
            code: ErrorCode::AiContextOverflow,
            details,
            ..
        }) => {
            let fixed = details
                .as_ref()
                .and_then(|d| d.get("fixedTokens"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32;
            (Some(fixed), fixed, 0, 0)
        }
        Err(e) => return Err(e),
    };

    // 보내기 시점에만 도는 플러그인 훅·자동 회상은 현재 켜져 있을 때만 마지막 응답 기록을 참고용으로 붙인다
    let community_on = app.is_some_and(|a| crate::sys::plugins::is_enabled(a, crate::sys::plugins::id::COMMUNITY));
    let recall_on = app.is_some_and(|a| {
        crate::sys::plugins::is_enabled(a, crate::sys::plugins::id::MEMORY)
            && a.try_state::<SettingsState<crate::models::settings::PluginSetting>>()
                .and_then(|s| s.0.lock().ok().map(|g| g.memory.auto_recall))
                .unwrap_or(false)
            && crate::service::memory::is_eligible(conv.meta.as_ref().map(|m| m.to_string()).as_deref())
    });
    let mut plugins = vec![];
    let mut memory_recall = None;
    if community_on || recall_on {
        let recent = crate::db::conversation::get_messages(db, conversation_id, 10, None)
            .await
            .unwrap_or_default();
        if let Some(m) = recent.iter().rev().find(|m| m.role == "assistant" && !is_hidden(m.meta.as_ref())) {
            if community_on {
                plugins = m
                    .meta
                    .as_ref()
                    .and_then(|meta| meta.pointer("/info/pluginPrompt"))
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
            }
            if recall_on {
                memory_recall = m
                    .meta
                    .as_ref()
                    .and_then(|meta| meta.pointer("/info/memoryRecall"))
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
            }
        }
    }

    Ok(crate::commands::chat::ContextUsage {
        used,
        system,
        history,
        history_count,
        limit,
        reserved,
        preset_id,
        preset_name,
        plugins,
        memory_recall,
        summary: summary_use,
    })
}

/// 자동 회상에 필요한 것 (3-C-5)
struct RecallPlan {
    cfg: crate::models::settings::MemorySetting,
    profile: crate::models::ai_service::ApiProfiles,
    window: std::collections::HashSet<String>,
    reuse: Option<Vec<crate::service::memory::recall::MemoryRef>>,
    last_assistant: Option<String>,
}

/// 켜져 있고 쓸 수 있을 때만 계획을 만든다 (꺼져 있으면 아무것도 읽지 않음). 다시 만들기는 이전 응답에서 떠올린 참조를 이어 쓴다
async fn recall_setup(
    app: &AppHandle,
    db: &crate::db::conn::DbPool,
    conv: &crate::models::conversation::Conversation,
    kind: &TurnKind,
    target: &Option<Message>,
    context: &ChatContext,
    preset_id: &str,
) -> Option<RecallPlan> {
    use crate::service::memory::{self, recall::RecallMeta};
    if !crate::sys::plugins::is_enabled(app, crate::sys::plugins::id::MEMORY) {
        return None;
    }
    let cfg = app
        .try_state::<SettingsState<crate::models::settings::PluginSetting>>()?
        .0
        .lock()
        .ok()?
        .memory
        .clone();
    if !cfg.auto_recall || !memory::is_eligible(conv.meta.as_ref().map(|m| m.to_string()).as_deref()) {
        return None;
    }
    let profile_id = memory::check_config(&cfg).ok()?;
    let profile = app.state::<Arc<AiService>>().load_profile(&profile_id).await.ok()?;
    let reuse = match (kind, target) {
        (TurnKind::Regenerate { .. }, Some(old)) => {
            let prev: Option<RecallMeta> = old
                .meta
                .as_ref()
                .and_then(|m| m.pointer("/info/memoryRecall"))
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            Some(prev.filter(|m| m.status == "used").map(|m| m.hits).unwrap_or_default())
        }
        (TurnKind::Regenerate { .. }, None) => Some(vec![]),
        _ => None,
    };
    let loaded = PromptManager::load_preset(db, Some(preset_id)).await.ok().flatten();
    let limit = loaded
        .as_ref()
        .and_then(|p| crate::service::prompt::prompt_manager::context_limit_from(&p.preset))
        .unwrap_or(DEFAULT_CONTEXT_LIMIT);
    let hist: Vec<(String, usize)> = context
        .history
        .iter()
        .map(|h| (h.role.clone(), crate::service::prompt::token::count_tokens(&h.content)))
        .collect();
    let window = memory::recall::window_ids(db, &conv.id, &hist, limit as usize).await.unwrap_or_default();
    let last_assistant = context.history.last().filter(|h| h.role == "assistant").map(|h| h.content.clone());
    Some(RecallPlan {
        cfg,
        profile,
        window,
        reuse,
        last_assistant,
    })
}

impl ChatManager {
    /// 대화 화면의 한 차례. 메시지 저장까지 여기서 끝낸다 (프론트는 결과만 반영).
    /// - 캐릭터·페르소나·프리셋·API는 대화에 저장된 값, 없으면 앱 설정의 선택값
    /// - channel이 있으면 응답 조각을 그대로 흘려보낸다 (저장되는 최종본은 정규식·<cg> 정리 후)
    /// - 취소되면 받은 데까지 저장한다 (info.cancelled = true). 받은 게 없으면 취소 오류
    /// - task_id: 새로 저장하는 사용자 메시지의 meta.taskId (다시 시도할 때 입력을 두 번 저장하지 않게)
    #[allow(clippy::too_many_arguments)]
    pub async fn turn(
        &self,
        app: &AppHandle,
        db: &crate::db::conn::DbPool,
        settings: &SettingsState<AppSetting>,
        conversation_id: &str,
        kind: TurnKind,
        channel: Option<Channel<StreamPayload>>,
        cancel: &CancellationToken,
        task_id: Option<&str>,
    ) -> Result<ChatTurn> {
        // 이어 쓰기는 빈 보내기 + "이어 쓰기" 표시로 다룬다 (아래에서 이어 붙일 메시지를 찾는다)
        let (kind, continue_last) = match kind {
            TurnKind::Continue => (TurnKind::Send(String::new(), vec![], false), true),
            k => (k, false),
        };
        let conv = match get_conversation(db, conversation_id).await {
            Ok(c) => c,
            Err(sqlx::Error::RowNotFound) => return Err(AppError::not_found("대화를 찾을 수 없음").into()),
            Err(e) => return Err(AppError::from(e).into()),
        };
        reject_read_mode(&conv)?;
        let mut fallback = current_conv_request(settings)?;
        phone_defaults(app, &conv.mode, &mut fallback);
        let api_profile_id = conv
            .api_profile_id
            .clone()
            .filter(|s| !s.is_empty())
            .or(Some(fallback.api_profile_id.clone()).filter(|s| !s.is_empty()))
            .ok_or_else(|| AppError::validation("API를 먼저 고르세요 (대화 설정 또는 설정 > API 키)", None))?;
        let persona_id = resolve_conv_persona(db, &conv.persona_id, &conv.character_id).await;
        if conv.character_id.is_empty() {
            return Err(AppError::validation("캐릭터가 지워진 대화", None).into());
        }
        let character = get_character(db, &conv.character_id).await.map_err(AppError::from)?;
        let persona = get_persona(db, &persona_id).await.map_err(AppError::from)?;
        // 프리셋: 대화 → 캐릭터 기본 → 앱 설정
        let preset_id = conv
            .prompt_preset_id
            .clone()
            .filter(|s| !s.is_empty())
            .or(character.default_prompt_preset_id.clone().filter(|s| !s.is_empty()))
            .unwrap_or_else(|| fallback.preset_id.clone());
        // 프리셋이 고친 유틸 프롬프트 (ADR-030: 프리셋 > 전역 > 기본)
        let preset_prompts = crate::service::app_prompts::PresetPrompts::load(db, &preset_id).await;

        // 나/상대: 캐릭터 폰이면 주인 캐릭터와 연락처, 아니면 페르소나와 캐릭터
        let (me, other, char_phone) = phone_sides::sides(db, conv.meta.as_ref(), &character, &persona).await?;

        // 다시 생성할 대상
        let target = match kind {
            TurnKind::Regenerate { .. } => {
                let last = chat_ui::last_message(db, conversation_id).await.map_err(AppError::from)?;
                match last {
                    Some(m) if m.role == "assistant" => {
                        if let TurnKind::Regenerate { auto: Some(a), .. } = &kind {
                            if !a.matches(&m) {
                                return Err(AppError::validation("다시 쓸 메시지가 바뀌어 자동 재생성을 멈춤", None).into());
                            }
                        }
                        Some(m)
                    }
                    _ => return Err(AppError::validation("다시 만들 AI 메시지가 없음", None).into()),
                }
            }
            TurnKind::Send(..) | TurnKind::Reply | TurnKind::Continue => None,
        };
        // AI 변수 (5-F-4-4): 다시 만들면 그 메시지가 바꾼 몫을 먼저 되돌린 값이 기준 (프롬프트에도 그 값)
        let var_defs = ai_var_defs(app, db, &conv.character_id).await;
        let stored_vars = super::vars::current_vars(conv.meta.as_ref());
        let mut base_vars = stored_vars.clone();
        if let Some(old) = &target {
            super::vars::revert(&mut base_vars, &super::vars::changes_of(old.meta.as_ref()));
        }
        // 이어 쓰기 (5-E-26): "이어서 응답 받기"일 때만 마지막(숨기지 않은) AI 메시지 뒤에 이어 붙인다.
        // 빈 보내기는 이어 붙이지 않고 사용자 메시지 없이 새 AI 메시지를 만든다. 이어 붙일 AI 메시지가 없으면 새로 만든다
        let continue_of = if continue_last {
            chat_ui::last_message(db, conversation_id)
                .await
                .map_err(AppError::from)?
                .filter(|m| m.role == "assistant" && !is_hidden(m.meta.as_ref()))
        } else {
            None
        };

        let mut context = ContextManager
            .build_context(
                db,
                ContextRequest {
                    conversation_id: conversation_id.to_string(),
                    character_id: conv.character_id.clone(),
                    persona_id: persona_id.clone(),
                    history_limit: HISTORY_FETCH_LIMIT,
                },
            )
            .await?;
        with_global_vars(app, &mut context);
        if char_phone {
            phone_sides::apply_sides(&mut context, &me, &other);
        }

        // 이번에 보낼 사용자 입력: 새로 쓴 것, 또는 (다시 생성일 때) 기록의 마지막 사용자 메시지
        let (input, already_stored, generation_type) = match (&kind, &target) {
            (TurnKind::Send(text, images, _), _) => {
                let t = text.trim().to_string();
                context.user_images = images.clone();
                let gt = if continue_of.is_some() {
                    GenerationType::Continue
                } else {
                    GenerationType::Normal
                };
                (t, false, gt)
            }
            (TurnKind::Regenerate { swipe, .. }, Some(t)) => {
                if !is_hidden(t.meta.as_ref()) {
                    context.history.pop();
                }
                let input = match context.history.last() {
                    Some(h) if h.role == "user" => match context.history.pop() {
                        Some(h) => {
                            context.user_images = h.images;
                            h.content
                        }
                        None => String::new(),
                    },
                    _ => String::new(),
                };
                (input, true, if *swipe { GenerationType::Swipe } else { GenerationType::Regenerate })
            }
            (TurnKind::Reply, _) => match context.history.last() {
                Some(h) if h.role == "user" => {
                    let h = context.history.pop().expect("방금 확인한 마지막 기록");
                    context.user_images = h.images;
                    (h.content, true, GenerationType::Normal)
                }
                _ => return Err(AppError::validation("응답할 사용자 메시지가 없음 (이미 응답했을 수 있음)", None).into()),
            },
            _ => unreachable!(),
        };

        // 자동 회상 (3-C-5): 켜져 있을 때만. 프롬프트 창에 든 메시지는 메모·플러그인 블록이 history에 끼기 전에 계산한다
        let recall_plan = recall_setup(app, db, &conv, &kind, &target, &context, &preset_id).await;

        let EnrichedContext {
            images,
            auto_image,
            summary_use,
        } = enrich_context(
            Some(app),
            db,
            settings,
            &conv,
            &fallback,
            &other,
            &preset_prompts,
            &var_defs,
            &base_vars,
            &mut context,
        )
        .await?;
        let summary_use = summary_use.and_then(|u| serde_json::to_value(&u).ok());

        // 커뮤니티 플러그인의 프롬프트 블록 (6-D-5). prompt.modify 플러그인이 돌고 있을 때만 화면에 묻고, 늦으면 없이 보낸다
        let plugin_prompt = if crate::sys::plugins::is_enabled(app, crate::sys::plugins::id::COMMUNITY) {
            use crate::service::community::prompt_hook;
            let answers = prompt_hook::ask(conversation_id, Some(&conv.character_id), &input, |r| {
                app.emit(prompt_hook::REQUEST_EVENT, r).is_ok()
            })
            .await;
            prompt_hook::apply(&mut context, answers)
        } else {
            vec![]
        };

        // 지난 대화 떠올리기: 보낼 때마다 질의 임베딩 1회가 자동 실행으로 듦 (사용자가 켠 스위치가 있을 때만). 실패하면 기억 없이 보냄
        let recall_meta = match recall_plan {
            Some(plan) => {
                let (tasks, ai, store) = (
                    app.state::<Arc<crate::task::TaskManager>>(),
                    app.state::<Arc<AiService>>(),
                    app.state::<Arc<crate::service::memory::store::MemoryStore>>(),
                );
                let out = crate::service::memory::recall::recall(crate::service::memory::recall::RecallParams {
                    db,
                    store: &store,
                    ai: &ai,
                    tasks: &tasks,
                    cfg: &plan.cfg,
                    profile: &plan.profile,
                    conversation_id,
                    character_id: &conv.character_id,
                    input: &input,
                    last_assistant: plan.last_assistant.as_deref(),
                    window_ids: &plan.window,
                    reuse: plan.reuse.as_deref(),
                    chat_cancel: cancel,
                })
                .await;
                if let Some(block) = out.block {
                    // user 역할로 현재 입력 바로 앞에 (system으로 넣으면 공급자에 따라 지시 권한이 생기거나 캐시 앞부분이 깨짐)
                    context.history.push(crate::models::ai_service::HistoryItem {
                        images: vec![],
                        role: "user".into(),
                        content: block,
                        source: Some(crate::models::ai_service::PromptSource::of("recall")),
                    });
                }
                serde_json::to_value(&out.meta).ok()
            }
            None => None,
        };

        // 이번 생성에만 붙이는 지시 (코어 "지시와 함께 다시 생성"): 입력 바로 앞에 user 역할로, 회상 블록 뒤. 저장하는 글에는 들어가지 않는다
        let regen_extra = match &kind {
            TurnKind::Regenerate { extra: Some(e), .. } => clean_extra(e),
            _ => None,
        };
        if let Some(e) = &regen_extra {
            context.history.push(crate::models::ai_service::HistoryItem {
                images: vec![],
                role: "user".into(),
                content: format!("[Instructions for this response]\n{e}"),
                source: Some(crate::models::ai_service::PromptSource::of("regen_extra")),
            });
        }

        let regex = RegexProcessor::load(
            db,
            Some(&conv.character_id),
            Some(&preset_id),
            RegexMacros {
                char_name: other.name.clone(),
                user_name: me.name.clone(),
            },
        )
        .await?;

        // 새 입력은 저장용 정규식을 거쳐 먼저 저장한다 (응답이 실패해도 입력은 남는다)
        let mut user_msg = None;
        let attachments = match &kind {
            TurnKind::Send(_, images, _) => images.clone(),
            _ => vec![],
        };
        let prompt_input = if already_stored || (input.is_empty() && attachments.is_empty()) {
            apply_prompt_regex(&regex, &mut context, &input, &[RegexTarget::Prompt])
        } else {
            let stored = regex.apply(&input, RegexPlacement::UserInput, &[RegexTarget::Stored], Some(0));
            let saved = chat_ui::push_message(
                db,
                MessageInput {
                    conversation_id: conversation_id.to_string(),
                    seq: 0,
                    role: "user".into(),
                    name: Some(me.name.clone()),
                    content: stored.clone(),
                    character_id: None,
                    meta: Some({
                        let mut meta = json!({ "sender": { "type": "user", "name": me.name, "id": me.id, "kind": me.kind } });
                        if let Some(id) = task_id {
                            meta["taskId"] = json!(id);
                        }
                        if !attachments.is_empty() {
                            meta["attachments"] = json!(attachments);
                        }
                        if matches!(&kind, TurnKind::Send(_, _, true)) {
                            meta["ghost"] = json!(true);
                        }
                        meta
                    }),
                },
            )
            .await
            .map_err(AppError::from)?;
            user_msg = Some(saved);
            apply_prompt_regex(&regex, &mut context, &stored, &[RegexTarget::Prompt])
        };

        // 응답 받기 (조각을 모아 두면 취소돼도 받은 데까지 저장할 수 있다)
        let gen_started = now_ms();
        let mut received = String::new();
        let streaming = channel.is_some();
        let outcome = {
            let mut sink = |delta: &str| {
                received.push_str(delta);
                if let Some(ch) = &channel {
                    let _ = ch.send(StreamPayload { chunk: delta.to_string() });
                }
            };
            self.generate_with(
                app,
                db,
                api_profile_id.clone(),
                conv_model(&conv).or_else(|| fallback_model(&conv, &fallback.model)),
                conv_effort(conv.meta.as_ref(), fallback.reasoning_effort.as_deref()),
                Some(preset_id.clone()),
                context,
                prompt_input,
                generation_type,
                cancel,
                streaming,
                &mut sink,
            )
            .await
        };
        let (raw, model, finish_reason, prompt, cancelled): (String, String, Option<String>, Option<PromptMessage>, bool) = match outcome {
            Ok((
                AiResponse {
                    content, model, finish_reason, ..
                },
                prompt,
            )) => (content, model, finish_reason, Some(prompt), false),
            // 사용자가 멈추면 받은 데까지 저장하지만, 자동 재생성이 멈춘 것은 버린다 (반쯤 쓴 글이 스와이프로 쌓이면 안 됨)
            Err(_) if cancel.is_cancelled() && !received.trim().is_empty() && !matches!(kind, TurnKind::Regenerate { auto: Some(_), .. }) => {
                (received, String::new(), None, None, true)
            }
            Err(e) => return Err(e),
        };

        let result = regex.apply(&raw, RegexPlacement::AiOutput, &[RegexTarget::Stored], Some(0));
        // <image> 그림 프롬프트는 본문에서 빼서 meta에 (자동 생성이 꺼져 있어도 태그는 지운다)
        let (result, auto_image_prompt) = take_image_tag(&result);
        // AI 변수 태그 (5-F-4-4): 본문에서 떼고 정의로 걸러 바뀔 몫으로
        let (result, var_tags) = if var_defs.is_empty() {
            (result, vec![])
        } else {
            super::vars::take_tags(&result)
        };
        let var_changes = super::vars::plan(&var_defs, &base_vars, &var_tags);
        let auto_image_prompt = auto_image_prompt.filter(|_| auto_image);
        let (content, used_images) = sanitize_cg_tags(&result, &images);
        // 메시지 linter (3-B, ADR-014): 정규식·태그 정리 뒤, 저장 직전. 플러그인 후처리(화면)보다 먼저라 플러그인은 늘 검사된 글을 받는다
        let (content, lint_meta) = super::lint::apply_to_response(app, &content, &me.name, conv.meta.as_ref(), is_truncated(finish_reason.as_deref()));
        // 자동 재생성 (3-B-4): linter가 걸었고 켜져 있으면 다음 시도가 필요하다 — 응답이 취소된 것에는 이어가지 않는다
        let (attempts_done, auto_info) = match &kind {
            TurnKind::Regenerate { auto: Some(a), .. } => (a.attempt, Some(json!({ "attempt": a.attempt, "max": a.max, "rules": a.rules }))),
            _ => (0, None),
        };
        // 이어쓰기에는 자동 재생성을 걸지 않는다 (다시 만들면 이어 붙이기 전의 글까지 바뀐다)
        let rewrite_cfg = if cancelled || continue_of.is_some() {
            None
        } else {
            super::rewrite::enabled_for(app, &conv)
        };
        // 자동 재생성의 금지어는 linter와 따로: 자기 목록으로 검사해 결과를 meta.lint에 남긴다 (linter의 금지어는 다시 쓰기를 일으키지 않음)
        let lint_meta = match &rewrite_cfg {
            Some(cfg) if cfg.banned => match super::lint::rewrite_banned_finding(&content, &cfg.banned_words) {
                Some(f) => Some(super::lint::with_finding(lint_meta, &content, f)),
                None => lint_meta,
            },
            _ => lint_meta,
        };
        let rewrite_pending = rewrite_cfg.and_then(|cfg| super::rewrite::decide(&cfg, lint_meta.as_ref(), attempts_done));
        // 이어쓰기: linter 결과의 위치는 이어 붙인 조각 기준이라 합친 글과 맞지 않는다 — 고친 글은 쓰고 결과 표시는 남기지 않는다
        let lint_meta = if continue_of.is_some() { None } else { lint_meta };
        let info = json!({
            "apiProfileId": api_profile_id,
            "presetId": preset_id,
            "model": model,
            "genStarted": gen_started,
            "genFinished": now_ms(),
            "finishReason": finish_reason,
            "cancelled": cancelled,
            "pluginPrompt": plugin_prompt,
            "memoryRecall": recall_meta,
            "summary": summary_use,
            "regenExtra": regen_extra,
            "rewrite": auto_info,
            "rewritePending": rewrite_pending,
        });
        let images_json = serde_json::to_value(&used_images).unwrap_or(Value::Array(vec![]));

        let mut assistant = match (&target, &continue_of) {
            (None, Some(old)) => {
                let joined = join_continue(&old.content, &content);
                let meta = continue_meta(old, &joined, &info, &used_images);
                crate::db::conversation::update_message(db, &old.id, &joined, &Some(meta))
                    .await
                    .map_err(AppError::from)?;
                crate::db::conversation::touch_conversation(db, conversation_id).await.map_err(AppError::from)?;
                chat_ui::get_message(db, &old.id).await.map_err(AppError::from)?
            }
            (None, None) => chat_ui::push_message(
                db,
                MessageInput {
                    conversation_id: conversation_id.to_string(),
                    seq: 0,
                    role: "assistant".into(),
                    name: Some(other.name.clone()),
                    content: content.clone(),
                    character_id: other.character_id().map(str::to_string),
                    meta: Some(json!({
                        "sender": { "type": "assistant", "name": other.name, "id": other.id, "kind": other.kind },
                        "info": info,
                        "images": images_json,
                    })),
                },
            )
            .await
            .map_err(AppError::from)?,
            (Some(old), _) => {
                let swipe = matches!(kind, TurnKind::Regenerate { swipe: true, .. });
                let meta = if swipe {
                    push_swipe(old, &content, &info, &images_json)
                } else {
                    replace_meta(old, &info, &images_json)
                };
                crate::db::conversation::update_message(db, &old.id, &content, &Some(meta))
                    .await
                    .map_err(AppError::from)?;
                crate::db::conversation::touch_conversation(db, conversation_id).await.map_err(AppError::from)?;
                chat_ui::get_message(db, &old.id).await.map_err(AppError::from)?
            }
        };
        if let Some(prompt) = prompt {
            if let Ok(v) = serde_json::to_value(&prompt) {
                chat_ui::save_prompt(db, &assistant.id, &v).await.map_err(AppError::from)?;
            }
        }
        // AI 변수 (5-F-4-4): 바뀐 몫을 메시지(지금 스와이프)에 남기고 대화 변수에 적용. 다시 만든 경우 앞 몫은 이미 되돌렸다.
        // 이어 쓰기는 앞 몫 뒤에 더한다
        let var_meta = if var_defs.is_empty() {
            None
        } else {
            let mut all = match &continue_of {
                Some(old) if target.is_none() => super::vars::changes_of(old.meta.as_ref()),
                _ => vec![],
            };
            all.extend(var_changes.iter().cloned());
            let mut after = base_vars.clone();
            super::vars::apply(&mut after, &var_changes);
            if let Some(patch) = super::vars::meta_patch(&stored_vars, &after) {
                chat_ui::patch_conversation_meta(db, conversation_id, &patch).await.map_err(AppError::from)?;
            }
            Some(serde_json::to_value(&all).unwrap_or(Value::Array(vec![])))
        };
        if auto_image_prompt.is_some() || lint_meta.is_some() || var_meta.is_some() {
            let mut meta = assistant.meta.clone().filter(Value::is_object).unwrap_or_else(|| json!({}));
            if let Some(v) = var_meta {
                let sid = meta.get("swipeId").and_then(Value::as_u64);
                if let Some(e) = sid.and_then(|i| meta.get_mut("swipes").and_then(Value::as_array_mut).and_then(|a| a.get_mut(i as usize))) {
                    if !e["meta"].is_object() {
                        e["meta"] = json!({});
                    }
                    e["meta"]["varChanges"] = v.clone();
                }
                meta["varChanges"] = v;
            }
            if let Some(p) = auto_image_prompt {
                meta["autoImagePrompt"] = json!(p);
            }
            if let Some(l) = lint_meta {
                // 스와이프마다 자기 linter 결과를 가진다 (자동 재생성이 가장 나은 스와이프를 고를 때 읽음)
                let sid = meta.get("swipeId").and_then(Value::as_u64);
                if let Some(e) = sid.and_then(|i| meta.get_mut("swipes").and_then(Value::as_array_mut).and_then(|a| a.get_mut(i as usize))) {
                    e["meta"]["lint"] = l.clone();
                }
                meta["lint"] = l;
            }
            crate::db::conversation::update_message_meta(db, &assistant.id, meta)
                .await
                .map_err(AppError::from)?;
            assistant = chat_ui::get_message(db, &assistant.id).await.map_err(AppError::from)?;
        }

        let _ = app.emit(
            EVENT_DIALOGUE_DONE,
            DialogueDoneEvent {
                conversation_id: conversation_id.to_string(),
                full_message: content,
                images: used_images,
            },
        );
        // 쌓인 만큼이 되었으면 백그라운드로 요약 (켜져 있을 때만, 보내기를 늦추지 않음)
        crate::service::summary::hooks::after_response(app, &conv, &api_profile_id, &preset_id);
        Ok(ChatTurn { user: user_msg, assistant })
    }

    /// 대필 (5-D, ADR-018): 사용자의 다음 대사를 사용자의 말투로 쓴다. 메시지로 저장하지 않고 글만 돌려준다 (화면이 입력칸에 채움).
    /// 컨텍스트는 보통 응답과 같지만 자동 회상·정규식·linter·커뮤니티 프롬프트 훅은 거치지 않는다.
    /// 프리셋의 뒤쪽 블록(지침·prefill)이 마지막 말이 되지 않게, 지시를 **맨 끝 user 메시지**로 붙이고 끝의 assistant prefill은 뗀다.
    #[allow(clippy::too_many_arguments)]
    pub async fn ghost(
        &self,
        app: &AppHandle,
        db: &crate::db::conn::DbPool,
        settings: &SettingsState<AppSetting>,
        conversation_id: &str,
        brief: Option<String>,
        cfg: &crate::models::settings::GhostSetting,
        channel: Option<Channel<StreamPayload>>,
        cancel: &CancellationToken,
    ) -> Result<String> {
        let conv = match get_conversation(db, conversation_id).await {
            Ok(c) => c,
            Err(sqlx::Error::RowNotFound) => return Err(AppError::not_found("대화를 찾을 수 없음").into()),
            Err(e) => return Err(AppError::from(e).into()),
        };
        reject_read_mode(&conv)?;
        let mut fallback = current_conv_request(settings)?;
        phone_defaults(app, &conv.mode, &mut fallback);
        let api_profile_id = cfg
            .api_profile_id
            .clone()
            .filter(|s| !s.is_empty())
            .or(conv.api_profile_id.clone().filter(|s| !s.is_empty()))
            .or(Some(fallback.api_profile_id.clone()).filter(|s| !s.is_empty()))
            .ok_or_else(|| AppError::validation("API를 먼저 고르세요 (대화 설정 또는 설정 > API 키)", None))?;
        let persona_id = resolve_conv_persona(db, &conv.persona_id, &conv.character_id).await;
        if conv.character_id.is_empty() {
            return Err(AppError::validation("캐릭터가 지워진 대화", None).into());
        }
        let character = get_character(db, &conv.character_id).await.map_err(AppError::from)?;
        let persona = get_persona(db, &persona_id).await.map_err(AppError::from)?;
        let preset_id = conv
            .prompt_preset_id
            .clone()
            .filter(|s| !s.is_empty())
            .or(character.default_prompt_preset_id.clone().filter(|s| !s.is_empty()))
            .unwrap_or_else(|| fallback.preset_id.clone());
        // 나/상대: 캐릭터 폰이면 주인 캐릭터와 연락처 — "내 대사"는 `me`의 것
        let (me, other, char_phone) = phone_sides::sides(db, conv.meta.as_ref(), &character, &persona).await?;

        let mut context = ContextManager
            .build_context(
                db,
                ContextRequest {
                    conversation_id: conversation_id.to_string(),
                    character_id: conv.character_id.clone(),
                    persona_id: persona_id.clone(),
                    history_limit: HISTORY_FETCH_LIMIT,
                },
            )
            .await?;
        with_global_vars(app, &mut context);
        if char_phone {
            phone_sides::apply_sides(&mut context, &me, &other);
        }
        crate::service::chat::notes::apply(&mut context, conv.meta.as_ref());
        if crate::sys::plugins::is_enabled(app, crate::sys::plugins::id::SUMMARY) && !crate::service::summary::disabled_for(conv.meta.as_ref()) {
            if let Some((block, _)) = crate::service::summary::block_for(db, conversation_id).await {
                context.system_rules.push(block);
            }
        }
        let history: Vec<(String, String)> = context.history.iter().map(|h| (h.role.clone(), h.content.clone())).collect();
        let samples = super::ghost::style_samples(&history);
        let instruction = super::ghost::build_instruction(&me.name, &other.name, brief.as_deref(), &samples, cfg);

        // 프롬프트: 입력 없이(user_message 비움) Impersonate로 짜서 프리셋이 대필용 블록을 고르게 하고, 마지막에 지시를 붙인다
        let mut prompt = PromptManager
            .build_with_db(
                db,
                PromptRequest {
                    context,
                    user_message: String::new(),
                    preset_id: Some(preset_id),
                    context_limit: DEFAULT_CONTEXT_LIMIT,
                    world_info_ratio: DEFAULT_WORLD_INFO_RATIO,
                    generation_type: GenerationType::Impersonate,
                },
            )
            .await?;
        super::ghost::finish_prompt(&mut prompt, instruction, &cfg.length, channel.is_some());

        let ai = app
            .try_state::<Arc<AiService>>()
            .ok_or_else(|| AppError::internal("AiService가 초기화되지 않았습니다"))?
            .inner()
            .clone();
        let response = ai
            .generate(
                AiRequest {
                    profile_id: api_profile_id,
                    // 대필 API를 따로 정했으면 대필 모델(비우면 그 API의 기본), 아니면 이 대화의 모델
                    model: if cfg.api_profile_id.as_deref().is_some_and(|s| !s.is_empty()) {
                        Some(cfg.model.trim().to_string()).filter(|m| !m.is_empty())
                    } else {
                        conv_model(&conv)
                    },
                    prompt,
                },
                cancel,
                |d| {
                    if let Some(ch) = &channel {
                        let _ = ch.send(StreamPayload { chunk: d.to_string() });
                    }
                },
            )
            .await?;
        super::ghost::clean_output(&response.content, &me.name, &other.name)
            .ok_or_else(|| AppError::validation("대필 결과가 비어 있음 — 다시 눌러 보세요", None).into())
    }

    /// 슬래시 커맨드 /gen (5-E-20-5): 이 대화의 맥락(프리셋·캐릭터·기록·메모) 끝에 지시를 붙여 글만 받는다.
    /// 메시지로 저장하지 않는다 (스크립트가 결과를 어떻게 쓸지 정함).
    #[allow(clippy::too_many_arguments)]
    pub async fn quiet_gen(
        &self,
        app: &AppHandle,
        db: &crate::db::conn::DbPool,
        settings: &SettingsState<AppSetting>,
        conversation_id: &str,
        instruction: &str,
        raw: Option<Option<String>>,
        cancel: &CancellationToken,
    ) -> Result<(String, bool, Value)> {
        let conv = match get_conversation(db, conversation_id).await {
            Ok(c) => c,
            Err(sqlx::Error::RowNotFound) => return Err(AppError::not_found("대화를 찾을 수 없음").into()),
            Err(e) => return Err(AppError::from(e).into()),
        };
        reject_read_mode(&conv)?;
        let mut fallback = current_conv_request(settings)?;
        phone_defaults(app, &conv.mode, &mut fallback);
        let (profile_id, prompt) = match raw {
            // /genraw: 대화 맥락 없이 (system=이 있으면 시스템 글 + 지시만)
            Some(system) => raw_prompt(&conv, fallback, instruction, system.as_deref())?,
            None => quiet_prompt(db, Some(app), &conv, fallback, instruction).await?,
        };
        // 보낸 프롬프트 (QR로 만든 메시지에도 '보낸 프롬프트'를 남기게, 5-E-20-6-10)
        let sent = serde_json::to_value(&prompt).unwrap_or(Value::Null);
        let ai = app
            .try_state::<Arc<AiService>>()
            .ok_or_else(|| AppError::internal("AiService가 초기화되지 않았습니다"))?
            .inner()
            .clone();
        let response = ai
            .generate(
                AiRequest {
                    profile_id,
                    model: conv_model(&conv),
                    prompt,
                },
                cancel,
                |_| {},
            )
            .await?;
        // 응답 길이 한도에서 잘렸는가 (5-E-20-6-6)
        Ok((response.content.trim().to_string(), is_truncated(response.finish_reason.as_deref()), sent))
    }

    /// 폰 엿보기: 두 사람 몫을 AI가 모두 이어서 쓰고, 말풍선 메시지로 나눠 저장한다.
    /// 취소되면 받은 데까지 나눠 저장한다.
    pub async fn peek(
        &self,
        app: &AppHandle,
        db: &crate::db::conn::DbPool,
        settings: &SettingsState<AppSetting>,
        conversation_id: &str,
        channel: Option<Channel<StreamPayload>>,
        cancel: &CancellationToken,
    ) -> Result<Vec<Message>> {
        let conv = match get_conversation(db, conversation_id).await {
            Ok(c) => c,
            Err(sqlx::Error::RowNotFound) => return Err(AppError::not_found("대화를 찾을 수 없음").into()),
            Err(e) => return Err(AppError::from(e).into()),
        };
        if conv.mode != "phone" {
            return Err(AppError::validation("엿보기는 폰 대화에서만 됨", None).into());
        }
        let mut fallback = current_conv_request(settings)?;
        phone_defaults(app, &conv.mode, &mut fallback);
        let api_profile_id = conv
            .api_profile_id
            .clone()
            .filter(|s| !s.is_empty())
            .or(Some(fallback.api_profile_id.clone()).filter(|s| !s.is_empty()))
            .ok_or_else(|| AppError::validation("API를 먼저 고르세요 (대화 설정 또는 설정 > API 키)", None))?;
        let persona_id = resolve_conv_persona(db, &conv.persona_id, &conv.character_id).await;
        let character = get_character(db, &conv.character_id).await.map_err(AppError::from)?;
        let persona = get_persona(db, &persona_id).await.map_err(AppError::from)?;
        let preset_id = conv
            .prompt_preset_id
            .clone()
            .filter(|s| !s.is_empty())
            .or(character.default_prompt_preset_id.clone().filter(|s| !s.is_empty()))
            .unwrap_or_else(|| fallback.preset_id.clone());
        let (me, other, _) = phone_sides::sides(db, conv.meta.as_ref(), &character, &persona).await?;

        let mut context = ContextManager
            .build_context(
                db,
                ContextRequest {
                    conversation_id: conversation_id.to_string(),
                    character_id: conv.character_id.clone(),
                    persona_id: persona_id.clone(),
                    history_limit: HISTORY_FETCH_LIMIT,
                },
            )
            .await?;
        with_global_vars(app, &mut context);
        phone_sides::apply_sides(&mut context, &me, &other);
        phone_sides::peek_history(&mut context.history, &me, &other);
        for item in context.history.iter_mut() {
            item.content = strip_cg_tags(&item.content);
        }
        context.system_rules.push(phone_sides::peek_rule(&me, &other));

        let gen_started = now_ms();
        let mut received = String::new();
        let streaming = channel.is_some();
        let outcome = {
            let mut sink = |delta: &str| {
                received.push_str(delta);
                if let Some(ch) = &channel {
                    let _ = ch.send(StreamPayload { chunk: delta.to_string() });
                }
            };
            self.generate_with(
                app,
                db,
                api_profile_id.clone(),
                conv_model(&conv).or_else(|| fallback_model(&conv, &fallback.model)),
                conv_effort(conv.meta.as_ref(), fallback.reasoning_effort.as_deref()),
                Some(preset_id.clone()),
                context,
                format!("(Continue the messenger chat between {} and {})", me.name, other.name),
                GenerationType::Normal,
                cancel,
                streaming,
                &mut sink,
            )
            .await
        };
        let (raw, model, cancelled) = match outcome {
            Ok((AiResponse { content, model, .. }, _)) => (content, model, false),
            Err(_) if cancel.is_cancelled() && !received.trim().is_empty() => (received, String::new(), true),
            Err(e) => return Err(e),
        };
        let info = json!({
            "apiProfileId": api_profile_id,
            "presetId": preset_id,
            "model": model,
            "genStarted": gen_started,
            "genFinished": now_ms(),
            "cancelled": cancelled,
            "peek": true,
        });

        let mut saved = vec![];
        for (mine, text) in phone_sides::parse_peek(&strip_cg_tags(&raw), &me.name, &other.name) {
            let side: &Side = if mine { &me } else { &other };
            let role = if mine { "user" } else { "assistant" };
            let msg = chat_ui::push_message(
                db,
                MessageInput {
                    conversation_id: conversation_id.to_string(),
                    seq: 0,
                    role: role.into(),
                    name: Some(side.name.clone()),
                    content: text,
                    character_id: side.character_id().map(str::to_string),
                    meta: Some(json!({
                        "sender": { "type": role, "name": side.name, "id": side.id, "kind": side.kind },
                        "info": info,
                    })),
                },
            )
            .await
            .map_err(AppError::from)?;
            saved.push(msg);
        }
        if saved.is_empty() {
            return Err(AppError::validation("AI가 메시지를 쓰지 않았음. 다시 시도해 보세요", None).into());
        }
        Ok(saved)
    }
}

/// 응답 끝의 <image>그림 프롬프트</image>를 떼어 낸다 (본문, 프롬프트). 여러 개면 마지막 것을 쓴다
pub(crate) fn take_image_tag(text: &str) -> (String, Option<String>) {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut prompt = None;
    let mut i = 0;
    while let Some(start) = lower[i..].find("<image>").map(|p| p + i) {
        out.push_str(&text[i..start]);
        let body_start = start + "<image>".len();
        match lower[body_start..].find("</image>").map(|p| p + body_start) {
            Some(end) => {
                let p = text[body_start..end].trim();
                if !p.is_empty() {
                    prompt = Some(p.to_string());
                }
                i = end + "</image>".len();
            }
            None => {
                // 닫히지 않은 태그(응답이 끊김): 남은 글을 프롬프트로 본다
                let p = text[body_start..].trim();
                if !p.is_empty() {
                    prompt = Some(p.to_string());
                }
                i = text.len();
            }
        }
    }
    out.push_str(&text[i.min(text.len())..]);
    (out.trim_end().to_string(), prompt)
}

/// 응답 길이(max_tokens) 한도에서 끊겼는가 — 공급자마다 이름이 다르다 (OpenAI length, Claude max_tokens, Gemini MAX_TOKENS)
pub fn is_truncated(finish_reason: Option<&str>) -> bool {
    matches!(finish_reason, Some("length" | "max_tokens" | "MAX_TOKENS"))
}

/// /genraw 프롬프트: 맥락 없이 지시만 (system=이 있으면 그 앞에 시스템 글). (API 프로필 id, 프롬프트)
fn raw_prompt(
    conv: &crate::models::conversation::Conversation,
    fallback: ChatRequest,
    instruction: &str,
    system: Option<&str>,
) -> Result<(String, crate::models::ai_service::PromptMessage)> {
    use crate::models::ai_service::{ChatOptions, PromptMessage, PromptMessageItem, PromptRole};
    let api_profile_id = conv
        .api_profile_id
        .clone()
        .filter(|s| !s.is_empty())
        .or(Some(fallback.api_profile_id.clone()).filter(|s| !s.is_empty()))
        .ok_or_else(|| AppError::validation("API를 먼저 고르세요 (대화 설정 또는 설정 > API 키)", None))?;
    let item = |role, content: &str| PromptMessageItem {
        role,
        content: content.to_string(),
        token: 0,
        images: vec![],
        source: Some(crate::models::ai_service::PromptSource::of("qr")),
    };
    let mut messages = vec![];
    if let Some(s) = system.map(str::trim).filter(|s| !s.is_empty()) {
        messages.push(item(PromptRole::System, s));
    }
    messages.push(item(PromptRole::User, instruction));
    Ok((
        api_profile_id,
        PromptMessage {
            chat_options: ChatOptions {
                stream: false,
                ..Default::default()
            },
            messages,
        },
    ))
}

/// /gen 프롬프트: 대화 맥락(Quiet 생성 유형) + 맨 끝 사용자 차례에 지시. (API 프로필 id, 프롬프트)
async fn quiet_prompt(
    db: &crate::db::conn::DbPool,
    app: Option<&AppHandle>,
    conv: &crate::models::conversation::Conversation,
    fallback: ChatRequest,
    instruction: &str,
) -> Result<(String, crate::models::ai_service::PromptMessage)> {
    let api_profile_id = conv
        .api_profile_id
        .clone()
        .filter(|s| !s.is_empty())
        .or(Some(fallback.api_profile_id.clone()).filter(|s| !s.is_empty()))
        .ok_or_else(|| AppError::validation("API를 먼저 고르세요 (대화 설정 또는 설정 > API 키)", None))?;
    let persona_id = resolve_conv_persona(db, &conv.persona_id, &conv.character_id).await;
    if conv.character_id.is_empty() {
        return Err(AppError::validation("캐릭터가 지워진 대화", None).into());
    }
    let character = get_character(db, &conv.character_id).await.map_err(AppError::from)?;
    let preset_id = conv
        .prompt_preset_id
        .clone()
        .filter(|s| !s.is_empty())
        .or(character.default_prompt_preset_id.clone().filter(|s| !s.is_empty()))
        .unwrap_or(fallback.preset_id);
    let mut context = ContextManager
        .build_context(
            db,
            ContextRequest {
                conversation_id: conv.id.clone(),
                character_id: conv.character_id.clone(),
                persona_id,
                history_limit: HISTORY_FETCH_LIMIT,
            },
        )
        .await?;
    if let Some(app) = app {
        with_global_vars(app, &mut context);
    }
    crate::service::chat::notes::apply(&mut context, conv.meta.as_ref());
    // 마지막 AI 답장도 맥락이라 지우지 않는다 (대필과 달리 이어 쓸 자리가 아님).
    // 지시를 user_message로 넘겨 월드인포 키워드 스캔·토큰 예산에 포함시키고, Quiet 생성이므로 프롬프트 맨 끝에 붙인다.
    let mut prompt = PromptManager
        .build_with_db(
            db,
            PromptRequest {
                context,
                user_message: instruction.to_string(),
                preset_id: Some(preset_id),
                context_limit: DEFAULT_CONTEXT_LIMIT,
                world_info_ratio: DEFAULT_WORLD_INFO_RATIO,
                generation_type: GenerationType::Quiet,
            },
        )
        .await?;
    prompt.chat_options.stream = false;
    Ok((api_profile_id, prompt))
}

#[cfg(test)]
mod quiet_gen_tests {
    use super::*;

    #[tokio::test]
    async fn gen_prompt_is_the_chat_context_with_the_instruction_last() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::conn::init_pool(dir.path().to_path_buf()).await.unwrap();
        for sql in [
            "INSERT INTO characters (id, name, description, created_at, updated_at) VALUES ('ch', '세라피나', '숲의 수호자', 1, 1)",
            "INSERT INTO conversations (id, title, character_id) VALUES ('c', '대화', 'ch')",
            "INSERT INTO messages (id, conversation_id, seq, role, content) VALUES ('m1', 'c', 1, 'user', '안녕'), ('m2', 'c', 2, 'assistant', '반가워요')",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .execute(&db)
                .await
                .unwrap_or_else(|e| panic!("{sql}: {e}"));
        }
        let conv = get_conversation(&db, "c").await.unwrap();
        let fallback = ChatRequest {
            api_profile_id: "p".into(),
            model: None,
            preset_id: String::new(),
            phone_mode_prompt: String::new(),
            reasoning_effort: None,
        };
        let (profile, prompt) = quiet_prompt(&db, None, &conv, fallback, "다음 장면을 한 줄로 요약해").await.unwrap();
        assert_eq!(profile, "p");
        let last = prompt.messages.last().unwrap();
        assert_eq!(last.role, crate::models::ai_service::PromptRole::User);
        assert_eq!(last.content, "다음 장면을 한 줄로 요약해");
        let all: String = prompt.messages.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n");
        assert!(
            all.contains("반가워요") && all.contains("숲의 수호자"),
            "대화 기록·캐릭터가 맥락에 들어감: {all}"
        );
        assert!(!prompt.chat_options.stream);

        // /genraw: 맥락 없이 시스템 글 + 지시만
        let fb = || ChatRequest {
            api_profile_id: "p".into(),
            model: None,
            preset_id: String::new(),
            phone_mode_prompt: String::new(),
            reasoning_effort: None,
        };
        let (_, raw) = raw_prompt(&conv, fb(), "한 줄 요약", Some("너는 요약가")).unwrap();
        assert_eq!(
            raw.messages.iter().map(|m| m.content.as_str()).collect::<Vec<_>>(),
            vec!["너는 요약가", "한 줄 요약"]
        );

        // API가 없으면 부르지 않고 알린다
        let none = ChatRequest {
            api_profile_id: String::new(),
            model: None,
            preset_id: String::new(),
            phone_mode_prompt: String::new(),
            reasoning_effort: None,
        };
        assert!(quiet_prompt(&db, None, &conv, none, "x").await.is_err());
    }

    #[tokio::test]
    async fn gen_prompt_includes_world_info_matched_by_instruction_keyword() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::conn::init_pool(dir.path().to_path_buf()).await.unwrap();
        for sql in [
            "INSERT INTO characters (id, name, description, created_at, updated_at) VALUES ('ch', '세라피나', '숲의 수호자', 1, 1)",
            "INSERT INTO worldinfo_books (id, name, description) VALUES ('b1', '세계관', '')",
            "INSERT INTO character_worldinfo (character_id, book_id) VALUES ('ch', 'b1')",
            r#"INSERT INTO worldinfo_entries (id, book_id, keys, secondary_keys, content, comment, enabled, priority, constant) VALUES ('e1', 'b1', '["엘도리아"]', '[]', '엘도리아는 고대의 숲이다.', '숲', 1, 10, 0)"#,
            r#"INSERT INTO worldinfo_entries (id, book_id, keys, secondary_keys, content, comment, enabled, priority, constant) VALUES ('e2', 'b1', '["바다"]', '[]', '끝없는 푸른 바다.', '바다', 1, 10, 0)"#,
            "INSERT INTO conversations (id, title, character_id, prompt_preset_id) VALUES ('c', '대화', 'ch', 'default')",
            "INSERT INTO messages (id, conversation_id, seq, role, content) VALUES ('m1', 'c', 1, 'user', '안녕'), ('m2', 'c', 2, 'assistant', '반가워요')",
        ] {
            sqlx::query(sqlx::AssertSqlSafe(sql))
                .execute(&db)
                .await
                .unwrap_or_else(|e| panic!("{sql}: {e}"));
        }
        let conv = get_conversation(&db, "c").await.unwrap();
        let fallback = ChatRequest {
            api_profile_id: "p".into(),
            model: None,
            preset_id: "default".into(),
            phone_mode_prompt: String::new(),
            reasoning_effort: None,
        };
        let (_, prompt) = quiet_prompt(&db, None, &conv, fallback, "프롬프트 내용 : 엘도리아에 대해 설명해줘")
            .await
            .unwrap();
        let all: String = prompt.messages.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join("\n");
        assert!(all.contains("엘도리아는 고대의 숲이다."), "QR 지시 키워드에 걸린 월드인포가 포함됨: {all}");
        assert!(!all.contains("끝없는 푸른 바다."), "걸리지 않은 월드인포는 빠짐: {all}");
        let last = prompt.messages.last().unwrap();
        assert_eq!(last.role, crate::models::ai_service::PromptRole::User);
        assert_eq!(last.content, "프롬프트 내용 : 엘도리아에 대해 설명해줘");
        assert!(last.token > 0);
    }
}

#[cfg(test)]
mod image_tag_tests {
    use super::take_image_tag;

    #[test]
    fn strips_image_tag_and_keeps_prompt() {
        let (body, p) = take_image_tag("그녀가 웃었다.\n<image>1girl, smile, forest</image>");
        assert_eq!(body, "그녀가 웃었다.");
        assert_eq!(p.as_deref(), Some("1girl, smile, forest"));
        let (body, p) = take_image_tag("태그 없음");
        assert_eq!((body.as_str(), p), ("태그 없음", None));
        let (body, p) = take_image_tag("끊김 <IMAGE>1girl, night");
        assert_eq!(body, "끊김");
        assert_eq!(p.as_deref(), Some("1girl, night"));
    }
}

#[cfg(test)]
mod extra_tests {
    use super::*;

    #[test]
    fn extra_instruction_is_trimmed_capped_and_empty_means_none() {
        assert_eq!(clean_extra("  묘사는 짧게  ").as_deref(), Some("묘사는 짧게"));
        assert_eq!(clean_extra("   \n "), None);
        assert_eq!(clean_extra(&"가".repeat(5000)).unwrap().chars().count(), MAX_EXTRA_CHARS);
    }
}

#[cfg(test)]
mod continue_tests {
    use super::*;

    fn msg(meta: Value) -> Message {
        Message {
            id: "m1".into(),
            conversation_id: "c1".into(),
            seq: 2,
            role: "assistant".into(),
            name: None,
            content: "앞부분".into(),
            character_id: None,
            meta: Some(meta),
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn continue_updates_current_swipe_and_keeps_other_marks() {
        let old = msg(json!({
            "chapter": "1장",
            "lint": { "findings": [] },
            "images": [{ "key": "a" }],
            "swipeId": 1,
            "swipes": [
                { "content": "첫째", "meta": {} },
                { "content": "앞부분", "meta": { "lint": {} } }
            ]
        }));
        let info = json!({ "model": "m" });
        let m = continue_meta(&old, "앞부분 뒷부분", &info, &[]);
        assert_eq!(m["chapter"], "1장");
        assert!(m.get("lint").is_none());
        assert_eq!(m["info"], info);
        assert_eq!(m["images"], json!([{ "key": "a" }]));
        assert_eq!(m["swipes"][0]["content"], "첫째");
        assert_eq!(m["swipes"][1]["content"], "앞부분 뒷부분");
        assert_eq!(m["swipes"][1]["meta"]["info"], info);
        assert!(m["swipes"][1]["meta"].get("lint").is_none());
    }

    /// AI 변수 몫 (5-F-4-4): 새 스와이프는 앞 응답의 몫을 첫 스와이프로 옮기고 위에서 뺀다, 처음부터 다시는 지운다
    #[test]
    fn swipes_keep_var_changes_per_swipe() {
        let changes = json!([{ "key": "hp", "from": "100", "to": "90" }]);
        let old = msg(json!({ "info": { "model": "m" }, "varChanges": changes }));
        let m = push_swipe(&old, "새 응답", &json!({}), &json!([]));
        assert!(m.get("varChanges").is_none(), "새 스와이프의 몫은 저장 뒤 따로 넣는다");
        assert_eq!(m["swipes"][0]["meta"]["varChanges"], changes);
        assert_eq!(m["swipeId"], 1);
        let r = replace_meta(&old, &json!({}), &json!([]));
        assert!(r.get("varChanges").is_none());
    }

    #[test]
    fn continue_join_adds_space_only_after_sentence_end() {
        assert_eq!(join_continue("굳는다.", "세라피나가"), "굳는다. 세라피나가");
        assert_eq!(
            join_continue(
                "굳는다.",
                "

세라피나가"
            ),
            "굳는다.

세라피나가"
        );
        assert_eq!(join_continue("세라피", "나가"), "세라피나가");
        assert_eq!(join_continue("\"좋아\"", "그녀가"), "\"좋아\" 그녀가");
        assert_eq!(join_continue("", "처음"), "처음");
    }

    #[test]
    fn conversation_effort_overrides_only_when_set() {
        assert_eq!(conv_effort(Some(&json!({ "reasoningEffort": "high" })), None).as_deref(), Some("high"));
        assert_eq!(conv_effort(Some(&json!({ "reasoningEffort": "auto" })), None), None);
        assert_eq!(conv_effort(Some(&json!({ "reasoningEffort": null })), None), None);
        assert_eq!(conv_effort(None, None), None);
        // 자물쇠를 풀어 둔 대화는 전역, 잠근 대화는 auto여도 전역을 따르지 않음 (5-E-29-3)
        assert_eq!(conv_effort(None, Some("low")).as_deref(), Some("low"));
        assert_eq!(conv_effort(Some(&json!({ "reasoningEffort": null })), Some("low")).as_deref(), Some("low"));
        assert_eq!(conv_effort(Some(&json!({ "reasoningEffort": "auto" })), Some("low")), None);
        assert_eq!(conv_effort(Some(&json!({ "reasoningEffort": "high" })), Some("low")).as_deref(), Some("high"));
    }

    #[test]
    fn continue_without_swipes_only_sets_info() {
        let m = continue_meta(&msg(json!({})), "합친 글", &json!({ "model": "m" }), &[]);
        assert!(m.get("swipes").is_none());
        assert_eq!(m["images"], json!([]));
    }
}

#[cfg(test)]
mod context_usage_tests {
    use super::*;
    use crate::models::conversation::{ConversationInput, ConversationOptions, MessageInput};
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO characters (id, name, description, created_at, updated_at) VALUES ('c1', 'Aria', '숲의 마법사 Aria이다.', 0, 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    async fn create_conv(pool: &sqlx::SqlitePool) -> String {
        crate::db::conversation::create_conversation(
            pool,
            ConversationInput {
                title: "시험".into(),
                character_id: "c1".into(),
                persona_id: "default".into(),
                world_info_ids: vec![],
                mode: "chat".into(),
                meta: None,
                images_enabled: None,
            },
        )
        .await
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn computes_live_usage_without_sent_prompt_and_tracks_changes() {
        let pool = test_pool().await;
        let settings = SettingsState(Mutex::new(AppSetting::default()));
        let conv_id = create_conv(&pool).await;

        // 1. 아직 보낸 적 없는 새 대화도 지금 상태(캐릭터·페르소나·프리셋)로 바로 계산한다
        let u0 = compute_context_usage(None, &pool, &settings, &conv_id).await.unwrap();
        assert!(u0.used.is_some() && u0.system > 0);
        assert_eq!(u0.history, 0);
        assert_eq!(u0.history_count, 0);
        assert_eq!(u0.used, Some(u0.system));

        // 2. 메시지를 넣거나 숨기면 보낸 프롬프트 기록 없이도 대화 토큰·개수가 바로 바뀐다
        let m1 = chat_ui::push_message(
            &pool,
            MessageInput {
                conversation_id: conv_id.clone(),
                seq: 0,
                role: "user".into(),
                name: None,
                content: "안녕하세요! 오늘 숲의 날씨는 어떤가요?".into(),
                character_id: None,
                meta: None,
            },
        )
        .await
        .unwrap();
        let u1 = compute_context_usage(None, &pool, &settings, &conv_id).await.unwrap();
        assert_eq!(u1.history_count, 1);
        assert!(u1.history > 0);
        assert_eq!(u1.used, Some(u1.system + u1.history));

        // 숨긴 메시지는 프롬프트에서 빠지므로 계산에서도 빠진다
        crate::db::conversation::update_message_meta(&pool, &m1.id, json!({ "hidden": true }))
            .await
            .unwrap();
        let u_hidden = compute_context_usage(None, &pool, &settings, &conv_id).await.unwrap();
        assert_eq!(u_hidden.history_count, 0);
        assert_eq!(u_hidden.history, 0);

        // 3. 작가 노트를 적으면 설정·캐릭터(system) 토큰이 늘고 메시지 수(history_count)는 늘지 않는다
        chat_ui::patch_conversation_meta(&pool, &conv_id, &json!({ "authorNote": { "text": "{{char}}는 비밀을 숨기고 있다.", "depth": 2 } }))
            .await
            .unwrap();
        let u_note = compute_context_usage(None, &pool, &settings, &conv_id).await.unwrap();
        assert!(u_note.system > u0.system);
        assert_eq!(u_note.history_count, 0);

        // 4. 프리셋에 assistant 역할 prefill 블록이 있어도 대화 메시지 수로 세지 않고, 한도 초과 시에도 오류 대신 사용량을 돌려준다
        sqlx::query(
            "INSERT INTO prompt_presets (id, name, config_json, settings_json, created_at, updated_at) VALUES ('p1', '작은창', '{}', ?, 0, 0)",
        )
        .bind(json!({ "context_size": 4096, "max_tokens": 512 }).to_string())
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO prompt_blocks (id, preset_id, identifier, name, role, content, enabled, position, depth, sort_order, triggers_json, extra_json, created_at, updated_at)
             VALUES ('b1', 'p1', 'charDescription', '캐릭터', 'system', '', 1, 'relative', 0, 0, '[]', '{}', 0, 0),
                    ('b2', 'p1', 'chatHistory', '기록', 'system', '', 1, 'relative', 0, 10, '[]', '{}', 0, 0),
                    ('b3', 'p1', 'prefill', '프리필', 'assistant', '물론이죠, ', 1, 'in_chat', 0, 20, '[]', '{}', 0, 0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        chat_ui::set_options(
            &pool,
            &conv_id,
            &ConversationOptions {
                persona_id: "default".into(),
                prompt_preset_id: Some("p1".into()),
                api_profile_id: None,
                model: None,
            },
        )
        .await
        .unwrap();
        let u_preset = compute_context_usage(None, &pool, &settings, &conv_id).await.unwrap();
        assert_eq!((u_preset.limit, u_preset.reserved), (4096, 512));
        assert_eq!(u_preset.preset_name.as_deref(), Some("작은창"));
        assert_eq!(u_preset.history_count, 0, "프리셋 prefill 블록은 대화 메시지 수에 들어가지 않음");

        // 고정 프롬프트가 한도를 넘어도 오류 대신 고정 토큰을 돌려준다
        sqlx::query("UPDATE prompt_presets SET settings_json = ? WHERE id = 'p1'")
            .bind(json!({ "context_size": 100, "max_tokens": 95 }).to_string())
            .execute(&pool)
            .await
            .unwrap();
        let u_over = compute_context_usage(None, &pool, &settings, &conv_id).await.unwrap();
        assert!(u_over.used.unwrap_or(0) > 5);
        assert_eq!(u_over.history_count, 0);
    }
}
