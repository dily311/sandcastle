//! AI 호출 서비스.
//!
//! ```text
//! ChatManager / GameRuntime / (TaskExecutor)
//!        │  AiRequest { profile_id, prompt }
//!        v
//!   AiService ── api_profiles 조회, keyring에서 키 조회, 취소 처리
//!        │
//!        v
//!   adapter::provider_for(format_type) ── 공급자별 HTTP 형식
//! ```
//!
//! AiService는 작업 상태를 모른다. 오래 걸리는 호출을 작업으로 관리하려면 호출자가
//! TaskExecutor 안에서 AiService를 부르고, `ctx.cancel_token()`과 `ctx.reporter().text_delta`를
//! 그대로 넘기면 된다.
mod ai_service;

pub use ai_service::{AiRequest, AiResponse, AiService};
