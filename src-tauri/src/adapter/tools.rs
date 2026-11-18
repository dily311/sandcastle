//! 도구 호출·구조화 출력의 요청/응답 타입 (CLAUDE.md 3-A-2·3-A-3, ADR-013).
//!
//! 기존 `PromptMessage`(system/user/assistant 글)는 그대로 두고, 도구 루프가 덧붙이는 부분만 `ChatExtras`로 따로 둔다.
//! 도구 루프는 늘 대화 끝에 덧붙으므로(호출 → 결과 → 다시 호출) 꼬리(`tail`)로 충분하고, 기존 프롬프트 조립·저장 경로는 건드리지 않는다.
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::schema::check_schema;
use crate::error::{AppError, Result};

/// 도구 하나 (모델에게 "이런 함수를 부를 수 있다"고 알리는 것)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// 인자의 JSON Schema 부분집합 (`adapter::schema`) — 최상위는 object
    pub parameters: Value,
}

/// 도구를 쓰게 할지
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolChoice {
    /// 모델이 정함
    #[default]
    Auto,
    /// 도구를 쓰지 않음
    None,
    /// 아무 도구나 꼭 하나 이상
    Required,
    /// 이 도구를 꼭
    Named(String),
}

/// 모델이 한 도구 호출
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    /// 결과를 돌려줄 때 짝을 맞추는 id. 공급자가 주지 않으면(Gemini) 앱이 `call_<응답표>_<번호>`를 만든다 (루프 전체에서 겹치지 않게)
    pub id: String,
    pub name: String,
    /// 파싱된 인자. `parse_error`가 있으면 빈 객체
    pub arguments: Value,
    /// 모델이 보낸 인자 글 그대로 (기록용·오류 안내용)
    pub raw_arguments: String,
    /// 인자가 JSON이 아니면 이유. 호출을 버리지 않고 남겨서, 루프가 "인자 오류" 결과를 모델에게 돌려줄 수 있게 한다
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_error: Option<String>,
}

/// 도구 실행 결과 (앱이 도구를 실행한 뒤 모델에게 돌려주는 것)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub is_error: bool,
}

/// 공급자가 다음 턴에 **그대로** 돌려받기를 바라는 응답 원본 (Gemini의 parts와 `thoughtSignature`, Claude의 thinking 블록 등).
/// 앱은 내용을 해석하지 않고, 같은 공급자 형식으로 이어 보낼 때만 되돌려 보낸다 (공급자가 바뀌면 버리고 글과 호출로 다시 만든다)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderState {
    /// 어댑터 형식 이름: "openai" / "claude" / "gemini"
    pub provider: String,
    pub raw: Value,
}

/// 대화 끝에 덧붙는 도구 루프 한 턴
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolTurn {
    /// 모델의 직전 응답: 글과 도구 호출들 (+ 공급자 원본)
    Assistant {
        text: String,
        calls: Vec<ToolCall>,
        state: Option<ProviderState>,
    },
    /// 그 호출들의 결과 (호출마다 하나). `note`는 결과 뒤에 붙이는 사용자 글 (예: "남은 단계 2")
    Results { results: Vec<ToolResult>, note: Option<String> },
}

/// 응답이 멈춘 이유 (공급자 문자열을 정규화한 것. 원문은 `Generation.finish_reason`)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopKind {
    /// 알 수 없음·그 밖
    #[default]
    Other,
    /// 끝까지 답함
    End,
    /// 도구를 부르려고 멈춤
    ToolUse,
    /// 출력 한도(max_tokens)에서 잘림
    MaxTokens,
    /// 안전 필터·저작권 등으로 막힘
    Filtered,
}

/// 구조화 출력: 응답이 따라야 할 스키마
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseSchema {
    /// 영문·숫자·`_`·`-` 64자 이하 (공급자가 이름을 요구한다)
    pub name: String,
    pub description: Option<String>,
    pub schema: Value,
}

/// 일반 글 생성에 더하는 것. 비어 있으면 기존 `generate`와 똑같다
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChatExtras {
    pub tools: Vec<ToolSpec>,
    pub tool_choice: ToolChoice,
    pub tail: Vec<ToolTurn>,
    pub response_schema: Option<ResponseSchema>,
}

const MAX_TOOLS: usize = 64;
const MAX_TAIL: usize = 100;

/// 세 공급자의 교집합: 글자나 `_`로 시작, 영문·숫자·`_`·`-` 64자 이하 (Gemini는 숫자·`-`로 시작할 수 없음)
fn valid_name(n: &str) -> bool {
    n.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && n.len() <= 64
        && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl ChatExtras {
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty() && self.tail.is_empty() && self.response_schema.is_none() && matches!(self.tool_choice, ToolChoice::Auto)
    }

    /// 공급자를 부르기 전에 모양을 확인한다 (요금이 나가기 전에 거절)
    pub fn validate(&self) -> Result<()> {
        let bad = |msg: String| AppError::validation(msg, None);
        if !self.tools.is_empty() && self.response_schema.is_some() {
            return Err(bad("도구와 구조화 출력은 한 요청에 같이 쓸 수 없음 (공급자마다 조합 지원이 다름)".into()));
        }
        if self.tools.len() > MAX_TOOLS {
            return Err(bad(format!("도구는 {MAX_TOOLS}개까지")));
        }
        let mut seen = std::collections::HashSet::new();
        for t in &self.tools {
            if !valid_name(&t.name) {
                return Err(bad(format!("도구 이름이 올바르지 않음 (영문·숫자·_·- 64자 이하): '{}'", t.name)));
            }
            if !seen.insert(t.name.as_str()) {
                return Err(bad(format!("도구 이름이 겹침: '{}'", t.name)));
            }
            let problems = check_schema(&t.parameters, true);
            if !problems.is_empty() {
                return Err(bad(format!("도구 '{}'의 인자 스키마를 쓸 수 없음: {}", t.name, problems.join("; "))));
            }
        }
        match &self.tool_choice {
            ToolChoice::Named(n) if !self.tools.iter().any(|t| &t.name == n) => return Err(bad(format!("tool_choice가 없는 도구를 가리킴: '{n}'"))),
            ToolChoice::Named(_) | ToolChoice::Required if self.tools.is_empty() => return Err(bad("도구가 없는데 도구를 쓰게 할 수 없음".into())),
            _ => {}
        }
        if !self.tail.is_empty() && self.tools.is_empty() {
            return Err(bad("도구 호출 기록(tail)에는 도구 목록도 필요함".into()));
        }
        if self.tail.len() > MAX_TAIL {
            return Err(bad(format!("도구 호출 기록은 {MAX_TAIL}턴까지")));
        }
        // 호출마다 결과가 하나씩, 같은 id로 돌아와야 한다 (공급자는 짝이 안 맞으면 400). id는 루프 전체에서 겹치면 안 된다
        let mut open: Vec<&str> = vec![];
        let mut all_ids = std::collections::HashSet::new();
        for turn in &self.tail {
            match turn {
                ToolTurn::Assistant { calls, .. } => {
                    if !open.is_empty() {
                        return Err(bad("앞선 도구 호출에 결과가 없음".into()));
                    }
                    for c in calls {
                        if !all_ids.insert(c.id.as_str()) {
                            return Err(bad(format!("도구 호출 id가 겹침: '{}'", c.id)));
                        }
                    }
                    open = calls.iter().map(|c| c.id.as_str()).collect();
                }
                ToolTurn::Results { results, .. } => {
                    let mut ids: Vec<&str> = results.iter().map(|r| r.call_id.as_str()).collect();
                    let mut want = open.clone();
                    ids.sort_unstable();
                    want.sort_unstable();
                    if ids != want {
                        return Err(bad("도구 결과가 직전 호출들과 짝이 맞지 않음 (id가 다르거나 빠짐)".into()));
                    }
                    open.clear();
                }
            }
        }
        if let Some(rs) = &self.response_schema {
            if !valid_name(&rs.name) {
                return Err(bad(format!("구조화 출력 이름이 올바르지 않음 (영문·숫자·_·- 64자 이하): '{}'", rs.name)));
            }
            let problems = check_schema(&rs.schema, true);
            if !problems.is_empty() {
                return Err(bad(format!("구조화 출력 스키마를 쓸 수 없음: {}", problems.join("; "))));
            }
        }
        Ok(())
    }
}

/// 스트림·응답에서 도구 호출 조각을 모은다 (어댑터 공용). 조각은 인덱스(순서)로 이어 붙이고, 끝에 인자를 JSON으로 파싱한다
#[derive(Debug, Default)]
pub(crate) struct CallAssembler {
    parts: Vec<PartialCall>,
}

#[derive(Debug, Default)]
struct PartialCall {
    index: usize,
    id: String,
    name: String,
    args: String,
}

impl CallAssembler {
    fn slot(&mut self, index: usize) -> &mut PartialCall {
        if let Some(i) = self.parts.iter().position(|p| p.index == index) {
            return &mut self.parts[i];
        }
        self.parts.push(PartialCall { index, ..Default::default() });
        self.parts.last_mut().expect("방금 넣음")
    }

    /// 호출을 연다 / 이미 열렸으면 id·이름을 채운다 (호환 서버는 조각마다 id를 되풀이하기도 한다)
    pub fn start(&mut self, index: usize, id: Option<&str>, name: Option<&str>) {
        let p = self.slot(index);
        if let Some(id) = id.filter(|s| !s.is_empty()) {
            p.id = id.to_string();
        }
        if let Some(name) = name.filter(|s| !s.is_empty()) {
            p.name = name.to_string();
        }
    }

    /// 인자 글 조각을 이어 붙인다
    pub fn push_args(&mut self, index: usize, fragment: &str) {
        self.slot(index).args.push_str(fragment);
    }

    /// 모은 호출들 (인덱스 순). id가 없으면 응답마다 다른 표를 붙여 `call_<표>_<번호>`. 이름이 없으면 오류.
    /// 인자가 JSON이 아니면 호출을 버리지 않고 `parse_error`를 단다 (잘렸는지는 호출하는 쪽이 종료 사유로 판단)
    pub fn finish(mut self) -> Result<Vec<ToolCall>> {
        self.parts.sort_by_key(|p| p.index);
        let tag: String = uuid::Uuid::new_v4().simple().to_string().chars().take(6).collect();
        self.parts
            .into_iter()
            .enumerate()
            .map(|(n, p)| {
                if p.name.is_empty() {
                    return Err(crate::adapter::parse_err("도구 호출", "도구 이름이 없음"));
                }
                // 인자가 없는 도구는 빈 글로 온다
                let (arguments, parse_error) = if p.args.trim().is_empty() {
                    (Value::Object(Default::default()), None)
                } else {
                    match serde_json::from_str::<Value>(&p.args) {
                        Ok(v) if v.is_object() => (v, None),
                        Ok(_) => (Value::Object(Default::default()), Some("인자가 객체가 아님".to_string())),
                        Err(e) => (Value::Object(Default::default()), Some(e.to_string())),
                    }
                };
                Ok(ToolCall {
                    id: if p.id.is_empty() { format!("call_{tag}_{}", n + 1) } else { p.id },
                    name: p.name,
                    arguments,
                    raw_arguments: p.args,
                    parse_error,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tool(name: &str) -> ToolSpec {
        ToolSpec {
            name: name.into(),
            description: "d".into(),
            parameters: json!({ "type": "object", "properties": { "q": { "type": "string" } }, "required": ["q"] }),
        }
    }
    pub(crate) fn call(id: &str) -> ToolCall {
        ToolCall {
            id: id.into(),
            name: "search".into(),
            arguments: json!({ "q": "x" }),
            raw_arguments: "{\"q\":\"x\"}".into(),
            parse_error: None,
        }
    }
    fn result(id: &str) -> ToolResult {
        ToolResult {
            call_id: id.into(),
            name: "search".into(),
            content: "ok".into(),
            is_error: false,
        }
    }
    fn asst(ids: &[&str]) -> ToolTurn {
        ToolTurn::Assistant {
            text: String::new(),
            calls: ids.iter().map(|i| call(i)).collect(),
            state: None,
        }
    }
    fn results(ids: &[&str]) -> ToolTurn {
        ToolTurn::Results {
            results: ids.iter().map(|i| result(i)).collect(),
            note: None,
        }
    }

    #[test]
    fn validates_requests_before_any_call_is_made() {
        assert!(ChatExtras::default().validate().is_ok() && ChatExtras::default().is_empty());
        let ok = ChatExtras {
            tools: vec![tool("search")],
            tool_choice: ToolChoice::Named("search".into()),
            tail: vec![asst(&["a", "b"]), results(&["b", "a"])],
            ..Default::default()
        };
        assert!(ok.validate().is_ok());

        let with = |f: &dyn Fn(&mut ChatExtras)| {
            let mut e = ok.clone();
            f(&mut e);
            e.validate().unwrap_err().to_string()
        };
        assert!(with(&|e| e.tools[0].name = "bad name!".into()).contains("도구 이름"));
        assert!(
            with(&|e| e.tools[0].name = "1search".into()).contains("도구 이름"),
            "숫자로 시작하면 Gemini가 거절"
        );
        assert!(with(&|e| e.tools.push(tool("search"))).contains("겹침"));
        assert!(with(&|e| e.tool_choice = ToolChoice::Named("nope".into())).contains("없는 도구"));
        assert!(with(&|e| e.tools[0].parameters = json!({ "type": "object", "properties": { "a": { "$ref": "#/x" } } })).contains("스키마"));
        assert!(with(&|e| e.tail = vec![asst(&["a"]), results(&["zzz"])]).contains("짝이 맞지"));
        assert!(with(&|e| e.tail = vec![asst(&["a"]), asst(&[])]).contains("결과가 없음"));
        assert!(
            with(&|e| e.tail = vec![asst(&["a"]), results(&["a"]), asst(&["a"]), results(&["a"])]).contains("id가 겹침"),
            "루프 전체에서 id가 유일해야 함"
        );
        assert!(with(&|e| e.response_schema = Some(ResponseSchema {
            name: "r".into(),
            description: None,
            schema: json!({ "type": "object" })
        }))
        .contains("같이 쓸 수 없음"));
        assert!(ChatExtras {
            tool_choice: ToolChoice::Required,
            ..Default::default()
        }
        .validate()
        .is_err());
        assert!(ChatExtras {
            tail: vec![results(&[])],
            ..Default::default()
        }
        .validate()
        .is_err());
        let bad_schema = ChatExtras {
            response_schema: Some(ResponseSchema {
                name: "r".into(),
                description: None,
                schema: json!({ "type": "array" }),
            }),
            ..Default::default()
        };
        assert!(bad_schema.validate().unwrap_err().to_string().contains("object"));
    }

    #[test]
    fn assembles_streamed_fragments_in_order_and_keeps_broken_calls() {
        let mut a = CallAssembler::default();
        a.start(1, Some("c2"), Some("lookup"));
        a.start(0, Some("c1"), Some("search"));
        a.start(0, Some("c1"), None); // 되풀이된 id는 그대로
        for f in ["{\"q\":", " \"고양", "이\"}"] {
            a.push_args(0, f);
        }
        let calls = a.finish().unwrap();
        assert_eq!(calls.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["c1", "c2"]);
        assert_eq!(calls[0].arguments, json!({ "q": "고양이" }));
        assert_eq!(calls[0].raw_arguments, "{\"q\": \"고양이\"}");
        assert_eq!(calls[1].arguments, json!({}), "인자가 없으면 빈 객체");

        // id가 없으면 응답마다 다른 표: 같은 번호여도 두 응답의 id가 겹치지 않는다
        let ids: Vec<String> = (0..2)
            .map(|_| {
                let mut a = CallAssembler::default();
                a.start(0, None, Some("x"));
                a.finish().unwrap().remove(0).id
            })
            .collect();
        assert!(ids[0].starts_with("call_") && ids[0].ends_with("_1") && ids[0] != ids[1], "{ids:?}");

        // 끊긴 JSON도 호출은 남긴다 (모델에게 "인자 오류"를 돌려줄 수 있게)
        let mut broken = CallAssembler::default();
        broken.start(0, Some("c"), Some("x"));
        broken.push_args(0, "{\"q\": ");
        let c = broken.finish().unwrap().remove(0);
        assert!(c.parse_error.is_some() && c.arguments == json!({}) && c.raw_arguments == "{\"q\": ");
        let mut not_obj = CallAssembler::default();
        not_obj.start(0, Some("c"), Some("x"));
        not_obj.push_args(0, "[1]");
        assert!(not_obj.finish().unwrap()[0].parse_error.is_some());
        let mut nameless = CallAssembler::default();
        nameless.push_args(0, "{}");
        assert!(nameless.finish().is_err());
    }
}
