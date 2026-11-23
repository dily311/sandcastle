//! 대화 변수 정의 (5-F-4-3, ADR-029): 캐릭터에 두는 변수의 설명·종류·범위·AI 읽기/쓰기.
//! 정의는 선택이다 — 정의 없는 변수는 지금처럼 이름=글자(SillyTavern chat 변수와 같음).
//! 게임 변수 정의(`game_state_defs`)와 같은 뜻의 칸을 쓴다 (게임 → 대화 복사에서 그대로 옮기게, 5-F-4-6).
use std::collections::HashSet;

use crate::{
    error::{AppError, Result},
    models::conversation::{VarDef, VarKind},
};

/// 캐릭터 하나에 둘 수 있는 정의 수 (프롬프트에 들어갈 수 있어 넉넉하되 끝없이는 아님)
pub const MAX_DEFS: usize = 50;
pub const MAX_KEY: usize = 64;
pub const MAX_DESCRIPTION: usize = 500;
pub const MAX_VALUE: usize = 4000;

/// 저장 전에 확인하고 다듬는다: 이름은 앞뒤 공백을 빼고 겹치지 않게, 범위는 최소 ≤ 최대, 폭은 0보다 크게,
/// 수가 아닌 종류는 범위·폭을 지운다, AI가 쓸 수 있으면 읽기도 켠다(지금 값을 알아야 바꿀 수 있음 — 게임과 같음)
pub fn normalize(defs: Vec<VarDef>) -> Result<Vec<VarDef>> {
    if defs.len() > MAX_DEFS {
        return Err(AppError::validation(format!("변수 정의는 {MAX_DEFS}개까지"), None));
    }
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(defs.len());
    for mut d in defs {
        d.key = d.key.trim().to_string();
        if d.key.is_empty() {
            return Err(AppError::validation("변수 이름이 비어 있음", None));
        }
        if d.key.chars().count() > MAX_KEY {
            return Err(AppError::validation(format!("변수 이름은 {MAX_KEY}자까지: {}", d.key), None));
        }
        if d.key.contains("::") || d.key.contains("}}") {
            return Err(AppError::validation(format!("변수 이름에 '::'·'}}}}'는 쓸 수 없음: {}", d.key), None));
        }
        if !seen.insert(d.key.clone()) {
            return Err(AppError::validation(format!("같은 이름의 변수가 둘: {}", d.key), None));
        }
        d.description = d.description.trim().chars().take(MAX_DESCRIPTION).collect();
        d.default_value = d.default_value.map(|v| v.chars().take(MAX_VALUE).collect());
        if d.kind == VarKind::Number {
            if let (Some(lo), Some(hi)) = (d.min, d.max) {
                if lo > hi {
                    return Err(AppError::validation(format!("'{}'의 최소가 최대보다 큼", d.key), None));
                }
            }
            if d.max_step.is_some_and(|s| s <= 0.0 || !s.is_finite()) {
                return Err(AppError::validation(format!("'{}'의 한 번 폭은 0보다 커야 함", d.key), None));
            }
            if let Some(v) = d.default_value.as_deref().filter(|v| !v.trim().is_empty()) {
                if v.trim().parse::<f64>().is_err() {
                    return Err(AppError::validation(format!("'{}'의 처음 값이 수가 아님: {v}", d.key), None));
                }
            }
        } else {
            d.min = None;
            d.max = None;
            d.max_step = None;
        }
        if d.kind == VarKind::Boolean {
            if let Some(v) = d.default_value.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
                if v != "true" && v != "false" {
                    return Err(AppError::validation(format!("'{}'의 처음 값은 true 또는 false", d.key), None));
                }
            }
        }
        if d.ai_write {
            d.ai_read = true;
        }
        out.push(d);
    }
    Ok(out)
}

// =======================================
// AI가 변수를 바꾸기 (5-F-4-4, ADR-029 ②③)
// =======================================

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// 한 응답에서 받는 변경 수 (게임과 같은 생각: 한 번에 조금씩)
pub const MAX_CHANGES: usize = 5;
/// 한 응답에서 읽는 <updateVar> 태그 수
const MAX_TAGS: usize = 3;

/// 메시지에 남기는 바뀐 몫 (meta.varChanges, 스와이프마다). from이 None이면 그 전에는 값이 없었다
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VarChange {
    pub key: String,
    pub from: Option<String>,
    pub to: String,
}

/// 대화 meta.variables (글자 값. 스크립트가 넣은 목록·객체는 JSON 글로 본다)
pub fn current_vars(meta: Option<&Value>) -> Map<String, Value> {
    meta.and_then(|m| m.get("variables")).and_then(Value::as_object).cloned().unwrap_or_default()
}

fn as_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// 지금 값: 대화에 있으면 그것, 없으면 정의의 처음 값
fn value_of(vars: &Map<String, Value>, d: &VarDef) -> Option<String> {
    vars.get(&d.key).map(as_text).or_else(|| d.default_value.clone()).filter(|v| !v.is_empty())
}

/// 엔진(plan_ai_changes)이 읽는 모양: 종류에 맞는 JSON 값
fn typed(d: &VarDef, raw: Option<&str>) -> Value {
    let Some(raw) = raw else { return Value::Null };
    match d.kind {
        VarKind::Number => raw
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        VarKind::Boolean => match raw.trim() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => Value::Null,
        },
        VarKind::String => Value::String(raw.to_string()),
    }
}

/// AI에게 보이는 변수(읽기 허용)가 있나
pub fn any_visible(defs: &[VarDef]) -> bool {
    defs.iter().any(|d| d.ai_read || d.ai_write)
}

/// AI 변수 규칙: 보이는 변수의 지금 값·범위를 앱 프롬프트 슬롯(`vars`, {{var_list}})에 채운다 — 프리셋의 {{vars}} 자리에 (영어 — 앱 지시문, 5-E-21, ADR-030)
pub fn prompt_rule(defs: &[VarDef], vars: &Map<String, Value>) -> Option<String> {
    let visible: Vec<&VarDef> = defs.iter().filter(|d| d.ai_read || d.ai_write).collect();
    if visible.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    for d in &visible {
        let kind = match d.kind {
            VarKind::Number => "number",
            VarKind::Boolean => "true/false",
            VarKind::String => "text",
        };
        let mut notes = vec![kind.to_string()];
        if d.kind == VarKind::Number {
            match (d.min, d.max) {
                (Some(lo), Some(hi)) => notes.push(format!("{lo}..{hi}")),
                (Some(lo), None) => notes.push(format!("min {lo}")),
                (None, Some(hi)) => notes.push(format!("max {hi}")),
                _ => {}
            }
            if d.ai_write {
                if let Some(s) = d.max_step {
                    notes.push(format!("at most ±{s} per reply"));
                }
            }
        }
        notes.push(if d.ai_write { "writable".into() } else { "read-only".into() });
        let desc = if d.description.is_empty() {
            String::new()
        } else {
            format!(" — {}", d.description)
        };
        let value = value_of(vars, d).unwrap_or_else(|| "(unset)".into());
        lines.push(format!("- {} ({}){desc}: {value}", d.key, notes.join(", ")));
    }
    Some(crate::service::app_prompts::vars(&lines.join("\n")))
}

/// 응답에서 <updateVar> 태그를 떼어 낸다 (본문, 태그 안 글들)
pub fn take_tags(text: &str) -> (String, Vec<String>) {
    let (body, tags) = crate::game::npc::extract_tag(text, "updateVar");
    (body.trim_end().to_string(), tags)
}

/// 태그를 정의(AI 쓰기 허용·범위·한 번 폭·종류)로 걸러 바뀔 몫으로. 게임 엔진의 plan_ai_changes와 같은 규칙
pub fn plan(defs: &[VarDef], vars: &Map<String, Value>, tags: &[String]) -> Vec<VarChange> {
    use crate::game::engine::{plan_ai_changes, VarRule};
    let rules: std::collections::HashMap<String, VarRule> = defs
        .iter()
        .map(|d| {
            let rule = VarRule {
                readable: d.ai_read || d.ai_write,
                writable: d.ai_write,
                min: d.min,
                max: d.max,
                max_step: d.max_step,
            };
            (d.key.clone(), rule)
        })
        .collect();
    let mut flags = Map::new();
    for d in defs {
        let v = typed(d, value_of(vars, d).as_deref());
        if !v.is_null() {
            flags.insert(d.key.clone(), v);
        }
    }
    let mut flags = Value::Object(flags);
    let mut out: Vec<VarChange> = vec![];
    for tag in tags.iter().take(MAX_TAGS) {
        let Ok(effect) = serde_json::from_str::<Value>(tag) else { continue };
        for (key, _old, new) in plan_ai_changes(&flags, &effect, &rules, MAX_CHANGES.saturating_sub(out.len())) {
            let from = vars.get(&key).map(as_text);
            let to = as_text(&new);
            flags[&key] = new;
            match out.iter_mut().find(|c| c.key == key) {
                Some(c) => c.to = to, // 한 응답에서 두 번 바뀌면 마지막 값 (앞 값은 처음 것)
                None => out.push(VarChange { key, from, to }),
            }
        }
    }
    out.retain(|c| c.from.as_deref() != Some(c.to.as_str()));
    out
}

/// 바뀐 몫을 적용
pub fn apply(vars: &mut Map<String, Value>, changes: &[VarChange]) {
    for c in changes {
        vars.insert(c.key.clone(), Value::String(c.to.clone()));
    }
}

/// 되돌리기: 그 뒤로 다른 것이 바꾸지 않은 변수만(지금 값이 그 메시지가 남긴 값일 때) 앞 값으로. 거꾸로 한다
pub fn revert(vars: &mut Map<String, Value>, changes: &[VarChange]) {
    for c in changes.iter().rev() {
        if vars.get(&c.key).map(as_text).as_deref() != Some(c.to.as_str()) {
            continue;
        }
        match &c.from {
            Some(v) => {
                vars.insert(c.key.clone(), Value::String(v.clone()));
            }
            None => {
                vars.remove(&c.key);
            }
        }
    }
}

/// 메시지 meta의 바뀐 몫 (지금 보이는 스와이프의 것)
pub fn changes_of(meta: Option<&Value>) -> Vec<VarChange> {
    meta.and_then(|m| m.get("varChanges"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// 대화 meta에 넣을 patch: 바뀐 키만 (지운 키는 null — json_patch가 지운다)
pub fn meta_patch(before: &Map<String, Value>, after: &Map<String, Value>) -> Option<Value> {
    let mut vars = Map::new();
    for (k, v) in after {
        if before.get(k) != Some(v) {
            vars.insert(k.clone(), v.clone());
        }
    }
    for k in before.keys() {
        if !after.contains_key(k) {
            vars.insert(k.clone(), Value::Null);
        }
    }
    (!vars.is_empty()).then(|| serde_json::json!({ "variables": vars }))
}

// ---- 되돌리기 (5-F-4-4-2): 스와이프 넘기기·메시지 지우기·분기 ----

/// 대화 변수에 몫을 바꿔 넣는다: `undo`를 되돌리고(뒤에서부터) `redo`를 적용. 바뀐 게 없으면 쓰지 않는다
pub async fn swap_changes(db: &sqlx::SqlitePool, conversation_id: &str, undo: &[VarChange], redo: &[VarChange]) -> crate::error::Result<()> {
    if undo.is_empty() && redo.is_empty() {
        return Ok(());
    }
    let conv = crate::db::conversation::get_conversation(db, conversation_id).await?;
    let before = current_vars(conv.meta.as_ref());
    let mut after = before.clone();
    revert(&mut after, undo);
    apply(&mut after, redo);
    if let Some(patch) = meta_patch(&before, &after) {
        crate::db::chat_ui::patch_conversation_meta(db, conversation_id, &patch).await?;
    }
    Ok(())
}

/// 지운(지울) 메시지들이 바꾼 몫을 되돌린다 — 뒤 메시지부터
pub async fn revert_messages(db: &sqlx::SqlitePool, conversation_id: &str, messages: &[crate::models::conversation::Message]) -> crate::error::Result<()> {
    let mut sorted: Vec<&crate::models::conversation::Message> = messages.iter().filter(|m| m.conversation_id == conversation_id).collect();
    sorted.sort_by_key(|a| std::cmp::Reverse(a.seq));
    // 뒤에서부터 차례로 되돌린 것을 한 번에 쓰려고 몫을 이어 붙인다 (revert가 거꾸로 돈다 → 앞 메시지 몫이 뒤에 오게)
    let undo: Vec<VarChange> = sorted.iter().rev().flat_map(|m| changes_of(m.meta.as_ref())).collect();
    swap_changes(db, conversation_id, &undo, &[]).await
}

/// 분기: 분기 지점(seq) 뒤 메시지들이 바꾼 몫을 되돌린 값을 새 대화 변수로
pub async fn branch_vars(db: &sqlx::SqlitePool, source_id: &str, new_id: &str, after_seq: i64) -> crate::error::Result<()> {
    let messages = crate::db::conversation::get_messages(db, source_id, i64::MAX, None).await?;
    let later: Vec<crate::models::conversation::Message> = messages.into_iter().filter(|m| m.seq > after_seq).collect();
    revert_messages(
        db,
        new_id,
        &later
            .into_iter()
            .map(|mut m| {
                m.conversation_id = new_id.to_string();
                m
            })
            .collect::<Vec<_>>(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(key: &str) -> VarDef {
        VarDef {
            key: key.into(),
            description: String::new(),
            kind: VarKind::String,
            default_value: None,
            min: None,
            max: None,
            max_step: None,
            ai_read: false,
            ai_write: false,
        }
    }

    #[test]
    fn normalizes_and_rejects_bad_definitions() {
        let hp = VarDef {
            kind: VarKind::Number,
            min: Some(0.0),
            max: Some(100.0),
            max_step: Some(10.0),
            default_value: Some("100".into()),
            ai_write: true,
            ..def("  hp ")
        };
        let mood = VarDef {
            min: Some(1.0),
            max_step: Some(2.0),
            ..def("mood")
        };
        let out = normalize(vec![hp, mood]).unwrap();
        assert_eq!(out[0].key, "hp", "이름 앞뒤 공백을 뺌");
        assert!(out[0].ai_read, "AI가 쓸 수 있으면 읽기도");
        assert_eq!((out[1].min, out[1].max_step), (None, None), "수가 아니면 범위·폭을 지움");

        assert!(normalize(vec![def("a"), def(" a")]).is_err(), "이름이 겹침");
        assert!(normalize(vec![def("")]).is_err(), "빈 이름");
        assert!(normalize(vec![def("a::b")]).is_err(), "매크로를 깨는 이름");
        assert!(
            normalize(vec![VarDef {
                kind: VarKind::Number,
                min: Some(5.0),
                max: Some(1.0),
                ..def("x")
            }])
            .is_err(),
            "최소 > 최대"
        );
        assert!(
            normalize(vec![VarDef {
                kind: VarKind::Number,
                max_step: Some(0.0),
                ..def("x")
            }])
            .is_err(),
            "폭 0"
        );
        assert!(
            normalize(vec![VarDef {
                kind: VarKind::Number,
                default_value: Some("많음".into()),
                ..def("x")
            }])
            .is_err(),
            "수가 아닌 처음 값"
        );
        assert!(
            normalize(vec![VarDef {
                kind: VarKind::Boolean,
                default_value: Some("yes".into()),
                ..def("x")
            }])
            .is_err(),
            "참거짓이 아닌 처음 값"
        );
        assert!(normalize((0..=MAX_DEFS).map(|i| def(&format!("v{i}"))).collect()).is_err(), "개수 상한");
    }

    fn hp_def() -> VarDef {
        VarDef {
            kind: VarKind::Number,
            min: Some(0.0),
            max: Some(100.0),
            max_step: Some(10.0),
            default_value: Some("100".into()),
            ai_write: true,
            ai_read: true,
            ..def("hp")
        }
    }

    #[test]
    fn plans_only_allowed_changes_within_rules() {
        let defs = vec![
            hp_def(),
            VarDef {
                kind: VarKind::Boolean,
                ai_write: true,
                ai_read: true,
                ..def("met")
            },
            VarDef {
                ai_read: true,
                ..def("secret")
            }, // 읽기만
        ];
        let vars: Map<String, Value> = serde_json::from_value(serde_json::json!({ "secret": "x" })).unwrap();
        let tags = vec![
            r#"{"add":{"hp":-30},"set":{"met":true,"secret":"y","nope":1}}"#.to_string(),
            "깨진 JSON".to_string(),
        ];
        let changes = plan(&defs, &vars, &tags);
        assert_eq!(
            changes,
            vec![
                VarChange {
                    key: "met".into(),
                    from: None,
                    to: "true".into()
                },
                // 처음 값 100에서 한 번 폭 10까지만
                VarChange {
                    key: "hp".into(),
                    from: None,
                    to: "90".into()
                },
            ]
        );
        let (body, tags) = take_tags("문을 연다.\n<updateVar>{\"add\":{\"hp\":-3}}</updateVar>");
        assert_eq!((body.as_str(), tags.len()), ("문을 연다.", 1));
    }

    #[test]
    fn revert_only_when_value_is_still_what_the_message_left() {
        let mut vars: Map<String, Value> = serde_json::from_value(serde_json::json!({ "hp": "50" })).unwrap();
        let changes = vec![
            VarChange {
                key: "hp".into(),
                from: Some("50".into()),
                to: "40".into(),
            },
            VarChange {
                key: "met".into(),
                from: None,
                to: "true".into(),
            },
        ];
        let before = vars.clone();
        apply(&mut vars, &changes);
        assert_eq!(
            meta_patch(&before, &vars).unwrap(),
            serde_json::json!({ "variables": { "hp": "40", "met": "true" } })
        );
        // 사용자가 그 뒤로 hp를 고쳤으면 hp는 두고, met만 되돌린다(없던 값이면 지움)
        vars.insert("hp".into(), "77".into());
        let mid = vars.clone();
        revert(&mut vars, &changes);
        assert_eq!(vars.get("hp"), Some(&Value::String("77".into())));
        assert!(!vars.contains_key("met"));
        assert_eq!(meta_patch(&mid, &vars).unwrap(), serde_json::json!({ "variables": { "met": null } }));
    }

    #[test]
    fn prompt_lists_visible_variables_with_current_values() {
        let defs = vec![
            hp_def(),
            def("hidden"),
            VarDef {
                ai_read: true,
                description: "기분".into(),
                ..def("mood")
            },
        ];
        let vars: Map<String, Value> = serde_json::from_value(serde_json::json!({ "mood": "calm" })).unwrap();
        let rule = prompt_rule(&defs, &vars).unwrap();
        assert!(rule.contains("- hp (number, 0..100, at most ±10 per reply, writable): 100"), "{rule}");
        assert!(rule.contains("- mood (text, read-only) — 기분: calm"), "{rule}");
        assert!(!rule.contains("hidden"));
        assert!(rule.contains("<updateVar>"));
        assert!(prompt_rule(&[def("x")], &vars).is_none(), "보이는 변수가 없으면 규칙도 없음");
    }
}
