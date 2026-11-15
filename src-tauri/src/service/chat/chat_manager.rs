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
        conversation::{get_character, get_conversation, get_persona, get_system_default_persona},
    },
    error::AppError,
    models::{
        ai_service::{
            ChatContext, ContextRequest, DialogueDoneEvent, GenerationType, PromptMessage, PromptRequest,
            StreamPayload,
        },
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
    pub(crate) preset_id: String,
    phone_mode_prompt: String,
}

pub fn current_conv_request(state: &SettingsState<AppSetting>) -> Result<ChatRequest> {
    let guard = state
        .0
        .lock()
        .map_err(|_| AppError::internal("settings lock poisoned"))?;
    Ok(ChatRequest {
        api_profile_id: guard.selected_api_profile.clone().unwrap_or_default(),
        preset_id: guard.selected_preset.clone(),
        phone_mode_prompt: guard.phone_mode_prompt.clone(),
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

async fn load_enabled_images(
    app: &AppHandle,
    db: &crate::db::conn::DbPool,
    character_id: &str,
) -> Result<Vec<CharacterImage>> {
    if character_id.is_empty() {
        return Ok(vec![]);
    }
    let base = crate::commands::character_image::app_data_dir(app)?;
    let rows = crate::db::character_image::get_character_images(db, character_id, true).await?;
    Ok(rows.into_iter().map(|r| CharacterImage::from_row(r, &base)).collect())
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
        let profile = target.api_profile_id.filter(|s| !s.is_empty()).unwrap_or(chat_req.api_profile_id);
        let preset = target.prompt_preset_id.filter(|s| !s.is_empty()).unwrap_or(chat_req.preset_id);
        // 스트리밍: 받은 조각은 호출자(game::npc)가 태그를 걸러 화면으로 보낸다
        let streaming = on_delta.is_some();
        let mut noop = |_: &str| {};
        let sink: &mut (dyn FnMut(&str) + Send) = match on_delta {
            Some(f) => f,
            None => &mut noop,
        };
        let (response, prompt) = self
            .generate_with(&app, &state.db, profile, Some(preset), context, message, GenerationType::Normal, &CancellationToken::new(), streaming, sink)
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

        let ai = app
            .try_state::<Arc<AiService>>()
            .ok_or_else(|| AppError::internal("AiService가 초기화되지 않았습니다"))?
            .inner()
            .clone();

        let response = ai
            .generate(AiRequest { profile_id, prompt: prompt.clone() }, cancel, |d| on_delta(d))
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
}

/// 게임 세션별 마지막으로 보낸 프롬프트 (컨텍스트 사용량 표시용, 앱을 끄면 사라짐)
pub(crate) static LAST_GAME_PROMPT: LazyLock<Mutex<HashMap<String, PromptMessage>>> = LazyLock::new(Default::default);

/// 한 차례의 종류
pub enum TurnKind {
    /// 사용자 입력을 저장하고 응답을 만든다. 입력이 비었으면 저장 없이 이어서 응답 (이어쓰기)
    /// 두 번째 값은 붙인 이미지 경로 (글 없이 이미지만 보내도 됨)
    Send(String, Vec<String>),
    /// 마지막 AI 메시지를 다시 만든다. `swipe`면 스와이프로 쌓고, 아니면 스와이프를 버리고 바꾼다
    Regenerate { swipe: bool },
    /// 이미 저장된 마지막 사용자 메시지에 응답한다 (보내기 작업을 다시 시도할 때: 입력은 첫 시도에 저장됨)
    Reply,
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
    let mut swipes = m.get("swipes").and_then(Value::as_array).cloned().unwrap_or_default();
    if swipes.is_empty() {
        let mut first = Map::new();
        if let Some(i) = m.get("info") {
            first.insert("info".into(), i.clone());
        }
        if let Some(i) = m.get("images") {
            first.insert("images".into(), i.clone());
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

/// 다시 생성(처음부터): 스와이프를 버리고 내용만 바꾼다. 숨김·챕터 같은 표시는 남긴다
fn replace_meta(old: &Message, info: &Value, images: &Value) -> Value {
    let mut m = meta_obj(&old.meta);
    m.remove("swipes");
    m.remove("swipeId");
    m.remove("reasoning");
    m.insert("info".into(), info.clone());
    m.insert("images".into(), images.clone());
    Value::Object(m)
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
        let conv = match get_conversation(db, conversation_id).await {
            Ok(c) => c,
            Err(sqlx::Error::RowNotFound) => return Err(AppError::not_found("대화를 찾을 수 없음").into()),
            Err(e) => return Err(AppError::from(e).into()),
        };
        let fallback = current_conv_request(settings)?;
        let api_profile_id = conv
            .api_profile_id
            .clone()
            .filter(|s| !s.is_empty())
            .or(Some(fallback.api_profile_id).filter(|s| !s.is_empty()))
            .ok_or_else(|| AppError::validation("API를 먼저 고르세요 (대화 설정 또는 설정 > API 키)", None))?;
        let persona_id = match conv.persona_id.as_str() {
            "" => get_system_default_persona(db).await.unwrap_or_else(|_| "default".into()),
            id => id.to_string(),
        };
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
            .unwrap_or(fallback.preset_id);

        // 나/상대: 캐릭터 폰이면 주인 캐릭터와 연락처, 아니면 페르소나와 캐릭터
        let (me, other, char_phone) = phone_sides::sides(db, conv.meta.as_ref(), &character, &persona).await?;

        // 다시 생성할 대상
        let target = match kind {
            TurnKind::Regenerate { .. } => {
                let last = chat_ui::last_message(db, conversation_id).await.map_err(AppError::from)?;
                match last {
                    Some(m) if m.role == "assistant" => Some(m),
                    _ => return Err(AppError::validation("다시 만들 AI 메시지가 없음", None).into()),
                }
            }
            TurnKind::Send(..) | TurnKind::Reply => None,
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
        if char_phone {
            phone_sides::apply_sides(&mut context, &me, &other);
        }

        // 이번에 보낼 사용자 입력: 새로 쓴 것, 또는 (다시 생성일 때) 기록의 마지막 사용자 메시지
        let (input, already_stored, generation_type) = match (&kind, &target) {
            (TurnKind::Send(text, images), _) => {
                let t = text.trim().to_string();
                context.user_images = images.clone();
                let gt = if t.is_empty() && images.is_empty() { GenerationType::Continue } else { GenerationType::Normal };
                (t, false, gt)
            }
            (TurnKind::Regenerate { swipe }, Some(t)) => {
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

        // 대화별 메모·작가 노트 (오른쪽 패널)
        crate::service::chat::notes::apply(&mut context, conv.meta.as_ref());

        if let Some(prompt) = ConversationMode::parse_or_default(&conv.mode).system_prompt(&fallback.phone_mode_prompt) {
            context.system_rules.push(prompt);
        }

        let images_on = conv.images_enabled && images_feature_on(app);
        let images = match other.character_id() {
            Some(cid) if images_on => load_enabled_images(app, db, cid).await?,
            _ => vec![],
        };
        match image_rules(&images) {
            Some(rules) => context.system_rules.push(rules),
            None => {
                for item in context.history.iter_mut() {
                    item.content = strip_cg_tags(&item.content);
                }
            }
        }

        let regex = RegexProcessor::load(
            db,
            Some(&conv.character_id),
            Some(&preset_id),
            RegexMacros { char_name: other.name.clone(), user_name: me.name.clone() },
        )
        .await?;

        // 새 입력은 저장용 정규식을 거쳐 먼저 저장한다 (응답이 실패해도 입력은 남는다)
        let mut user_msg = None;
        let attachments = match &kind {
            TurnKind::Send(_, images) => images.clone(),
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
            self.generate_with(app, db, api_profile_id.clone(), Some(preset_id.clone()), context, prompt_input, generation_type, cancel, streaming, &mut sink)
                .await
        };
        let (raw, model, finish_reason, prompt, cancelled): (String, String, Option<String>, Option<PromptMessage>, bool) = match outcome {
            Ok((AiResponse { content, model, finish_reason, .. }, prompt)) => (content, model, finish_reason, Some(prompt), false),
            Err(_) if cancel.is_cancelled() && !received.trim().is_empty() => (received, String::new(), None, None, true),
            Err(e) => return Err(e),
        };

        let result = regex.apply(&raw, RegexPlacement::AiOutput, &[RegexTarget::Stored], Some(0));
        let (content, used_images) = sanitize_cg_tags(&result, &images);
        let info = json!({
            "apiProfileId": api_profile_id,
            "presetId": preset_id,
            "model": model,
            "genStarted": gen_started,
            "genFinished": now_ms(),
            "finishReason": finish_reason,
            "cancelled": cancelled,
        });
        let images_json = serde_json::to_value(&used_images).unwrap_or(Value::Array(vec![]));

        let assistant = match &target {
            None => chat_ui::push_message(
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
            Some(old) => {
                let swipe = matches!(kind, TurnKind::Regenerate { swipe: true });
                let meta = if swipe { push_swipe(old, &content, &info, &images_json) } else { replace_meta(old, &info, &images_json) };
                crate::db::conversation::update_message(db, &old.id, &content, &Some(meta)).await.map_err(AppError::from)?;
                crate::db::conversation::touch_conversation(db, conversation_id).await.map_err(AppError::from)?;
                chat_ui::get_message(db, &old.id).await.map_err(AppError::from)?
            }
        };
        if let Some(prompt) = prompt {
            if let Ok(v) = serde_json::to_value(&prompt) {
                chat_ui::save_prompt(db, &assistant.id, &v).await.map_err(AppError::from)?;
            }
        }

        let _ = app.emit(
            EVENT_DIALOGUE_DONE,
            DialogueDoneEvent { conversation_id: conversation_id.to_string(), full_message: content, images: used_images },
        );
        Ok(ChatTurn { user: user_msg, assistant })
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
        let fallback = current_conv_request(settings)?;
        let api_profile_id = conv
            .api_profile_id
            .clone()
            .filter(|s| !s.is_empty())
            .or(Some(fallback.api_profile_id).filter(|s| !s.is_empty()))
            .ok_or_else(|| AppError::validation("API를 먼저 고르세요 (대화 설정 또는 설정 > API 키)", None))?;
        let persona_id = match conv.persona_id.as_str() {
            "" => get_system_default_persona(db).await.unwrap_or_else(|_| "default".into()),
            id => id.to_string(),
        };
        let character = get_character(db, &conv.character_id).await.map_err(AppError::from)?;
        let persona = get_persona(db, &persona_id).await.map_err(AppError::from)?;
        let preset_id = conv
            .prompt_preset_id
            .clone()
            .filter(|s| !s.is_empty())
            .or(character.default_prompt_preset_id.clone().filter(|s| !s.is_empty()))
            .unwrap_or(fallback.preset_id);
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
                Some(preset_id.clone()),
                context,
                format!("({}와 {}의 메신저 대화를 이어서 써)", me.name, other.name),
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
