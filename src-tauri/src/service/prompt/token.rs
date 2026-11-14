//! 토큰 수 추정.
//!
//! 공급자마다 토크나이저가 달라 정확한 값은 알 수 없으므로, 예산 계산용으로
//! cl100k_base 기준 값을 쓴다. 인코딩을 못 얻으면 글자 수 기반으로 보수적으로 추정한다.
use tiktoken::get_encoding;

/// 메시지 하나당 역할/구분자에 붙는 대략적인 오버헤드
pub const MESSAGE_OVERHEAD: usize = 4;

pub fn count_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    match get_encoding("cl100k_base") {
        Some(enc) => enc.count(text),
        // 한글은 대략 글자당 1토큰 이상이라 글자 수를 그대로 쓴다
        None => text.chars().count(),
    }
}

/// 채팅 메시지 하나의 토큰 (본문 + 오버헤드)
pub fn count_message_tokens(text: &str) -> usize {
    count_tokens(text) + MESSAGE_OVERHEAD
}
