//! 채팅 흐름: 컨텍스트 수집(ContextManager) → 프롬프트 조립(PromptManager) → AI 호출(AiService)
//!
//! 지금은 커맨드에서 바로 호출되지만, 이후에는 ChatStream 작업의 TaskExecutor 안에서 같은 흐름을
//! 실행하고 커맨드는 TaskManager::submit만 하게 된다. 그때도 ChatManager는 작업 상태를 직접
//! 바꾸지 않는다 (TaskContext로 진행 이벤트만 보냄).
use std::sync::Arc;

use anyhow::Result;
use tauri::{ipc::Channel, AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use crate::{
    error::AppError,
    models::{
        ai_service::{
            ChatContext, ContextRequest, DialogueDoneEvent, GenerationType, PromptRequest,
            StreamPayload,
        },
        regex::{RegexPlacement, RegexTarget},
        settings::AppSetting,
    },
    service::{
        ai::{AiRequest, AiService},
        chat::{
            context_manager::ContextManager,
            regex_processor::{RegexMacros, RegexProcessor},
        },
        prompt::PromptManager,
    },
    state::AppState,
    sys::settings::SettingsState,
};

const EVENT_DIALOGUE_DONE: &str = "chat:dialogue:done";

// TODO: 설정/프리셋에서 받아오기
const WORLD_INFO_RATIO: f32 = 0.5;
const HISTORY_LIMIT: i64 = 50;
const CONTEXT_LIMIT: u32 = 8192;

pub struct ChatRequest {
    api_profile_id: String,
    character_id: String,
    persona_id: String,
    preset_id: String,
}

pub fn current_conv_request(state: &SettingsState<AppSetting>) -> Result<ChatRequest> {
    let guard = state
        .0
        .lock()
        .map_err(|_| AppError::internal("settings lock poisoned"))?;
    Ok(ChatRequest {
        api_profile_id: guard.selected_api_profile.clone().unwrap_or_default(),
        character_id: guard.selected_character_id.clone().unwrap_or_default(),
        persona_id: guard.selected_persona_id.clone(),
        preset_id: guard.selected_preset.clone(),
    })
}

/// 프롬프트에 들어가기 전 정규식 적용. 치환된 이번 입력을 돌려준다.
/// - 이번 입력(깊이 0): 저장용 + 프롬프트용 (저장 시 append_message가 적용한 것과 같은 결과)
/// - 대화 기록: 이미 저장용이 적용된 상태라 프롬프트용만 (깊이 = 최근부터 1, 2, …)
/// - 월드인포: 저장되지 않으므로 둘 다
fn apply_prompt_regex(regex: &RegexProcessor, context: &mut ChatContext, message: &str) -> String {
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
    regex.apply(message, RegexPlacement::UserInput, &both, Some(0))
}

pub struct ChatManager;

impl ChatManager {
    /** 대화 세션
     * - channel이 있으면 스트리밍으로 받아 조각마다 channel로 보낸다
     * - 완료 시 chat:dialogue:done 이벤트 emit
     */
    pub async fn chat(
        &self,
        app: AppHandle,
        state: &State<'_, AppState>,
        settings: &State<'_, SettingsState<AppSetting>>,
        conversation_id: String,
        message: String,
        channel: Option<Channel<StreamPayload>>,
    ) -> Result<String> {
        let chat_req = current_conv_request(settings)?;

        let mut context = ContextManager
            .build_context(
                &state.db,
                ContextRequest {
                    conversation_id: conversation_id.clone(),
                    character_id: chat_req.character_id.clone(),
                    persona_id: chat_req.persona_id,
                    history_limit: HISTORY_LIMIT,
                },
            )
            .await?;

        // 정규식: 보내는 쪽(프롬프트)과 받는 쪽(응답)
        let regex = RegexProcessor::load(
            &state.db,
            Some(&chat_req.character_id),
            Some(&chat_req.preset_id),
            RegexMacros {
                char_name: context.char_name.clone().unwrap_or_default(),
                user_name: context.user_name.clone().unwrap_or_default(),
            },
        )
        .await?;
        let message = apply_prompt_regex(&regex, &mut context, &message);

        let raw = self
            .generate(
                &app,
                &state.db,
                chat_req.api_profile_id,
                Some(chat_req.preset_id),
                context,
                message,
                channel,
            )
            .await?;
        // 스트리밍 조각은 원문 그대로 나가고, 완료 이벤트/반환값은 치환된 최종본이다.
        // 프론트는 done 이벤트의 full_message로 메시지를 저장해야 한다.
        let result = regex.apply(&raw, RegexPlacement::AiOutput, &[RegexTarget::Stored], Some(0));

        // 프론트에 생성 완료 알리기 -> DB 저장
        app.emit(
            EVENT_DIALOGUE_DONE,
            DialogueDoneEvent {
                conversation_id,
                full_message: result.clone(),
            },
        )?;
        Ok(result)
    }

    /// 게임 대화: 컨텍스트는 GameRuntime(game::npc)이 만들어 넘긴다.
    pub async fn game_chat(
        &self,
        app: AppHandle,
        state: &State<'_, AppState>,
        settings: &State<'_, SettingsState<AppSetting>>,
        _game_conversation_id: String,
        message: String,
        context: ChatContext,
        channel: Option<Channel<StreamPayload>>,
    ) -> Result<String> {
        let chat_req = current_conv_request(settings)?;
        self.generate(
            &app,
            &state.db,
            chat_req.api_profile_id,
            Some(chat_req.preset_id),
            context,
            message,
            channel,
        )
        .await
    }

    /// 프롬프트 조립 + AI 호출 (채팅/게임 공통)
    async fn generate(
        &self,
        app: &AppHandle,
        db: &crate::db::conn::DbPool,
        profile_id: String,
        preset_id: Option<String>,
        context: ChatContext,
        message: String,
        channel: Option<Channel<StreamPayload>>,
    ) -> Result<String> {
        let mut prompt = PromptManager
            .build_with_db(
                db,
                PromptRequest {
                    context,
                    user_message: message,
                    preset_id,
                    context_limit: CONTEXT_LIMIT,
                    world_info_ratio: WORLD_INFO_RATIO,
                    generation_type: GenerationType::Normal,
                },
            )
            .await?;
        // 받을 곳이 없으면 스트리밍할 이유가 없다
        prompt.chat_options.stream = channel.is_some();

        let ai = app
            .try_state::<Arc<AiService>>()
            .ok_or_else(|| AppError::internal("AiService가 초기화되지 않았습니다"))?
            .inner()
            .clone();

        let response = ai
            .generate(
                AiRequest { profile_id, prompt },
                &CancellationToken::new(),
                |delta| {
                    if let Some(ch) = &channel {
                        let _ = ch.send(StreamPayload {
                            chunk: delta.to_string(),
                        });
                    }
                },
            )
            .await?;
        Ok(response.content)
    }
}
