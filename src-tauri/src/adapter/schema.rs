//! 앱이 받는 JSON Schema **부분집합** (CLAUDE.md 3-A-2·3-A-3, ADR-013).
//!
//! 도구의 `parameters`와 구조화 출력의 스키마가 모두 이 모양이다. 세 공급자(OpenAI·Claude·Gemini)가 공통으로 받는 것만 허용해서
//! "어느 공급자에서는 되고 다른 곳에서는 400"이 되는 일을 줄이고, 공급자를 부르기 **전에** 쓸 수 없는 스키마를 거절한다(요금 없음).
//! 결과는 공급자가 보장한다고 믿지 않고 `validate`로 앱이 다시 검사한다.
//!
//! 허용 키워드: type(문자열 또는 배열) · properties · required · items(스키마 하나) · enum · const · minimum · maximum ·
//! minLength · maxLength · minItems · maxItems · additionalProperties(true/false) · anyOf · description · title · default · examples
//! 거절: `$ref`·`$defs`·`allOf`·`oneOf`·`not`·`pattern`·`patternProperties`·배열 `items`(튜플) 등 그 밖의 모든 것
use serde_json::{json, Map, Value};

const MAX_DEPTH: usize = 8;
const MAX_BYTES: usize = 32 * 1024;
const MAX_PROBLEMS: usize = 20;

const TYPES: [&str; 7] = ["string", "number", "integer", "boolean", "object", "array", "null"];
/// 검증에 쓰이지 않는 설명용 키워드
const NOTES: [&str; 4] = ["description", "title", "default", "examples"];

// ---------------------------------------------------------------------------
// 스키마 검사
// ---------------------------------------------------------------------------

/// 쓸 수 있는 스키마인가. 문제 목록(빈 목록이면 통과). `root_object`면 최상위가 object여야 한다 (도구·구조화 출력은 object만)
pub fn check_schema(schema: &Value, root_object: bool) -> Vec<String> {
    let mut problems = vec![];
    if schema.to_string().len() > MAX_BYTES {
        problems.push(format!("스키마가 너무 큼 ({}KB 초과)", MAX_BYTES / 1024));
        return problems;
    }
    if root_object && type_names(schema).as_deref() != Some(&["object".to_string()][..]) {
        problems.push("$: 최상위는 type이 object여야 함".into());
    }
    check_node(schema, "$", 0, &mut problems);
    problems.truncate(MAX_PROBLEMS);
    problems
}

/// `type` 값을 이름 목록으로 (없으면 None)
fn type_names(node: &Value) -> Option<Vec<String>> {
    match node.get("type")? {
        Value::String(s) => Some(vec![s.clone()]),
        Value::Array(a) => a.iter().map(|v| v.as_str().map(String::from)).collect(),
        _ => None,
    }
}

fn check_node(node: &Value, path: &str, depth: usize, out: &mut Vec<String>) {
    let Some(obj) = node.as_object() else {
        out.push(format!("{path}: 스키마는 객체여야 함"));
        return;
    };
    if depth > MAX_DEPTH {
        out.push(format!("{path}: 너무 깊음 (최대 {MAX_DEPTH}단계)"));
        return;
    }
    for (key, value) in obj {
        let here = format!("{path}.{key}");
        match key.as_str() {
            k if NOTES.contains(&k) => {}
            "type" => match type_names(node) {
                Some(names) if !names.is_empty() && names.iter().all(|n| TYPES.contains(&n.as_str())) => {}
                _ => out.push(format!("{here}: type은 {} 중 하나(또는 그 배열)", TYPES.join("/"))),
            },
            "properties" => match value.as_object() {
                Some(props) => props
                    .iter()
                    .for_each(|(name, child)| check_node(child, &format!("{path}.{name}"), depth + 1, out)),
                None => out.push(format!("{here}: 객체여야 함")),
            },
            "required" => {
                if !value.as_array().is_some_and(|a| a.iter().all(Value::is_string)) {
                    out.push(format!("{here}: 글자 목록이어야 함"));
                }
            }
            "items" => check_node(value, &format!("{path}[]"), depth + 1, out),
            "anyOf" => match value.as_array() {
                Some(alts) if !alts.is_empty() => alts
                    .iter()
                    .enumerate()
                    .for_each(|(i, a)| check_node(a, &format!("{path}.anyOf[{i}]"), depth + 1, out)),
                _ => out.push(format!("{here}: 비어 있지 않은 목록이어야 함")),
            },
            "enum" => {
                if !value.as_array().is_some_and(|a| !a.is_empty()) {
                    out.push(format!("{here}: 비어 있지 않은 목록이어야 함"));
                }
            }
            "const" => {}
            "minimum" | "maximum" => {
                if !value.is_number() {
                    out.push(format!("{here}: 숫자여야 함"));
                }
            }
            "minLength" | "maxLength" | "minItems" | "maxItems" => {
                if value.as_u64().is_none() {
                    out.push(format!("{here}: 0 이상의 정수여야 함"));
                }
            }
            "additionalProperties" => {
                if !value.is_boolean() {
                    out.push(format!("{here}: true/false만 가능 (스키마는 지원하지 않음)"));
                }
            }
            other => out.push(format!("{here}: 지원하지 않는 키워드 '{other}'")),
        }
    }
}

// ---------------------------------------------------------------------------
// 값 검증 (공급자가 스키마를 지켰다고 믿지 않는다)
// ---------------------------------------------------------------------------

/// 값이 스키마에 맞는가. 문제 목록(빈 목록이면 통과). 스키마는 `check_schema`를 통과한 것이라고 본다
pub fn validate(schema: &Value, value: &Value) -> Vec<String> {
    let mut out = vec![];
    validate_node(schema, value, "$", &mut out);
    out.truncate(MAX_PROBLEMS);
    out
}

fn matches_type(name: &str, v: &Value) -> bool {
    match name {
        "string" => v.is_string(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        "object" => v.is_object(),
        "array" => v.is_array(),
        "number" => v.is_number(),
        "integer" => v.as_i64().is_some() || v.as_u64().is_some() || v.as_f64().is_some_and(|f| f.fract() == 0.0 && f.is_finite()),
        _ => false,
    }
}

fn kind_of(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "문자열",
        Value::Array(_) => "배열",
        Value::Object(_) => "객체",
    }
}

fn validate_node(schema: &Value, v: &Value, path: &str, out: &mut Vec<String>) {
    if out.len() >= MAX_PROBLEMS {
        return;
    }
    if let Some(alts) = schema.get("anyOf").and_then(Value::as_array) {
        if !alts.iter().any(|a| {
            let mut sub = vec![];
            validate_node(a, v, path, &mut sub);
            sub.is_empty()
        }) {
            out.push(format!("{path}: anyOf의 어느 것에도 맞지 않음"));
            return;
        }
    }
    if let Some(names) = type_names(schema) {
        if !names.iter().any(|n| matches_type(n, v)) {
            out.push(format!("{path}: {}이어야 하는데 {}임", names.join("/"), kind_of(v)));
            return;
        }
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array) {
        if !options.contains(v) {
            out.push(format!("{path}: 허용된 값이 아님 (enum)"));
        }
    }
    if let Some(c) = schema.get("const") {
        if c != v {
            out.push(format!("{path}: 정해진 값(const)과 다름"));
        }
    }
    match v {
        Value::Object(map) => {
            let props = schema.get("properties").and_then(Value::as_object);
            if let Some(required) = schema.get("required").and_then(Value::as_array) {
                for r in required.iter().filter_map(Value::as_str) {
                    if !map.contains_key(r) {
                        out.push(format!("{path}.{r}: 필수인데 없음"));
                    }
                }
            }
            if let Some(props) = props {
                for (k, child_schema) in props {
                    if let Some(child) = map.get(k) {
                        validate_node(child_schema, child, &format!("{path}.{k}"), out);
                    }
                }
            }
            if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                for k in map.keys().filter(|k| props.is_none_or(|p| !p.contains_key(*k))) {
                    out.push(format!("{path}.{k}: 스키마에 없는 항목"));
                }
            }
        }
        Value::Array(items) => {
            if let Some(n) = schema.get("minItems").and_then(Value::as_u64) {
                if (items.len() as u64) < n {
                    out.push(format!("{path}: 항목이 {n}개 이상이어야 함 ({}개)", items.len()));
                }
            }
            if let Some(n) = schema.get("maxItems").and_then(Value::as_u64) {
                if (items.len() as u64) > n {
                    out.push(format!("{path}: 항목이 {n}개 이하여야 함 ({}개)", items.len()));
                }
            }
            if let Some(item_schema) = schema.get("items") {
                for (i, item) in items.iter().enumerate() {
                    validate_node(item_schema, item, &format!("{path}[{i}]"), out);
                }
            }
        }
        Value::String(s) => {
            let len = s.chars().count() as u64;
            if schema.get("minLength").and_then(Value::as_u64).is_some_and(|n| len < n) {
                out.push(format!("{path}: 너무 짧음 (최소 {}자)", schema["minLength"]));
            }
            if schema.get("maxLength").and_then(Value::as_u64).is_some_and(|n| len > n) {
                out.push(format!("{path}: 너무 김 (최대 {}자)", schema["maxLength"]));
            }
        }
        Value::Number(n) => {
            let f = n.as_f64().unwrap_or(f64::NAN);
            if schema.get("minimum").and_then(Value::as_f64).is_some_and(|m| f < m) {
                out.push(format!("{path}: {} 이상이어야 함", schema["minimum"]));
            }
            if schema.get("maximum").and_then(Value::as_f64).is_some_and(|m| f > m) {
                out.push(format!("{path}: {} 이하여야 함", schema["maximum"]));
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// 공급자별 변환
// ---------------------------------------------------------------------------

/// OpenAI `strict: true`가 받는 모양인가: 모든 object가 `additionalProperties:false`이고 `required`가 모든 properties이며,
/// 길이·범위 같은 제약 키워드가 없다. 아니면 strict를 끄고(네이티브 json_schema는 그대로 쓰되) 결과는 앱이 검증한다
pub fn is_strict_ready(schema: &Value) -> bool {
    let Some(obj) = schema.as_object() else { return false };
    for key in obj.keys() {
        if matches!(
            key.as_str(),
            "minimum" | "maximum" | "minLength" | "maxLength" | "minItems" | "maxItems" | "const"
        ) {
            return false;
        }
    }
    if type_names(schema).is_some_and(|n| n.iter().any(|t| t == "object")) {
        if schema.get("additionalProperties") != Some(&Value::Bool(false)) {
            return false;
        }
        let props: Vec<&String> = schema
            .get("properties")
            .and_then(Value::as_object)
            .map(|p| p.keys().collect())
            .unwrap_or_default();
        let required: Vec<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        if props.iter().any(|p| !required.contains(&p.as_str())) {
            return false;
        }
    }
    let children_ok = obj.get("properties").and_then(Value::as_object).is_none_or(|p| p.values().all(is_strict_ready))
        && obj.get("items").is_none_or(is_strict_ready)
        && obj.get("anyOf").and_then(Value::as_array).is_none_or(|a| a.iter().all(is_strict_ready));
    children_ok
}

/// Gemini/Vertex의 OpenAPI 부분집합 `Schema`로: 모르는 키워드를 빼고, type을 대문자 열거값으로, `["string","null"]`은
/// `STRING + nullable`로, `const`는 한 값짜리 `enum`으로. (길이 제약 minLength/maxLength는 일부 버전이 모르므로 뺀다 — 앱 검증이 대신 본다)
pub fn to_gemini_schema(schema: &Value) -> Value {
    let Some(obj) = schema.as_object() else { return schema.clone() };
    let mut out = Map::new();
    if let Some(names) = type_names(schema) {
        let non_null: Vec<&String> = names.iter().filter(|n| *n != "null").collect();
        if let Some(first) = non_null.first() {
            out.insert("type".into(), json!(first.to_uppercase()));
        } else if names.iter().any(|n| n == "null") {
            out.insert("type".into(), json!("STRING"));
        }
        if names.iter().any(|n| n == "null") {
            out.insert("nullable".into(), json!(true));
        }
    }
    for key in ["description", "minimum", "maximum", "minItems", "maxItems"] {
        if let Some(v) = obj.get(key) {
            out.insert(key.into(), v.clone());
        }
    }
    // Gemini의 enum은 문자열만 된다 — 숫자 enum이면 빼고(type은 남김) 앱 검증이 대신 본다
    let strings = |e: &Value| e.as_array().is_some_and(|a| a.iter().all(Value::is_string));
    if let Some(e) = obj.get("enum").filter(|e| strings(e)) {
        out.insert("enum".into(), e.clone());
    } else if let Some(c) = obj.get("const").filter(|c| c.is_string()) {
        out.insert("enum".into(), json!([c]));
    }
    if let Some(props) = obj.get("properties").and_then(Value::as_object) {
        out.insert(
            "properties".into(),
            Value::Object(props.iter().map(|(k, v)| (k.clone(), to_gemini_schema(v))).collect()),
        );
    }
    if let Some(r) = obj.get("required") {
        out.insert("required".into(), r.clone());
    }
    if let Some(items) = obj.get("items") {
        out.insert("items".into(), to_gemini_schema(items));
    }
    if let Some(alts) = obj.get("anyOf").and_then(Value::as_array) {
        out.insert("anyOf".into(), Value::Array(alts.iter().map(to_gemini_schema).collect()));
    }
    Value::Object(out)
}

// ---------------------------------------------------------------------------
// 글에서 JSON 뽑기 (구조화 출력 대체 경로)
// ---------------------------------------------------------------------------

/// 모델 응답에서 JSON 값 하나를 뽑는다: 통째로 JSON이면 그대로, 아니면 코드 펜스를 벗기고, 그래도 아니면 글 속의 첫
/// 균형 잡힌 `{…}`/`[…]`. 문자열 안의 괄호는 세지 않는다. 못 뽑으면 None
pub fn extract_json(text: &str) -> Option<Value> {
    let t = text.trim();
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        return Some(v);
    }
    let unfenced = strip_fence(t);
    if let Ok(v) = serde_json::from_str::<Value>(unfenced) {
        return Some(v);
    }
    let bytes = unfenced.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' || bytes[i] == b'[' {
            if let Some(end) = balanced_end(unfenced, i) {
                if let Ok(v) = serde_json::from_str::<Value>(&unfenced[i..=end]) {
                    return Some(v);
                }
            }
        }
        i += 1;
    }
    None
}

fn strip_fence(t: &str) -> &str {
    let Some(rest) = t.strip_prefix("```") else { return t };
    let rest = rest.split_once('\n').map_or(rest, |(_, body)| body); // ```json 줄 버리기
    rest.rsplit_once("```").map_or(rest, |(body, _)| body).trim()
}

/// `start`의 여는 괄호와 짝이 맞는 닫는 괄호 위치 (바이트 인덱스). 글자열 안의 괄호는 건너뜀
fn balanced_end(s: &str, start: usize) -> Option<usize> {
    let mut depth = 0i32;
    let (mut in_str, mut escaped) = (false, false);
    for (off, c) in s[start..].char_indices() {
        if in_str {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_str = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' | '[' => depth += 1,
            '}' | ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + off);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person() -> Value {
        json!({
            "type": "object",
            "description": "사람",
            "properties": {
                "name": { "type": "string", "minLength": 1 },
                "age": { "type": "integer", "minimum": 0, "maximum": 150 },
                "mood": { "enum": ["happy", "sad"] },
                "tags": { "type": "array", "items": { "type": "string" }, "maxItems": 3 },
                "nick": { "type": ["string", "null"] }
            },
            "required": ["name", "age"],
            "additionalProperties": false
        })
    }

    #[test]
    fn accepts_the_subset_and_rejects_the_rest() {
        assert!(check_schema(&person(), true).is_empty(), "{:?}", check_schema(&person(), true));
        for (bad, why) in [
            (json!({ "type": "object", "properties": { "a": { "$ref": "#/$defs/x" } } }), "$ref"),
            (json!({ "type": "object", "oneOf": [] }), "oneOf"),
            (
                json!({ "type": "object", "properties": { "a": { "type": "string", "pattern": "^a" } } }),
                "pattern",
            ),
            (
                json!({ "type": "object", "additionalProperties": { "type": "string" } }),
                "additionalProperties",
            ),
            (json!({ "type": "object", "properties": { "a": { "type": "date" } } }), "type"),
            (
                json!({ "type": "object", "properties": { "a": { "type": "array", "items": [{ "type": "string" }] } } }),
                "items 배열",
            ),
        ] {
            assert!(!check_schema(&bad, true).is_empty(), "{why}를 거절해야 함");
        }
        assert!(!check_schema(&json!({ "type": "array", "items": {} }), true).is_empty(), "최상위는 object");
        assert!(check_schema(&json!({ "type": "array", "items": { "type": "string" } }), false).is_empty());
        // 너무 깊은 스키마
        let mut deep = json!({ "type": "string" });
        for _ in 0..12 {
            deep = json!({ "type": "object", "properties": { "a": deep } });
        }
        assert!(check_schema(&deep, true).iter().any(|p| p.contains("너무 깊음")));
        // 속성 이름이 키워드와 같아도 속성으로 읽는다
        assert!(check_schema(
            &json!({ "type": "object", "properties": { "type": { "type": "string" }, "enum": { "type": "string" } } }),
            true
        )
        .is_empty());
    }

    #[test]
    fn validates_values() {
        let s = person();
        assert!(validate(&s, &json!({ "name": "아리아", "age": 20, "mood": "happy", "tags": ["a"], "nick": null })).is_empty());
        let problems = validate(&s, &json!({ "age": 200.5, "mood": "angry", "tags": ["a", "b", "c", "d"], "extra": 1 }));
        let all = problems.join(" | ");
        for want in [
            "$.name: 필수",
            "$.age: integer",
            "$.mood",
            "$.tags: 항목이 3개 이하",
            "$.extra: 스키마에 없는 항목",
        ] {
            assert!(all.contains(want), "{want} 없음: {all}");
        }
        assert!(validate(&s, &json!({ "name": "", "age": 1 })).iter().any(|p| p.contains("너무 짧음")));
        assert!(validate(&s, &json!({ "name": "x", "age": -1 })).iter().any(|p| p.contains("0 이상")));
        // 정수로 보이는 실수는 integer로 인정, 아니면 거절
        assert!(validate(&json!({ "type": "integer" }), &json!(3.0)).is_empty());
        assert!(!validate(&json!({ "type": "integer" }), &json!(3.5)).is_empty());
        // anyOf·const
        let any = json!({ "anyOf": [{ "type": "string" }, { "type": "integer" }] });
        assert!(validate(&any, &json!("x")).is_empty() && validate(&any, &json!(2)).is_empty());
        assert!(!validate(&any, &json!(true)).is_empty());
        assert!(!validate(&json!({ "const": "a" }), &json!("b")).is_empty());
        // 문제가 아무리 많아도 상한
        let big = json!({ "type": "array", "items": { "type": "string" } });
        let many: Vec<Value> = (0..100).map(|i| json!(i)).collect();
        assert!(validate(&big, &Value::Array(many)).len() <= MAX_PROBLEMS);
    }

    #[test]
    fn openai_strict_only_when_the_schema_already_fits() {
        assert!(!is_strict_ready(&person()), "선택 항목과 제약이 있어 strict 불가");
        let ok = json!({
            "type": "object",
            "properties": { "a": { "type": "string" }, "b": { "type": "array", "items": { "type": "object", "properties": { "c": { "type": "integer" } }, "required": ["c"], "additionalProperties": false } } },
            "required": ["a", "b"],
            "additionalProperties": false
        });
        assert!(is_strict_ready(&ok));
        let mut nested_open = ok.clone();
        nested_open["properties"]["b"]["items"]["additionalProperties"] = json!(true);
        assert!(!is_strict_ready(&nested_open), "안쪽 object도 닫혀 있어야 함");
    }

    #[test]
    fn converts_to_the_gemini_subset() {
        let g = to_gemini_schema(&person());
        assert_eq!(g["type"], "OBJECT");
        assert!(g.get("additionalProperties").is_none());
        assert_eq!(g["properties"]["age"]["type"], "INTEGER");
        assert_eq!(g["properties"]["age"]["maximum"], 150);
        assert!(g["properties"]["name"].get("minLength").is_none(), "길이 제약은 뺌");
        assert_eq!(g["properties"]["nick"]["type"], "STRING");
        assert_eq!(g["properties"]["nick"]["nullable"], true);
        assert_eq!(g["properties"]["mood"]["enum"], json!(["happy", "sad"]));
        assert_eq!(g["properties"]["tags"]["items"]["type"], "STRING");
        assert_eq!(g["required"], json!(["name", "age"]));
        assert_eq!(to_gemini_schema(&json!({ "const": "x" }))["enum"], json!(["x"]));
        assert!(
            to_gemini_schema(&json!({ "type": "integer", "enum": [1, 2] })).get("enum").is_none(),
            "숫자 enum은 Gemini가 거절해서 뺌"
        );
    }

    #[test]
    fn extracts_json_from_chatty_text() {
        assert_eq!(extract_json(r#"{"a":1}"#), Some(json!({ "a": 1 })));
        assert_eq!(extract_json("```json\n{\"a\": [1, 2]}\n```"), Some(json!({ "a": [1, 2] })));
        assert_eq!(
            extract_json("물론이죠! 결과는 다음과 같아요:\n{\"a\": \"}\"}\n도움이 되었길!"),
            Some(json!({ "a": "}" })),
            "글자열 안의 괄호는 세지 않음"
        );
        assert_eq!(extract_json("앞 [1,2] 뒤"), Some(json!([1, 2])));
        assert_eq!(extract_json("{ 깨진 {\"ok\": true}"), Some(json!({ "ok": true })), "깨진 앞부분은 건너뜀");
        assert_eq!(extract_json("JSON이 없어요"), None);
        assert_eq!(extract_json("{\"a\": 1"), None, "닫히지 않음");
    }
}
