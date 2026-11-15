//! SillyTavern Chat Completion 프리셋 → prompt_presets + prompt_blocks
//!
//! - `prompts`: 블록 정의 (identifier, name, role, content, marker, injection_position/depth/trigger)
//! - `prompt_order`: 캐릭터별 배치. 전역 배치(character_id 100001) → 기본(100000) → 첫 번째 순으로 고른다.
//!   배치에 없는 블록도 버리지 않고 꺼진 상태로 뒤에 붙인다.
//! - 생성 설정(temperature, openai_max_tokens …)은 우리 키로 옮겨 settings_json에 넣는다.
//! - 원본 전체는 compatibility_json에 보존한다.
use serde_json::{json, Map, Value};

use crate::models::{
    ai_service::{PromptBlocksInput, PromptPresetsInput},
    regex::{RegexScope, RegexScriptInput},
};

use super::{as_str, regex::map_regex_scripts};

/// ST가 실제 내용 없이 "여기에 넣어라"로 쓰는 마커 중 우리가 채우지 못하는 것
const UNSUPPORTED_MARKERS: [&str; 2] = ["charPersonality", "scenario"];
const DEFAULT_IN_CHAT_DEPTH: i64 = 4;

pub struct ImportedPreset {
    pub preset: PromptPresetsInput,
    pub blocks: Vec<PromptBlocksInput>,
    pub regex: Vec<RegexScriptInput>,
    pub warnings: Vec<String>,
}

/// 프리셋에 묶인 정규식을 모두 모은다. 위치가 확장마다 달라서 넓게 찾는다:
/// - extensions.regex_scripts (SillyTavern 기본: 프리셋에 묶인 정규식)
/// - 최상위 regex_scripts
/// - extensions 아래 아무 곳(깊이 4까지)의 배열 중 findRegex를 가진 항목이 있는 것 (예: SPreset의 RegexBinding.regexes)
/// 같은 이름·같은 찾기식은 한 번만.
pub fn preset_regex(json: &Value) -> Value {
    fn is_regex_list(v: &Value) -> bool {
        v.as_array().is_some_and(|a| a.iter().any(|x| x.get("findRegex").is_some()))
    }
    fn walk<'a>(v: &'a Value, depth: usize, out: &mut Vec<&'a Value>) {
        if depth > 4 {
            return;
        }
        match v {
            Value::Array(a) if is_regex_list(v) => out.extend(a.iter().filter(|x| x.get("findRegex").is_some())),
            Value::Object(o) => o.values().for_each(|c| walk(c, depth + 1, out)),
            _ => {}
        }
    }
    let mut found: Vec<&Value> = Vec::new();
    if is_regex_list(&json["regex_scripts"]) {
        walk(&json["regex_scripts"], 0, &mut found);
    }
    // 기본 위치를 먼저 (확장 객체의 키 순서는 보장되지 않는다)
    walk(&json["extensions"]["regex_scripts"], 0, &mut found);
    walk(&json["extensions"], 0, &mut found);
    let mut seen = std::collections::HashSet::new();
    let unique: Vec<Value> = found
        .into_iter()
        .filter(|s| seen.insert((s.get("scriptName").and_then(Value::as_str).unwrap_or("").to_string(), s.get("findRegex").and_then(Value::as_str).unwrap_or("").to_string())))
        .cloned()
        .collect();
    Value::Array(unique)
}

pub fn map_preset(json: &Value, preset_id: &str, name: &str) -> ImportedPreset {
    let mut warnings = vec![];
    let prompts: Vec<&Value> = json["prompts"].as_array().map(|a| a.iter().collect()).unwrap_or_default();
    let order = pick_order(json);

    // (블록 정의, 켜짐 여부) 를 배치 순서대로
    let mut placed: Vec<(&Value, bool)> = vec![];
    match &order {
        Some(order) => {
            for item in order {
                let Some(id) = as_str(item, "identifier") else { continue };
                if let Some(p) = prompts.iter().find(|p| as_str(p, "identifier") == Some(id)) {
                    placed.push((p, item["enabled"].as_bool().unwrap_or(true)));
                }
            }
            for p in &prompts {
                let id = as_str(p, "identifier");
                if !order.iter().any(|o| as_str(o, "identifier") == id) {
                    placed.push((p, false));
                }
            }
        }
        None => placed.extend(prompts.iter().map(|p| (*p, p["enabled"].as_bool().unwrap_or(true)))),
    }

    let mut blocks = vec![];
    let mut merged_markers = false;
    for (i, (p, enabled)) in placed.into_iter().enumerate() {
        let Some(identifier) = as_str(p, "identifier").filter(|s| !s.is_empty()) else { continue };
        let marker = p["marker"].as_bool().unwrap_or(false);
        if enabled && UNSUPPORTED_MARKERS.contains(&identifier) && !merged_markers {
            // ST 기본 프리셋에 항상 있는 마커라 경고는 한 번만
            merged_markers = true;
            warnings.push(
                "성격(charPersonality)/시나리오(scenario) 블록은 비워 둠 — 캐릭터를 가져올 때 두 항목은 캐릭터 설명에 합쳐짐".into(),
            );
        }
        let role = match as_str(p, "role").unwrap_or("system") {
            r @ ("system" | "user" | "assistant") => r,
            _ => "system",
        };
        let in_chat = p["injection_position"].as_i64() == Some(1);

        let mut extra = Map::new();
        for k in ["system_prompt", "marker", "injection_order", "forbid_overrides"] {
            if !p[k].is_null() {
                extra.insert(k.to_string(), p[k].clone());
            }
        }

        blocks.push(PromptBlocksInput {
            id: None,
            preset_id: preset_id.to_string(),
            name: as_str(p, "name").filter(|s| !s.is_empty()).unwrap_or(identifier).to_string(),
            identifier: identifier.to_string(),
            block_type: "text".into(),
            role: role.into(),
            content: if marker { String::new() } else { as_str(p, "content").unwrap_or_default().to_string() },
            enabled,
            position: if in_chat { "in_chat" } else { "relative" }.into(),
            depth: if in_chat {
                p["injection_depth"].as_i64().filter(|d| *d >= 0).unwrap_or(DEFAULT_IN_CHAT_DEPTH)
            } else {
                0
            },
            sort_order: i as i64,
            triggers_json: match &p["injection_trigger"] {
                Value::Array(a) => Value::Array(a.clone()),
                _ => json!([]),
            },
            extra_json: (!extra.is_empty()).then_some(Value::Object(extra)),
        });
    }
    if blocks.is_empty() {
        warnings.push("프롬프트 블록이 없어 기본 배치로 동작함".into());
    }

    // 생성 설정
    let mut settings = Map::new();
    let mut copy = |to: &str, from: &[&str]| {
        if let Some(v) = from.iter().map(|k| &json[*k]).find(|v| !v.is_null()) {
            settings.insert(to.to_string(), v.clone());
        }
    };
    copy("temperature", &["temperature"]);
    copy("max_tokens", &["openai_max_tokens"]);
    copy("context_size", &["openai_max_context"]);
    copy("top_p", &["top_p"]);
    copy("top_k", &["top_k"]);
    copy("frequency_penalty", &["frequency_penalty"]);
    copy("presence_penalty", &["presence_penalty"]);
    copy("stream", &["stream_openai"]);

    if as_str(json, "assistant_prefill").is_some_and(|s| !s.trim().is_empty()) {
        warnings.push("assistant prefill은 아직 지원하지 않아 원본에만 보존함".into());
    }

    let regex = map_regex_scripts(
        &preset_regex(json),
        RegexScope::Preset,
        Some(preset_id.to_string()),
        &mut warnings,
    );

    ImportedPreset {
        preset: PromptPresetsInput {
            id: Some(preset_id.to_string()),
            name: name.to_string(),
            description: None,
            preset_type: "chat_completion".into(),
            source_format: "sillytavern".into(),
            config_json: json!({}),
            settings_json: Some(Value::Object(settings)),
            compatibility_json: Some(strip_secrets(json)),
        },
        blocks,
        regex,
        warnings,
    }
}

/// ST 프리셋에는 연결 설정이 같이 들어 있다. 비밀번호·인증 헤더는 원본 보존용으로도 저장하지 않는다
/// (DB에 평문으로 남고, 데이터 내보내기 때 함께 나갈 수 있음).
const SECRET_KEYS: [&str; 3] = ["proxy_password", "custom_include_headers", "api_key"];

fn strip_secrets(json: &Value) -> Value {
    let mut v = json.clone();
    if let Some(obj) = v.as_object_mut() {
        for k in SECRET_KEYS {
            obj.remove(k);
        }
    }
    v
}

fn pick_order(json: &Value) -> Option<Vec<Value>> {
    let orders = json["prompt_order"].as_array()?;
    let by_char = |id: i64| orders.iter().find(|o| o["character_id"].as_i64() == Some(id));
    let chosen = by_char(100001).or_else(|| by_char(100000)).or_else(|| orders.first())?;
    chosen["order"].as_array().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Value {
        json!({
            "temperature": 0.9, "openai_max_tokens": 500, "openai_max_context": 16000,
            "top_k": 0, "stream_openai": true, "assistant_prefill": "Sure,",
            "proxy_password": "hunter2", "custom_include_headers": "Authorization: Bearer x",
            "prompts": [
                { "identifier": "main", "name": "Main Prompt", "system_prompt": true, "role": "system", "content": "Be {{char}}." },
                { "identifier": "chatHistory", "name": "Chat History", "marker": true, "system_prompt": true },
                { "identifier": "charDescription", "marker": true },
                { "identifier": "scenario", "marker": true },
                { "identifier": "note", "name": "Note", "role": "user", "content": "Stay short",
                  "injection_position": 1, "injection_depth": 2, "injection_trigger": ["normal", "swipe"] },
                { "identifier": "unused", "name": "Unused", "content": "x" }
            ],
            "prompt_order": [
                { "character_id": 100000, "order": [ { "identifier": "main", "enabled": false } ] },
                { "character_id": 100001, "order": [
                    { "identifier": "main", "enabled": true },
                    { "identifier": "charDescription", "enabled": true },
                    { "identifier": "scenario", "enabled": true },
                    { "identifier": "chatHistory", "enabled": true },
                    { "identifier": "note", "enabled": true },
                    { "identifier": "ghost", "enabled": true }
                ]}
            ],
            "extensions": { "regex_scripts": [ { "scriptName": "r", "findRegex": "/a/", "placement": [2] } ] }
        })
    }

    #[test]
    fn maps_order_markers_and_injection() {
        let p = map_preset(&sample(), "p1", "My preset");
        let ids: Vec<_> = p.blocks.iter().map(|b| (b.identifier.as_str(), b.enabled)).collect();
        assert_eq!(
            ids,
            vec![
                ("main", true),
                ("charDescription", true),
                ("scenario", true),
                ("chatHistory", true),
                ("note", true),
                ("unused", false),
            ]
        );
        let note = &p.blocks[4];
        assert_eq!((note.position.as_str(), note.depth, note.role.as_str()), ("in_chat", 2, "user"));
        assert_eq!(note.triggers_json, json!(["normal", "swipe"]));
        assert_eq!(p.blocks[1].content, "", "마커는 내용 없음");
        assert_eq!(p.blocks[0].content, "Be {{char}}.");

        let s = p.preset.settings_json.unwrap();
        assert_eq!(s["max_tokens"], 500);
        assert_eq!(s["context_size"], 16000);
        assert_eq!(s["stream"], true);
        assert_eq!(p.preset.source_format, "sillytavern");
        let compat = p.preset.compatibility_json.unwrap();
        assert!(compat["prompts"].is_array());
        assert!(compat.get("proxy_password").is_none() && compat.get("custom_include_headers").is_none());

        assert_eq!(p.regex.len(), 1);
        assert_eq!(p.regex[0].scope_id.as_deref(), Some("p1"));
        // scenario 마커 + prefill
        assert_eq!(p.warnings.len(), 2, "{:?}", p.warnings);
    }

    #[test]
    fn without_prompt_order_uses_prompt_list() {
        let p = map_preset(
            &json!({ "prompts": [ { "identifier": "a", "content": "A" }, { "identifier": "b", "enabled": false } ] }),
            "p",
            "n",
        );
        let ids: Vec<_> = p.blocks.iter().map(|b| (b.identifier.as_str(), b.enabled)).collect();
        assert_eq!(ids, vec![("a", true), ("b", false)]);
    }

    #[test]
    fn finds_regex_in_extension_settings() {
        let json = json!({
            "prompts": [],
            "regex_scripts": [ { "scriptName": "top", "findRegex": "/t/", "placement": [2] } ],
            "extensions": {
                "regex_scripts": [ { "scriptName": "r", "findRegex": "/a/", "placement": [2] } ],
                "SPreset": { "RegexBinding": { "regexes": [ { "scriptName": "s", "findRegex": "/b/", "placement": [2] }, { "scriptName": "r", "findRegex": "/a/", "placement": [2] } ] } }
            }
        });
        let names: Vec<_> = preset_regex(&json).as_array().unwrap().iter().map(|v| v["scriptName"].as_str().unwrap().to_string()).collect();
        assert_eq!(names, vec!["top", "r", "s"]);
        assert_eq!(map_preset(&json, "p1", "x").regex.len(), 3);
    }
}
