//! 대화 기록 및 월드 인포, system role을 가져오는 매니저
//! chat과 game 모드를 지원하기 위해 두 메소드가 존재
//! return: ChatContext
use crate::{
    db::{conn::DbPool, conversation::*}, error::Result, models::ai_service::{ChatContext, ContextRequest, HistoryItem},
};

pub struct ContextManager;
impl ContextManager {
    pub async fn build_context(&self, pool: &DbPool, request: ContextRequest) -> Result<ChatContext> {
        // chat history 불러오기
        let (character, persona, messages, world_info) = futures::try_join!(
            get_character(pool, &request.character_id),
            get_persona(pool, &request.persona_id),
            get_messages(pool, &request.conversation_id, request.history_limit, None),
            get_all_conv_worldinfo_book_with_entries(pool, &request.conversation_id, &request.character_id, &request.persona_id)
        )?;

        let history = messages
            .into_iter()
            .map(|m| HistoryItem {
                role: m.role,
                content: m.content,
            })
            .collect();

        // 현재 대화 중인 캐릭터의 데이터 불러오기
        Ok(ChatContext {
            system_rules: vec![],
            char_name: Some(character.name.clone()),
            user_name: Some(persona.name.clone()),
            character_info: Some(character.description),
            example_dialogue: character.example_dialogue,
            persona_info: persona.description,
            world_info,
            history,
            extra_data: ().into(),
        })
    }

    pub async fn build_game_mode_context(&self, request: &ContextRequest) -> Result<ChatContext> {
        todo!()
    }
}
