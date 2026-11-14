//! SillyTavern regex 스크립트 → RegexScriptInput
//!
//! ST 필드: scriptName, findRegex, replaceString, trimStrings, placement(숫자 배열),
//! disabled, markdownOnly, promptOnly, runOnEdit, substituteRegex(0/1/2 또는 bool), minDepth, maxDepth
use serde_json::Value;

use crate::models::regex::{MacroMode, RegexPlacement, RegexScope, RegexScriptInput, RegexTarget};

use super::{as_str, str_list};

/// ST placement 번호 → 우리 placement. 1 사용자 입력, 2 AI 출력, 5 월드인포.
/// 0(옛 화면용), 3(슬래시 커맨드), 6(추론)은 지원하지 않는다.
fn placement(n: i64) -> Option<RegexPlacement> {
    match n {
        1 => Some(RegexPlacement::UserInput),
        2 => Some(RegexPlacement::AiOutput),
        5 => Some(RegexPlacement::WorldInfo),
        _ => None,
    }
}

/// 스크립트 하나 변환. 쓸 수 없는 스크립트면 None과 함께 경고를 남긴다.
pub fn map_regex_script(
    v: &Value,
    scope: RegexScope,
    scope_id: Option<String>,
    sort_order: i64,
    warnings: &mut Vec<String>,
) -> Option<RegexScriptInput> {
    let name = as_str(v, "scriptName").unwrap_or("이름 없는 정규식").to_string();
    let find = as_str(v, "findRegex").unwrap_or_default().to_string();
    if find.is_empty() {
        warnings.push(format!("정규식 '{name}': 찾을 패턴이 비어 있어 건너뜀"));
        return None;
    }

    let raw_places: Vec<i64> = v["placement"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_default();
    let places: Vec<RegexPlacement> = raw_places.iter().filter_map(|n| placement(*n)).collect();
    if places.len() < raw_places.len() {
        warnings.push(format!(
            "정규식 '{name}': 슬래시 커맨드/추론 등 지원하지 않는 적용 위치는 제외함"
        ));
    }
    if places.is_empty() {
        warnings.push(format!("정규식 '{name}': 지원하는 적용 위치가 없어 건너뜀"));
        return None;
    }

    let flag = |k: &str| v[k].as_bool().unwrap_or(false);
    let target = if flag("markdownOnly") {
        RegexTarget::Display
    } else if flag("promptOnly") {
        RegexTarget::Prompt
    } else {
        RegexTarget::Stored
    };
    let substitute = match &v["substituteRegex"] {
        Value::Bool(true) => MacroMode::Raw,
        Value::Number(n) => match n.as_i64() {
            Some(1) => MacroMode::Raw,
            Some(2) => MacroMode::Escaped,
            _ => MacroMode::None,
        },
        _ => MacroMode::None,
    };
    let depth = |k: &str| v[k].as_i64().filter(|d| *d >= 0);

    Some(RegexScriptInput {
        id: None,
        name,
        find_regex: find,
        replace_string: as_str(v, "replaceString").unwrap_or_default().to_string(),
        trim_strings: str_list(&v["trimStrings"]),
        placement: places,
        target,
        scope,
        scope_id,
        substitute_macros: substitute,
        run_on_edit: v["runOnEdit"].as_bool().unwrap_or(true),
        min_depth: depth("minDepth"),
        max_depth: depth("maxDepth"),
        enabled: !flag("disabled"),
        sort_order,
        source_format: Some("sillytavern".into()),
        extra_json: Some(v.clone()),
    })
}

/// 배열 또는 단일 객체 모두 받는다
pub fn map_regex_scripts(
    v: &Value,
    scope: RegexScope,
    scope_id: Option<String>,
    warnings: &mut Vec<String>,
) -> Vec<RegexScriptInput> {
    let items: Vec<&Value> = match v {
        Value::Array(a) => a.iter().collect(),
        Value::Object(_) => vec![v],
        _ => vec![],
    };
    items
        .into_iter()
        .enumerate()
        .filter_map(|(i, s)| map_regex_script(s, scope, scope_id.clone(), i as i64, warnings))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_fields_and_flags() {
        let mut w = vec![];
        let s = map_regex_script(
            &json!({
                "id": "x", "scriptName": "Hide thoughts", "findRegex": "/<think>[\\s\\S]*?<\\/think>/g",
                "replaceString": "", "trimStrings": ["a"], "placement": [2, 6], "disabled": true,
                "markdownOnly": false, "promptOnly": true, "runOnEdit": false, "substituteRegex": 2,
                "minDepth": null, "maxDepth": 3
            }),
            RegexScope::Character,
            Some("c1".into()),
            4,
            &mut w,
        )
        .unwrap();
        assert_eq!(s.placement, vec![RegexPlacement::AiOutput]);
        assert_eq!(s.target, RegexTarget::Prompt);
        assert_eq!(s.substitute_macros, MacroMode::Escaped);
        assert!(!s.enabled && !s.run_on_edit);
        assert_eq!((s.min_depth, s.max_depth), (None, Some(3)));
        assert_eq!(s.scope_id.as_deref(), Some("c1"));
        assert_eq!(s.sort_order, 4);
        assert_eq!(w.len(), 1, "추론(6) placement 경고");
    }

    #[test]
    fn skips_unusable_scripts() {
        let mut w = vec![];
        let list = map_regex_scripts(
            &json!([
                { "scriptName": "slash only", "findRegex": "/a/", "placement": [3] },
                { "scriptName": "empty", "findRegex": "", "placement": [1] },
                { "scriptName": "ok", "findRegex": "/a/", "placement": [1], "markdownOnly": true }
            ]),
            RegexScope::Global,
            None,
            &mut w,
        );
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].target, RegexTarget::Display);
        assert_eq!(w.len(), 3);
    }
}
