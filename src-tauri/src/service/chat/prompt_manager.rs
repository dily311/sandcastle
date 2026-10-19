use crate::{error::Result, models::ai_service::{ChatContext, PromptMessage, PromptRequest}};

pub struct PromptManager;

impl PromptManager {
    pub fn build(&self, request: PromptRequest) -> Result<PromptMessage> {
        
        // 컨텍스트 + 프롬프트 설정 -> 최종 메시지
        todo!()
    }
}
