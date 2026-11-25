//! SillyTavern 채팅 기록 (.jsonl)
//!
//! 1번째 줄은 헤더(`user_name`, `character_name`, `chat_metadata`), 그 뒤로 한 줄에 메시지 하나.
//! 메시지 필드: name, is_user, is_system(숨김), send_date, mes, extra{api, model, reasoning, type},
//! swipes, swipe_id, swipe_info, gen_started, gen_finished
//!
//! 우리 메시지 meta로 옮기는 방식 (프론트 MessageMeta와 같은 모양)
//! - sender { type, name, id? }      : 보낸 쪽
//! - info { api, model, genStarted, genFinished }
//! - swipes / swipeId                : 스와이프가 2개 이상일 때만
//! - hidden: true                    : ST에서 숨긴 메시지 (프롬프트에서 빠짐)
//! - reasoning                       : 추론 내용
//! - sillytavern { extra }           : 원본 extra 보존
use serde_json::{json, Map, Value};

use crate::error::{AppError, Result};

use super::as_str;

/// .jsonl 채팅 파일인지 (첫 줄이 헤더 객체)
pub fn looks_like_chat(bytes: &[u8]) -> bool {
    let first = bytes.split(|b| *b == b'\n').next().unwrap_or_default();
    serde_json::from_slice::<Value>(first)
        .ok()
        .is_some_and(|v| v.get("chat_metadata").is_some() || v.get("user_name").is_some() && v.get("character_name").is_some())
}

/// (헤더, 메시지 목록)
pub fn parse_jsonl(bytes: &[u8]) -> Result<(Value, Vec<Value>)> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let header: Value = lines
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .ok_or_else(|| AppError::validation("채팅 파일의 첫 줄(헤더)을 읽을 수 없습니다", None))?;
    let mut messages = vec![];
    for (i, line) in lines.enumerate() {
        let v: Value = serde_json::from_str(line).map_err(|e| AppError::validation(format!("채팅 {}번째 메시지를 읽을 수 없습니다: {e}", i + 1), None))?;
        messages.push(v);
    }
    Ok((header, messages))
}

pub struct ImportedMessage {
    pub role: String,
    /// 표시 이름 (ST의 name)
    pub name: String,
    pub content: String,
    pub created_at: Option<i64>,
    pub meta: Value,
    pub hidden: bool,
}

impl ImportedMessage {
    pub fn is_user(&self) -> bool {
        self.role == "user"
    }
}

/// ST의 불리언은 가끔 "" / "true" 같은 문자열로 들어온다
fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::String(s) => s.eq_ignore_ascii_case("true"),
        Value::Number(n) => n.as_i64().is_some_and(|n| n != 0),
        _ => false,
    }
}

/// ST 날짜: ISO 8601, 밀리초 숫자, 또는 옛 형식 "March 18, 2026 1:32pm"
pub fn parse_date(v: &Value) -> Option<i64> {
    if let Some(n) = v.as_i64() {
        return Some(n);
    }
    let s = v.as_str()?.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis());
    }
    for fmt in ["%B %d, %Y %I:%M%p", "%B %d, %Y %I:%M %p", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            return Some(dt.and_utc().timestamp_millis());
        }
    }
    None
}

fn info_from(extra: &Value, gen_started: &Value, gen_finished: &Value) -> Option<Value> {
    let mut info = Map::new();
    for k in ["api", "model"] {
        if let Some(s) = as_str(extra, k).filter(|s| !s.is_empty()) {
            info.insert(k.into(), json!(s));
        }
    }
    if let Some(t) = parse_date(gen_started) {
        info.insert("genStarted".into(), json!(t));
    }
    if let Some(t) = parse_date(gen_finished) {
        info.insert("genFinished".into(), json!(t));
    }
    (!info.is_empty()).then_some(Value::Object(info))
}

pub fn map_messages(raw: &[Value], warnings: &mut Vec<String>) -> Vec<ImportedMessage> {
    let mut out = vec![];
    let mut broken_swipes = 0;
    for m in raw {
        let content = as_str(m, "mes").unwrap_or_default().to_string();
        let extra = m.get("extra").cloned().unwrap_or(Value::Null);
        let is_user = truthy(&m["is_user"]);
        let is_narrator = as_str(&extra, "type") == Some("narrator");
        let role = if is_user {
            "user"
        } else if is_narrator {
            "system"
        } else {
            "assistant"
        };
        let name = as_str(m, "name").unwrap_or_default().to_string();
        let hidden = truthy(&m["is_system"]);
        let created_at = parse_date(&m["send_date"]);

        let mut meta = Map::new();
        meta.insert("sender".into(), json!({ "type": role, "name": name, "avatar": null, "color": null }));
        if let Some(info) = info_from(&extra, &m["gen_started"], &m["gen_finished"]) {
            meta.insert("info".into(), info);
        }
        if hidden {
            meta.insert("hidden".into(), json!(true));
        }
        if let Some(r) = as_str(&extra, "reasoning").filter(|s| !s.trim().is_empty()) {
            meta.insert("reasoning".into(), json!(r));
        }

        // 스와이프: 2개 이상이고 swipe_id가 범위 안일 때만 의미가 있다
        let swipes: Vec<&str> = m["swipes"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
        let swipe_id = m["swipe_id"].as_u64().map(|n| n as usize);
        if swipes.len() >= 2 {
            let selected = swipe_id.filter(|i| *i < swipes.len()).unwrap_or_else(|| {
                broken_swipes += 1;
                swipes.iter().position(|s| *s == content).unwrap_or(swipes.len() - 1)
            });
            let infos = m["swipe_info"].as_array().cloned().unwrap_or_default();
            let list: Vec<Value> = swipes
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let si = infos.get(i).cloned().unwrap_or(Value::Null);
                    let mut sm = Map::new();
                    if let Some(info) = info_from(&si["extra"], &si["gen_started"], &si["gen_finished"]) {
                        sm.insert("info".into(), info);
                    }
                    let at = parse_date(&si["send_date"]).or(created_at).unwrap_or_default();
                    json!({ "content": s, "meta": sm, "createdAt": at, "updatedAt": at })
                })
                .collect();
            meta.insert("swipes".into(), Value::Array(list));
            meta.insert("swipeId".into(), json!(selected));
        } else if swipe_id.is_some_and(|i| i > 0) {
            broken_swipes += 1;
        }

        if !extra.is_null() && extra.as_object().is_some_and(|o| !o.is_empty()) {
            meta.insert("sillytavern".into(), json!({ "extra": extra }));
        }

        out.push(ImportedMessage {
            role: role.to_string(),
            name,
            content,
            created_at,
            meta: Value::Object(meta),
            hidden,
        });
    }
    if broken_swipes > 0 {
        warnings.push(format!("메시지 {broken_swipes}개의 스와이프 정보가 맞지 않아 현재 표시된 내용만 가져옴"));
    }
    out
}

/// SillyTavern 채팅 헤더의 작가 노트 → 대화 meta.authorNote `{ text, depth }` (없으면 None)
pub fn author_note(meta: &Value, warnings: &mut Vec<String>) -> Option<Value> {
    let text = as_str(meta, "note_prompt").map(str::trim).filter(|s| !s.is_empty())?;
    let depth = meta["note_depth"]
        .as_u64()
        .or_else(|| meta["note_depth"].as_str().and_then(|s| s.trim().parse().ok()))
        .unwrap_or(crate::service::chat::notes::DEFAULT_DEPTH as u64);
    let interval = meta["note_interval"]
        .as_u64()
        .or_else(|| meta["note_interval"].as_str().and_then(|s| s.trim().parse().ok()));
    // ST의 위치: 0 = 시나리오 뒤(대화 밖), 1 = 대화 안 깊이, 2 = 맨 앞
    let position = meta["note_position"].as_u64();
    if interval.is_some_and(|i| i > 1) || position.is_some_and(|p| p != 1) {
        warnings.push("작가 노트의 빈도·위치는 지원하지 않아 '대화 안 깊이'로 가져옴 (원본은 대화 정보에 보존)".into());
    }
    Some(json!({ "text": text, "depth": depth }))
}

/// 시간 순서 보장: 없거나 앞 메시지보다 이르면 앞 메시지 + 1ms
pub fn monotonic_times(messages: &[ImportedMessage], fallback_start: i64) -> Vec<i64> {
    let mut prev: Option<i64> = None;
    messages
        .iter()
        .map(|m| {
            let t = match (m.created_at, prev) {
                (Some(t), Some(p)) if t > p => t,
                (Some(t), None) => t,
                (_, Some(p)) => p + 1,
                (None, None) => fallback_start,
            };
            prev = Some(t);
            t
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_roles_hidden_swipes_and_info() {
        let mut w = vec![];
        let raw = vec![
            json!({ "name": "Sera", "is_user": false, "is_system": true, "mes": "A", "swipe_id": 5,
                    "swipes": ["A"], "send_date": "2026-11-08T22:28:31.667Z" }),
            json!({ "name": "Me", "is_user": true, "is_system": "", "mes": "hi" }),
            json!({ "name": "Sera", "is_user": false, "mes": "two", "swipe_id": 1, "swipes": ["one", "two"],
                    "swipe_info": [{ "extra": { "model": "m1" } }, { "extra": { "model": "m2" } }],
                    "extra": { "api": "vertexai", "model": "m2", "reasoning": "hmm" },
                    "gen_started": "2026-11-08T22:30:57.302Z", "gen_finished": "2026-11-08T22:31:15.459Z" }),
            json!({ "name": "Narrator", "is_user": false, "mes": "Night falls.", "extra": { "type": "narrator" } }),
        ];
        let msgs = map_messages(&raw, &mut w);
        let roles: Vec<_> = msgs.iter().map(|m| m.role.as_str()).collect();
        assert_eq!(roles, vec!["assistant", "user", "assistant", "system"]);
        assert!(msgs[0].hidden && msgs[0].meta["hidden"] == true);
        assert!(!msgs[1].hidden, "빈 문자열은 false");
        assert!(msgs[0].meta.get("swipes").is_none(), "스와이프 1개 + 잘못된 swipe_id는 무시");
        assert_eq!(w.len(), 1);

        let m = &msgs[2].meta;
        assert_eq!(m["swipeId"], 1);
        assert_eq!(m["swipes"][0]["content"], "one");
        assert_eq!(m["swipes"][0]["meta"]["info"]["model"], "m1");
        assert_eq!(m["info"]["api"], "vertexai");
        assert!(m["info"]["genFinished"].as_i64().unwrap() > m["info"]["genStarted"].as_i64().unwrap());
        assert_eq!(m["reasoning"], "hmm");
        assert_eq!(m["sender"]["name"], "Sera");
    }

    #[test]
    fn dates_and_ordering() {
        assert_eq!(parse_date(&json!("2026-11-08T22:28:31.667Z")), Some(1794176911667));
        assert!(parse_date(&json!("March 18, 2026 1:32pm")).is_some());
        assert_eq!(parse_date(&json!(123)), Some(123));
        assert_eq!(parse_date(&json!("")), None);

        let msg = |t: Option<i64>| ImportedMessage {
            role: "user".into(),
            name: String::new(),
            content: String::new(),
            created_at: t,
            meta: Value::Null,
            hidden: false,
        };
        let times = monotonic_times(&[msg(None), msg(Some(50)), msg(Some(10)), msg(None)], 100);
        assert_eq!(times, vec![100, 101, 102, 103]);
        let times = monotonic_times(&[msg(Some(5)), msg(Some(9))], 0);
        assert_eq!(times, vec![5, 9]);
    }

    #[test]
    fn author_note_from_header() {
        let mut w = vec![];
        assert_eq!(
            author_note(
                &json!({ "note_prompt": " 비가 온다 ", "note_depth": 2, "note_interval": 1, "note_position": 1 }),
                &mut w
            ),
            Some(json!({ "text": "비가 온다", "depth": 2 }))
        );
        assert!(w.is_empty());
        assert_eq!(
            author_note(&json!({ "note_prompt": "x", "note_interval": 3 }), &mut w),
            Some(json!({ "text": "x", "depth": 4 }))
        );
        assert_eq!(w.len(), 1);
        assert_eq!(author_note(&json!({ "note_prompt": "  " }), &mut w), None);
    }

    #[test]
    fn detects_jsonl_header() {
        assert!(looks_like_chat(b"{\"user_name\":\"u\",\"character_name\":\"c\"}\n{\"mes\":\"x\"}"));
        assert!(!looks_like_chat(b"{\"prompts\":[]}"));
    }
}
