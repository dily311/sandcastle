//! 폰 대화의 "나"와 "상대".
//!
//! - 페르소나 폰(기본): 나 = 페르소나, 상대(AI) = 캐릭터 — 일반 대화와 같음
//! - 캐릭터 폰: 대화 meta.phone = { owner: {kind:"character", id}, contact: {kind, id} }
//!   나 = 주인 캐릭터, 상대(AI) = 연락처(다른 캐릭터나 페르소나)
//! - 엿보기: 두 사람 몫을 AI가 모두 "이름: 메시지" 줄로 쓰고, 여기서 말풍선으로 나눈다
use serde_json::Value;

use crate::{
    db::{
        conn::DbPool,
        conversation::{get_character, get_persona},
    },
    error::AppError,
    models::{
        ai_service::{ChatContext, HistoryItem},
        conversation::{Character, Persona},
    },
};

/// 대화의 한쪽
#[derive(Debug, Clone)]
pub struct Side {
    /// "persona" | "character"
    pub kind: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub example_dialogue: Option<String>,
    pub main_prompt: Option<String>,
}

impl Side {
    pub fn character(c: &Character) -> Self {
        Self {
            kind: "character".into(),
            id: c.id.clone(),
            name: c.name.clone(),
            description: c.description.clone(),
            example_dialogue: c.example_dialogue.clone(),
            main_prompt: c.system_prompt_override.clone().filter(|s| !s.trim().is_empty()),
        }
    }
    pub fn persona(p: &Persona) -> Self {
        Self {
            kind: "persona".into(),
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            example_dialogue: None,
            main_prompt: None,
        }
    }
    /// 캐릭터면 id (메시지의 character_id, 이미지 에셋용)
    pub fn character_id(&self) -> Option<&str> {
        (self.kind == "character").then_some(self.id.as_str())
    }
}

/// meta.phone.owner / contact
#[derive(Debug, Clone, PartialEq)]
pub struct PhoneMeta {
    pub owner_kind: String,
    pub owner_id: String,
    pub contact_kind: String,
    pub contact_id: String,
}

pub fn phone_meta(meta: Option<&Value>) -> Option<PhoneMeta> {
    let p = meta?.get("phone")?;
    let s = |a: &str, b: &str| p.get(a)?.get(b)?.as_str().map(str::to_string);
    Some(PhoneMeta {
        owner_kind: s("owner", "kind")?,
        owner_id: s("owner", "id")?,
        contact_kind: s("contact", "kind")?,
        contact_id: s("contact", "id")?,
    })
}

async fn load_side(db: &DbPool, kind: &str, id: &str) -> Result<Side, AppError> {
    match kind {
        "character" => get_character(db, id)
            .await
            .map(|c| Side::character(&c))
            .map_err(|_| AppError::validation("연락처 캐릭터가 지워짐", None)),
        "persona" => get_persona(db, id)
            .await
            .map(|p| Side::persona(&p))
            .map_err(|_| AppError::validation("연락처 페르소나가 지워짐", None)),
        _ => Err(AppError::validation(format!("알 수 없는 종류: {kind}"), None)),
    }
}

/// (나, 상대). 캐릭터 폰이면 주인 캐릭터와 연락처, 아니면 페르소나와 캐릭터
pub async fn sides(db: &DbPool, meta: Option<&Value>, character: &Character, persona: &Persona) -> Result<(Side, Side, bool), AppError> {
    match phone_meta(meta) {
        Some(m) if m.owner_kind == "character" => {
            let me = load_side(db, &m.owner_kind, &m.owner_id).await?;
            let other = load_side(db, &m.contact_kind, &m.contact_id).await?;
            Ok((me, other, true))
        }
        _ => Ok((Side::persona(persona), Side::character(character), false)),
    }
}

/// 컨텍스트의 {{char}}/{{user}}와 설명을 두 쪽에 맞게 바꿈
pub fn apply_sides(ctx: &mut ChatContext, me: &Side, other: &Side) {
    ctx.char_name = Some(other.name.clone());
    ctx.user_name = Some(me.name.clone());
    ctx.character_info = Some(other.description.clone());
    ctx.persona_info = me.description.clone();
    ctx.example_dialogue = other.example_dialogue.clone();
    ctx.main_prompt = other.main_prompt.clone();
}

/// 엿보기용 규칙. 두 사람 모두 AI가 씀
pub fn peek_rule(me: &Side, other: &Side) -> String {
    format!(
        "[Messenger peek]\nContinue the smartphone messenger chat between {me} and {other}. You write the messages of both people.\n\
         - Write the next 2 to 6 messages. One person may send several in a row.\n\
         - One message per line, always in the form `{me}: text` or `{other}: text`.\n\
         - No narration, action descriptions or explanations — only the messages actually sent. Write in the language of the conversation.\n\n\
         [{me}]\n{me_desc}\n\n[{other}]\n{other_desc}",
        me = me.name,
        other = other.name,
        me_desc = me.description.trim(),
        other_desc = other.description.trim(),
    )
}

/// 엿보기 기록: 대화 기록을 "이름: 내용" 줄로 (AI가 같은 형식으로 이어 쓰게)
pub fn peek_history(history: &mut [HistoryItem], me: &Side, other: &Side) {
    for h in history.iter_mut() {
        let name = if h.role == "user" { &me.name } else { &other.name };
        h.content = h
            .content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| format!("{name}: {}", l.trim()))
            .collect::<Vec<_>>()
            .join("\n");
        h.role = "assistant".into();
    }
}

/// AI가 쓴 "이름: 내용" 줄 → (내 쪽인지, 내용). 같은 사람이 연달아 쓴 줄은 한 메시지(줄바꿈)로 묶음
pub fn parse_peek(text: &str, me: &str, other: &str) -> Vec<(bool, String)> {
    let mut out: Vec<(bool, String)> = vec![];
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let line = line.trim_start_matches(['-', '*', '•']).trim();
        let speaker = line.split_once(':').or_else(|| line.split_once('：')).and_then(|(n, rest)| {
            let n = n.trim().trim_matches(['*', '"', '「', '」']);
            if n == me {
                Some((true, rest.trim()))
            } else if n == other {
                Some((false, rest.trim()))
            } else {
                None
            }
        });
        match (speaker, out.last_mut()) {
            (Some((mine, body)), Some((last_mine, last))) if *last_mine == mine => {
                if !body.is_empty() {
                    last.push('\n');
                    last.push_str(body);
                }
            }
            (Some((mine, body)), _) => {
                if !body.is_empty() {
                    out.push((mine, body.to_string()));
                }
            }
            // 이름 없는 줄은 앞 메시지에 붙임 (첫 줄이면 상대 말로)
            (None, Some((_, last))) => {
                last.push('\n');
                last.push_str(line);
            }
            (None, None) => out.push((false, line.to_string())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_phone_meta() {
        let m = json!({ "phone": { "owner": { "kind": "character", "id": "c1" }, "contact": { "kind": "persona", "id": "p1" } } });
        let p = phone_meta(Some(&m)).unwrap();
        assert_eq!(p.owner_id, "c1");
        assert_eq!(p.contact_kind, "persona");
        assert!(phone_meta(Some(&json!({}))).is_none());
    }

    #[test]
    fn splits_peek_lines_by_speaker() {
        let text = "Aria: 내일 훈련 몇 시야?\nAria: 늦으면 안 돼\nSeraphina: 해 뜰 때요\n- Aria: 알았어!\n그럼 자러 간다";
        let got = parse_peek(text, "Aria", "Seraphina");
        assert_eq!(
            got,
            vec![
                (true, "내일 훈련 몇 시야?\n늦으면 안 돼".to_string()),
                (false, "해 뜰 때요".to_string()),
                (true, "알았어!\n그럼 자러 간다".to_string()),
            ]
        );
    }

    #[test]
    fn peek_history_labels_lines() {
        let me = Side {
            kind: "character".into(),
            id: "a".into(),
            name: "Aria".into(),
            description: String::new(),
            example_dialogue: None,
            main_prompt: None,
        };
        let other = Side {
            name: "Sera".into(),
            ..me.clone()
        };
        let mut h = vec![HistoryItem {
            images: vec![],
            role: "user".into(),
            content: "안녕\n뭐 해?".into(),
        }];
        peek_history(&mut h, &me, &other);
        assert_eq!(h[0].content, "Aria: 안녕\nAria: 뭐 해?");
    }
}
