//! 대화 기록 및 월드 인포, system role을 가져오는 매니저
//! 게임 NPC 대화의 컨텍스트는 game::npc가 따로 만든다
//! return: ChatContext
use crate::{
    db::{conn::DbPool, conversation::*},
    error::Result,
    models::ai_service::{ChatContext, ContextRequest, HistoryItem},
};

pub struct ContextManager;
impl ContextManager {
    pub async fn build_context(&self, pool: &DbPool, request: ContextRequest) -> Result<ChatContext> {
        // chat history 불러오기
        let (character, persona, messages, world_info, conv) = futures::try_join!(
            get_character(pool, &request.character_id),
            get_persona(pool, &request.persona_id),
            get_messages(pool, &request.conversation_id, request.history_limit, None),
            get_all_conv_worldinfo_book_with_entries(pool, &request.conversation_id, &request.character_id, &request.persona_id),
            get_conversation(pool, &request.conversation_id)
        )?;
        // 이 대화의 변수 ({{getvar::x}}, ADR-029). 전역 변수는 설정에 있어 부르는 쪽이 넣는다 (with_global_vars)
        let variables = conv
            .meta
            .as_ref()
            .and_then(|m| m.get("variables"))
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        // 숨긴 메시지(meta.hidden, SillyTavern의 is_system)는 화면에만 남고 AI에게는 보내지 않는다
        let history = messages
            .into_iter()
            .filter(|m| !is_hidden(m.meta.as_ref()))
            .map(|m| HistoryItem {
                images: attachments_of(m.meta.as_ref()),
                role: m.role,
                content: m.content,
            })
            .collect();

        // 현재 대화 중인 캐릭터의 데이터 불러오기
        Ok(ChatContext {
            user_images: vec![],
            image_prompt: None,
            image_keys: None,
            util_prompts: Default::default(),
            system_rules: vec![],
            char_name: Some(character.name.clone()),
            user_name: Some(persona.name.clone()),
            character_info: Some(character.description),
            example_dialogue: character.example_dialogue,
            main_prompt: character.system_prompt_override.filter(|s| !s.trim().is_empty()),
            persona_info: persona.description,
            world_info,
            history,
            extra_data: serde_json::json!({ "variables": variables }),
        })
    }
}

/// 메시지 meta.attachments의 이미지 경로
pub fn attachments_of(meta: Option<&serde_json::Value>) -> Vec<String> {
    meta.and_then(|m| m.get("attachments"))
        .and_then(serde_json::Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// 메시지 meta의 hidden 플래그
pub fn is_hidden(meta: Option<&serde_json::Value>) -> bool {
    meta.and_then(|m| m.get("hidden")).and_then(serde_json::Value::as_bool).unwrap_or(false)
}
