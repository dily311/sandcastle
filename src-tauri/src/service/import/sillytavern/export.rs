//! 대화 → SillyTavern 채팅(.jsonl) (5-E-20). 만든 파일은 SillyTavern에서 그대로 열려야 한다 —
//! 샌드캐슬만의 기능(메모·자동 재생성·linter 등)은 싣지 않고, SillyTavern이 아는 칸만 쓴다.
//!
//! 한 줄째: 헤더 `{user_name, character_name, create_date, chat_metadata}`
//! 그 뒤: 메시지마다 `{name, is_user, is_system, send_date, mes, extra, swipes?, swipe_id?, swipe_info?}`
use serde_json::{json, Value};

use crate::models::conversation::Message;

#[derive(Debug, Clone, Default)]
pub struct JsonlOptions {
    /// `<cg>키</cg>` → `{{img::키}}` (SillyTavern 커뮤니티 이미지 확장의 형식)
    pub cg_as_img: bool,
}

pub struct JsonlChat<'a> {
    pub user_name: &'a str,
    pub char_name: &'a str,
    pub created_at: i64,
    pub conv_meta: Option<&'a Value>,
    pub messages: &'a [Message],
}

fn iso(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_default()
}

/// `<cg>키</cg>`(대소문자 무시) → `{{img::키}}`
pub fn cg_to_img(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    while let Some(rel) = lower[pos..].find("<cg>") {
        let start = pos + rel;
        let key_start = start + 4;
        match lower[key_start..].find("</cg>") {
            Some(rel_end) if !text[key_start..key_start + rel_end].contains('\n') => {
                out.push_str(&text[pos..start]);
                out.push_str(&format!("{{{{img::{}}}}}", text[key_start..key_start + rel_end].trim()));
                pos = key_start + rel_end + 5;
            }
            _ => {
                out.push_str(&text[pos..key_start]);
                pos = key_start;
            }
        }
    }
    out.push_str(&text[pos..]);
    out
}

/// 게임 대사 → 읽을 글: 연출·상태 태그(<bgm>·<emotion::이름>·<updateVar> 등)는 지우고, 선택지는 "- 선택지" 줄로 (5-E-20-3)
pub fn strip_game_tags(text: &str) -> String {
    use std::sync::LazyLock;
    static HIDDEN: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
        fancy_regex::Regex::new(r"(?is)<(bgm|sfx|cg|voice|effects|emotion|motion|updatevar|updaterel|scene)(::[^>]*)?>.*?</(bgm|sfx|cg|voice|effects|emotion|motion|updatevar|updaterel|scene)(::[^>]*)?>").unwrap()
    });
    static CHOICE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| fancy_regex::Regex::new(r"(?is)<choice>(.*?)</choice>").unwrap());
    let s = HIDDEN.replace_all(text, "");
    let s = CHOICE.replace_all(&s, "\n- $1");
    s.lines().map(str::trim_end).filter(|l| !l.trim().is_empty()).collect::<Vec<_>>().join("\n")
}

pub fn to_jsonl(chat: &JsonlChat, opts: &JsonlOptions) -> String {
    let body = |s: &str| if opts.cg_as_img { cg_to_img(s) } else { s.to_string() };
    let note = chat.conv_meta.and_then(|m| m.get("authorNote"));
    let mut lines = vec![json!({
        "user_name": chat.user_name,
        "character_name": chat.char_name,
        "create_date": iso(chat.created_at),
        "chat_metadata": {
            "note_prompt": note.and_then(|n| n.get("text")).and_then(Value::as_str).unwrap_or_default(),
            "note_depth": note.and_then(|n| n.get("depth")).and_then(Value::as_u64).unwrap_or(crate::service::chat::notes::DEFAULT_DEPTH as u64),
            "note_interval": 1,
            // 1 = 대화 안 깊이 (샌드캐슬 작가 노트와 같은 자리)
            "note_position": 1,
        },
    })
    .to_string()];
    for m in chat.messages {
        let meta = m.meta.clone().unwrap_or_default();
        let is_user = m.role == "user";
        let mut extra = serde_json::Map::new();
        if m.role == "system" {
            extra.insert("type".into(), json!("narrator"));
        }
        if let Some(r) = meta.get("reasoning").and_then(Value::as_str).filter(|r| !r.trim().is_empty()) {
            extra.insert("reasoning".into(), json!(r));
        }
        if let Some(model) = meta.pointer("/info/model").and_then(Value::as_str).filter(|s| !s.is_empty()) {
            extra.insert("model".into(), json!(model));
        }
        let mut row = json!({
            "name": m.name.clone().unwrap_or_else(|| if is_user { chat.user_name.to_string() } else { chat.char_name.to_string() }),
            "is_user": is_user,
            "is_system": meta.get("hidden").and_then(Value::as_bool).unwrap_or(false),
            "send_date": iso(m.created_at),
            "mes": body(&m.content),
            "extra": Value::Object(extra),
        });
        if let Some(sw) = meta.get("swipes").and_then(Value::as_array).filter(|a| !a.is_empty()) {
            let swipes: Vec<Value> = sw
                .iter()
                .map(|s| json!(body(s.get("content").and_then(Value::as_str).unwrap_or_default())))
                .collect();
            // SillyTavern은 swipes와 swipe_info 길이가 같다고 여긴다
            let info: Vec<Value> = sw
                .iter()
                .map(|s| {
                    let at = s.get("createdAt").and_then(Value::as_i64).unwrap_or(m.created_at);
                    json!({ "send_date": iso(at), "extra": {} })
                })
                .collect();
            let id = meta.get("swipeId").and_then(Value::as_u64).unwrap_or(0).min(swipes.len() as u64 - 1);
            row["swipes"] = Value::Array(swipes);
            row["swipe_id"] = json!(id);
            row["swipe_info"] = Value::Array(info);
        }
        lines.push(row.to_string());
    }
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::import::sillytavern::chat::{author_note, map_messages, parse_jsonl};

    fn msg(id: &str, role: &str, content: &str, meta: Value) -> Message {
        Message {
            id: id.into(),
            conversation_id: "c".into(),
            seq: 0,
            role: role.into(),
            name: None,
            content: content.into(),
            character_id: None,
            meta: Some(meta),
            created_at: 1_700_000_000_000,
            updated_at: 1_700_000_000_000,
        }
    }

    #[test]
    fn game_tags_are_removed_and_choices_become_lines() {
        let raw = "유나: 어서 와! <emotion::유나>happy</emotion><sfx>문</sfx>\n<effects>snow</effects>\n<choice>인사한다</choice><choice>지나간다</choice>\n<updateVar>{\"set\":{\"x\":1}}</updateVar>";
        assert_eq!(strip_game_tags(raw), "유나: 어서 와!\n- 인사한다\n- 지나간다");
    }

    #[test]
    fn cg_tags_become_img_macros() {
        assert_eq!(cg_to_img("웃는다 <cg> smile </cg>! <CG>cry</CG>"), "웃는다 {{img::smile}}! {{img::cry}}");
        assert_eq!(cg_to_img("열린 <cg>태그\n다음 줄</cg>"), "열린 <cg>태그\n다음 줄</cg>", "줄을 넘는 것은 그대로");
    }

    #[test]
    fn exported_chat_reads_back_through_the_sillytavern_importer() {
        let msgs = vec![
            msg("1", "assistant", "안녕 <cg>smile</cg>", json!({ "info": { "model": "m1" } })),
            msg("2", "user", "응", json!({ "hidden": true })),
            msg(
                "3",
                "assistant",
                "둘째",
                json!({ "swipeId": 1, "swipes": [{ "content": "첫째" }, { "content": "둘째", "createdAt": 1_700_000_100_000_i64 }] }),
            ),
            msg("4", "system", "(장면 전환)", json!({})),
        ];
        let meta = json!({ "authorNote": { "text": "비가 온다", "depth": 2 } });
        let out = to_jsonl(
            &JsonlChat {
                user_name: "나",
                char_name: "아리아",
                created_at: 1_700_000_000_000,
                conv_meta: Some(&meta),
                messages: &msgs,
            },
            &JsonlOptions { cg_as_img: true },
        );
        let (header, raw) = parse_jsonl(out.as_bytes()).unwrap();
        assert_eq!(header["character_name"], "아리아");
        let mut w = vec![];
        assert_eq!(author_note(&header["chat_metadata"], &mut w), Some(json!({ "text": "비가 온다", "depth": 2 })));
        assert!(w.is_empty(), "빈도·위치 경고 없음");
        let back = map_messages(&raw, &mut w);
        assert!(w.is_empty(), "스와이프 경고 없음: {w:?}");
        assert_eq!(back.len(), 4);
        assert_eq!(back[0].content, "안녕 {{img::smile}}");
        assert!(back[1].hidden);
        assert_eq!(back[2].meta["swipeId"], 1);
        assert_eq!(back[2].meta["swipes"].as_array().unwrap().len(), 2);
        assert_eq!(back[3].role, "system", "나레이터");
        assert_eq!(raw[2]["swipe_info"].as_array().unwrap().len(), 2);
    }
}
