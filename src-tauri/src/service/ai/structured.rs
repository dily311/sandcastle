//! 구조화 출력의 앱 쪽 로직 (CLAUDE.md 3-A-3, ADR-013): 네이티브를 못 쓸 때의 대체 지시문, 결과 검증, 잘림 판정.
//!
//! 공급자가 스키마를 지켰다고 믿지 않는다 — 네이티브든 대체든 결과는 늘 `parse_output`이 JSON으로 읽고 스키마로 검사한다.
//! 실패해도 자동으로 다시 부르지 않는다(이미 쓴 토큰은 청구된다). 호출한 쪽이 `AiStructuredOutputInvalid`의 details를 보고 정한다.
use serde_json::{json, Value};

use crate::{
    adapter::{
        schema::{extract_json, validate},
        ResponseSchema, StopKind,
    },
    error::{AppError, ErrorCode, Result},
};

/// 오류 details에 넣는 원문 길이 (너무 길면 기록이 부푼다)
const RAW_KEEP: usize = 2000;

/// 네이티브 구조화 출력이 없는 연결에서 프롬프트 끝에 덧붙이는 system 지시
pub fn instruction(rs: &ResponseSchema) -> String {
    let schema = serde_json::to_string_pretty(&rs.schema).unwrap_or_default();
    let what = rs.description.as_deref().map(|d| format!("\nDescription: {d}")).unwrap_or_default();
    format!("[Response format]{what}\nReply with exactly one JSON value that matches the JSON Schema below. No code fences (```), explanations, preface or closing remarks.\n{schema}")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

/// 응답 글 → 스키마를 만족하는 JSON 값. 잘렸으면 `AiOutputTruncated`, 읽지 못하거나 안 맞으면 `AiStructuredOutputInvalid`
pub fn parse_output(raw: &str, stop: StopKind, rs: &ResponseSchema, native: bool) -> Result<Value> {
    let problems: Vec<String> = match extract_json(raw) {
        Some(v) => {
            let p = validate(&rs.schema, &v);
            if p.is_empty() {
                return Ok(v);
            }
            p
        }
        None => vec!["JSON을 읽을 수 없음".to_string()],
    };
    if stop == StopKind::MaxTokens {
        return Err(truncated("구조화 출력"));
    }
    Err(AppError::AiProvider {
        code: ErrorCode::AiStructuredOutputInvalid,
        message: format!("응답이 '{}' 스키마에 맞지 않음: {}", rs.name, problems.join("; ")),
        retryable: false,
        details: Some(json!({ "problems": problems, "raw": truncate(raw, RAW_KEEP), "native": native, "stop": format!("{stop:?}") })),
    })
}

pub fn truncated(what: &str) -> AppError {
    AppError::AiProvider {
        code: ErrorCode::AiOutputTruncated,
        message: format!("{what}이(가) 출력 한도(max_tokens)에서 잘려 끊김 — 실행하지 않음. 출력 한도를 늘려 다시 시도하세요 (이미 쓴 토큰은 사용량에 기록됨)"),
        retryable: false,
        details: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rs() -> ResponseSchema {
        ResponseSchema {
            name: "mood".into(),
            description: Some("기분".into()),
            schema: json!({ "type": "object", "properties": { "mood": { "enum": ["happy", "sad"] }, "score": { "type": "integer", "minimum": 0 } }, "required": ["mood"], "additionalProperties": false }),
        }
    }

    #[test]
    fn instruction_carries_the_schema_and_forbids_chatter() {
        let i = instruction(&rs());
        assert!(i.contains("\"enum\"") && i.contains("기분") && i.contains("code fences"), "{i}");
    }

    #[test]
    fn accepts_chatty_but_valid_output_and_rejects_the_rest() {
        let ok = parse_output("물론이죠!\n```json\n{\"mood\": \"happy\", \"score\": 3}\n```", StopKind::End, &rs(), false).unwrap();
        assert_eq!(ok, json!({ "mood": "happy", "score": 3 }));

        let e = parse_output("{\"mood\": \"angry\", \"extra\": 1}", StopKind::End, &rs(), true).unwrap_err();
        let r = e.to_response();
        assert_eq!(r.code, ErrorCode::AiStructuredOutputInvalid);
        assert!(!r.retryable, "자동으로 다시 부르지 않음");
        let d = r.details.unwrap();
        assert_eq!(d["native"], true);
        assert!(d["problems"].as_array().unwrap().len() >= 2 && d["raw"].as_str().unwrap().contains("angry"));

        let e = parse_output("JSON이 아님", StopKind::End, &rs(), false).unwrap_err().to_response();
        assert_eq!(e.code, ErrorCode::AiStructuredOutputInvalid);
    }

    #[test]
    fn a_cut_off_answer_is_reported_as_truncated_not_invalid() {
        let e = parse_output("{\"mood\": \"hap", StopKind::MaxTokens, &rs(), true).unwrap_err().to_response();
        assert_eq!(e.code, ErrorCode::AiOutputTruncated);
        assert!(e.message.contains("max_tokens"));
        // 끝까지 온 완전한 값이면 한도에 닿았어도 받아들인다
        assert!(parse_output("{\"mood\": \"sad\"}", StopKind::MaxTokens, &rs(), true).is_ok());
    }

    #[test]
    fn long_raw_is_trimmed_in_details() {
        let long = format!("{}{}", "가".repeat(5000), "x");
        let d = parse_output(&long, StopKind::End, &rs(), false).unwrap_err().to_response().details.unwrap();
        assert!(d["raw"].as_str().unwrap().chars().count() <= RAW_KEEP + 1);
    }
}
