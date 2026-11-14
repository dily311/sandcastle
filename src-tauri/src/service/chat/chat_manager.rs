use crate::adapter::*;
use crate::db::ai_service;
use crate::error::{AppError, ErrorCode};
use crate::models::ai_service::{ApiProfiles, ChatContext, ChatOptions, DialogueDoneEvent, StreamPayload};
use crate::models::settings::AppSetting;
use crate::service::chat::context_manager::ContextManager;
use crate::service::chat::prompt_manager::PromptManager;
use crate::state::AppState;
use crate::sys::settings::SettingsState;
use anyhow::{anyhow, Result};
use tauri::State;
use tauri::{ipc::Channel, AppHandle, Emitter};

const EVENT_DIALOGUE_TOKEN: &str = "chat:dialogue:token";
const EVENT_DIALOGUE_DONE: &str = "chat:dialogue:done";
const EVENT_DIALOGUE_ERROR: &str = "chat:dialogue:error";

pub enum ChatMode {
    Chat,
    Game,
}
pub struct ChatRequest {
    api_profile_id: String,
    character_id: String,
    persona_id: String,
    preset_id: String,
}
pub fn current_conv_request(state: &SettingsState<AppSetting>) -> Result<ChatRequest> {
    let guard = state.0.lock().map_err(|_| AppError::archive(ErrorCode::InternalError, "archive lock poisaned"))?;
    Ok(ChatRequest {
        api_profile_id: guard.selected_api_profile.as_ref().map(|a|a.clone()).unwrap_or_default(),
        character_id: guard.selected_character_id.as_ref().map(|a|a.clone()).unwrap_or_default(),
        persona_id: guard.selected_persona_id.clone(),
        preset_id: guard.selected_preset.clone(),
    })    
}

pub struct ChatManager;
impl ChatManager {
    pub async fn get_models(
        &self,
        api_profile: ApiProfiles,
        client: reqwest::Client,
    ) -> Result<Vec<String>> {
        let result = match api_profile.format_type.as_str() {
            "openai" => openai_adapter::OpenAIClient.get_models(client, api_profile,).await?,
            "claude" => claude_adapter::ClaudeClient.get_models(client, api_profile).await?,
            "gemini" => gemini_adapter::GeminiClient.get_models(client, api_profile).await?,
            "vertexai" => vertexai_adapter::VertexAIClient.get_models(client, api_profile).await?,
            _ => return Err(anyhow!("공급자가 지정되지 않음"))
        };

        Ok(result)
    }

    pub async fn test_connection(&self) -> Result<()> {
        todo!()
    }

    /** 대화 세션
     * api_profile.provider에 따라 각 함수 호출
     * - api_profile.provider: openai | claude | gemini
     */
    pub async fn chat(&self, app: AppHandle, state: &State<'_, AppState>, settings: &State<'_, SettingsState<AppSetting>>, conversation_id: String, message: String, channel: Option<Channel<StreamPayload>>,) -> Result<String> {
        // TODO: context_limit 받아올 수 있게 하기
        let world_info_ratio: f32 = 0.5;
        let history_limit = 50;
        let context_limit = 4080;
        let chat_req = current_conv_request(settings)?;


        let context = ContextManager
            .build_context(
                &state.db,
                crate::models::ai_service::ContextRequest {
                    conversation_id: conversation_id.clone(),
                    character_id: chat_req.character_id,
                    persona_id: chat_req.persona_id,
                    history_limit,
                },
            )
            .await?;
        
        let prompt_message = PromptManager.build(crate::models::ai_service::PromptRequest {
            context,
            user_message: message,
            preset_id: Some(chat_req.preset_id),
            context_limit,
            world_info_ratio,
        })?;

        // adapter를 통해서 api 통신
        // 각 공급자마다 받는 형식이 다르므로 주의
        let client = state.http_client.clone();
        let api_profile = ai_service::get_api_profile(&state.db, &chat_req.api_profile_id).await?;

        let result = match channel {
            Some(on_event) => {
                match api_profile.format_type.as_str() {
                    "openai" => openai_adapter::OpenAIClient.stream_message(client, api_profile, prompt_message, on_event).await?,
                    "claude" => claude_adapter::ClaudeClient.stream_message(client, api_profile, prompt_message, on_event).await?,
                    "gemini" => gemini_adapter::GeminiClient.stream_message(client, api_profile, prompt_message, on_event).await?,
                    "vertexai" => vertexai_adapter::VertexAIClient.stream_message(client, api_profile, prompt_message, on_event).await?,
                    _ => return Err(anyhow!("공급자가 지정되지 않음"))
                }
            }
            None => {
                match api_profile.format_type.as_str() {
                    "openai" => openai_adapter::OpenAIClient.send_message(client, api_profile, prompt_message).await?,
                    "claude" => claude_adapter::ClaudeClient.send_message(client, api_profile, prompt_message).await?,
                    "gemini" => gemini_adapter::GeminiClient.send_message(client, api_profile, prompt_message).await?,
                    "vertexai" => vertexai_adapter::VertexAIClient.send_message(client, api_profile, prompt_message).await?,
                    _ => return Err(anyhow!("공급자가 지정되지 않음"))
                }
            }
        };

        // 프론트에 생성 완료 알리기 -> DB 저장
        let payload = DialogueDoneEvent {
            conversation_id: conversation_id.clone(),
            full_message: result.clone(),
        };
        
        app.emit(EVENT_DIALOGUE_DONE, payload)?;
        Ok(result)
    }

    pub async fn game_chat(&self, _app: AppHandle, _state: &State<'_, AppState>, _settings: &State<'_, SettingsState<AppSetting>>, _game_conversation_id: String, _message: String, _context: ChatContext, _channel: Option<Channel<StreamPayload>>,) -> Result<String> {
        Err(anyhow!("게임 대화 AI 연결이 아직 구현되지 않았습니다"))
    }
}
