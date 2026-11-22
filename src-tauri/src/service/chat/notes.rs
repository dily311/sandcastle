//! 대화별 작가 노트·메모 (대화 meta에 저장, 오른쪽 패널에서 씀)
//!
//! - memory: **사용자의 메모**. AI에게 보내지 않는다 (ADR-016 — 긴 대화의 기억은 대화 요약이 맡는다). 이 키에 예전 버전이 `"off"`를
//!   메모리 검색 스위치로 쓴 값이 남아 있을 수 있어 `memory`는 어디서도 읽어 쓰지 않는다 (스위치는 `memoryOff`)
//! - authorNote { text, depth }: 지금 장면의 방향. 대화 기록의 뒤에서 depth번째 앞에 시스템 메시지로 끼운다
//!   (SillyTavern의 Author's Note와 같은 자리. depth 0이면 맨 끝 = 이번 입력 바로 앞)
//! - injects { id: { value, position, depth, role } }: 슬래시 커맨드 /inject로 넣은 글 (5-E-20-6-4, SillyTavern script_injects와 같은 뜻)
//!   position: before(앱 규칙 맨 앞) / after(앱 규칙 끝, 기본) / chat(대화 기록 뒤에서 depth번째 앞) / none(넣지 않음)
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

    // /inject (이름순으로 넣어 순서가 일정하게)
    if let Some(injects) = meta.get("injects").and_then(Value::as_object) {
        let mut list: Vec<(&String, &Value)> = injects.iter().collect();
        list.sort_by(|a, b| a.0.cmp(b.0));
        for (_, inj) in list {
            let Some(value) = text(inj.get("value")) else { continue };
            let value = macros(&value, &char_name, &user_name);
            match inj.get("position").and_then(Value::as_str).unwrap_or("after") {
                "before" => ctx.system_rules.insert(0, value),
                "chat" => {
                    let depth = inj.get("depth").and_then(Value::as_u64).map(|d| d as usize).unwrap_or(DEFAULT_DEPTH);
                    let role = inj
                        .get("role")
                        .and_then(Value::as_str)
                        .filter(|r| matches!(*r, "user" | "assistant"))
                        .unwrap_or("system");
                    let at = ctx.history.len().saturating_sub(depth);
                    ctx.history.insert(
                        at,
                        HistoryItem {
                            images: vec![],
                            role: role.into(),
                            content: value,
                        },
                    );
                }
                "none" => {}
                _ => ctx.system_rules.push(value),
            }
        }
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
    fn injects_go_where_position_says() {
        let mut c = ctx(5);
        c.system_rules = vec!["규칙".into()];
        apply(
            &mut c,
            Some(&json!({ "injects": {
                "a": { "value": "앞에 {{char}}", "position": "before" },
                "b": { "value": "뒤에", "position": "after" },
                "c": { "value": "기록 안 {{user}}", "position": "chat", "depth": 1, "role": "user" },
                "d": { "value": "안 넣음", "position": "none" },
                "e": { "value": "" }
            } })),
        );
        assert_eq!(c.system_rules, vec!["앞에 Aria".to_string(), "규칙".into(), "뒤에".into()]);
        assert_eq!(c.history.len(), 6);
        assert_eq!((c.history[4].role.as_str(), c.history[4].content.as_str()), ("user", "기록 안 Min"));
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
