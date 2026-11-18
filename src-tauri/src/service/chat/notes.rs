//! 대화별 작가 노트·메모 (대화 meta에 저장, 오른쪽 패널에서 씀)
//!
//! - memory: **사용자의 메모**. AI에게 보내지 않는다 (ADR-016 — 긴 대화의 기억은 대화 요약이 맡는다). 이 키에 예전 버전이 `"off"`를
//!   메모리 검색 스위치로 쓴 값이 남아 있을 수 있어 `memory`는 어디서도 읽어 쓰지 않는다 (스위치는 `memoryOff`)
//! - authorNote { text, depth }: 지금 장면의 방향. 대화 기록의 뒤에서 depth번째 앞에 시스템 메시지로 끼운다
//!   (SillyTavern의 Author's Note와 같은 자리. depth 0이면 맨 끝 = 이번 입력 바로 앞)
use serde_json::Value;

use crate::models::ai_service::{ChatContext, HistoryItem};

pub const DEFAULT_DEPTH: usize = 4;

fn text(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

fn macros(s: &str, char_name: &str, user_name: &str) -> String {
    s.replace("{{char}}", char_name).replace("{{user}}", user_name)
}

/// 대화 meta의 메모·작가 노트를 컨텍스트에 넣는다
pub fn apply(ctx: &mut ChatContext, meta: Option<&Value>) {
    let Some(meta) = meta else { return };
    let char_name = ctx.char_name.clone().unwrap_or_default();
    let user_name = ctx.user_name.clone().unwrap_or_default();

    let note = meta.get("authorNote");
    if let Some(t) = text(note.and_then(|n| n.get("text"))) {
        let depth = note
            .and_then(|n| n.get("depth"))
            .and_then(Value::as_u64)
            .map(|d| d as usize)
            .unwrap_or(DEFAULT_DEPTH);
        let at = ctx.history.len().saturating_sub(depth);
        ctx.history.insert(
            at,
            HistoryItem {
                images: vec![],
                role: "system".into(),
                content: format!("[작가 노트]\n{}", macros(&t, &char_name, &user_name)),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx(n: usize) -> ChatContext {
        ChatContext {
            user_images: vec![],
            char_name: Some("Aria".into()),
            user_name: Some("Min".into()),
            history: (0..n)
                .map(|i| HistoryItem {
                    images: vec![],
                    role: "user".into(),
                    content: format!("m{i}"),
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn memo_is_not_sent_and_note_goes_into_history() {
        let mut c = ctx(6);
        apply(
            &mut c,
            Some(&json!({ "memory": "{{user}}는 열쇠를 잃음", "authorNote": { "text": "{{char}}는 긴장함", "depth": 2 } })),
        );
        assert!(c.system_rules.is_empty(), "메모는 사용자의 것 — AI에게 보내지 않음");
        assert_eq!(c.history.len(), 7);
        assert_eq!(c.history[4].role, "system");
        assert!(c.history[4].content.contains("Aria는 긴장함"));
    }

    #[test]
    fn empty_or_short_history_is_fine() {
        let mut c = ctx(1);
        apply(&mut c, Some(&json!({ "authorNote": { "text": "비" } })));
        assert_eq!(c.history[0].role, "system", "기록이 depth보다 짧으면 맨 앞");
        let mut c = ctx(2);
        apply(&mut c, Some(&json!({ "memory": "  ", "authorNote": { "text": "" } })));
        assert!(c.system_rules.is_empty());
        assert_eq!(c.history.len(), 2);
    }
}
