//! 정규식 치환 스크립트 모델 (SillyTavern regex 확장 호환)
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use strum_macros::{Display, EnumString};

/// 어떤 텍스트에 적용할지
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumString, Display, specta::Type)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RegexPlacement {
    /// 사용자가 보낸 메시지
    UserInput,
    /// AI 응답
    AiOutput,
    /// 프롬프트에 들어가는 월드인포 항목
    WorldInfo,
}

impl RegexPlacement {
    /// 메시지 role → placement
    pub fn for_role(role: &str) -> Option<Self> {
        match role {
            "user" => Some(Self::UserInput),
            "assistant" => Some(Self::AiOutput),
            _ => None,
        }
    }
}

/// 어디에 반영할지
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display, specta::Type)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RegexTarget {
    /// 저장되는 메시지 자체를 바꿈
    Stored,
    /// AI에게 보내는 프롬프트에서만 바꿈
    Prompt,
    /// 화면 표시에서만 바꿈
    Display,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display, specta::Type)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum RegexScope {
    Global,
    Character,
    Preset,
}

/// find_regex 안의 매크로 치환 방식
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, EnumString, Display, specta::Type)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MacroMode {
    #[default]
    None,
    /// 값을 그대로 넣음 (값 안의 정규식 특수문자가 패턴으로 해석됨)
    Raw,
    /// 정규식 특수문자를 이스케이프해서 넣음
    Escaped,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RegexScript {
    pub id: String,
    pub name: String,
    pub find_regex: String,
    pub replace_string: String,
    pub trim_strings: Vec<String>,
    pub placement: Vec<RegexPlacement>,
    pub target: RegexTarget,
    pub scope: RegexScope,
    pub scope_id: Option<String>,
    pub substitute_macros: MacroMode,
    pub run_on_edit: bool,
    pub min_depth: Option<i64>,
    pub max_depth: Option<i64>,
    pub enabled: bool,
    pub sort_order: i64,
    pub source_format: String,
    pub extra_json: Option<Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(FromRow)]
pub struct RegexScriptRow {
    pub id: String,
    pub name: String,
    pub find_regex: String,
    pub replace_string: String,
    pub trim_strings: String,
    pub placement: String,
    pub target: String,
    pub scope: String,
    pub scope_id: Option<String>,
    pub substitute_macros: String,
    pub run_on_edit: bool,
    pub min_depth: Option<i64>,
    pub max_depth: Option<i64>,
    pub enabled: bool,
    pub sort_order: i64,
    pub source_format: String,
    pub extra_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<RegexScriptRow> for RegexScript {
    fn from(r: RegexScriptRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            find_regex: r.find_regex,
            replace_string: r.replace_string,
            trim_strings: serde_json::from_str(&r.trim_strings).unwrap_or_default(),
            // 알 수 없는 값은 건너뜀 (이후 버전에서 늘어난 placement 등)
            placement: serde_json::from_str::<Vec<Value>>(&r.placement)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect(),
            target: r.target.parse().unwrap_or(RegexTarget::Stored),
            scope: r.scope.parse().unwrap_or(RegexScope::Global),
            scope_id: r.scope_id,
            substitute_macros: r.substitute_macros.parse().unwrap_or_default(),
            run_on_edit: r.run_on_edit,
            min_depth: r.min_depth,
            max_depth: r.max_depth,
            enabled: r.enabled,
            sort_order: r.sort_order,
            source_format: r.source_format,
            extra_json: r.extra_json.and_then(|s| serde_json::from_str(&s).ok()),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// 생성/수정 입력. id가 없으면 새로 만든다.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RegexScriptInput {
    pub id: Option<String>,
    pub name: String,
    pub find_regex: String,
    #[serde(default)]
    pub replace_string: String,
    #[serde(default)]
    pub trim_strings: Vec<String>,
    pub placement: Vec<RegexPlacement>,
    #[serde(default = "default_target")]
    pub target: RegexTarget,
    #[serde(default = "default_scope")]
    pub scope: RegexScope,
    pub scope_id: Option<String>,
    #[serde(default)]
    pub substitute_macros: MacroMode,
    #[serde(default = "default_true")]
    pub run_on_edit: bool,
    pub min_depth: Option<i64>,
    pub max_depth: Option<i64>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub sort_order: i64,
    pub source_format: Option<String>,
    pub extra_json: Option<Value>,
}

fn default_target() -> RegexTarget {
    RegexTarget::Stored
}
fn default_scope() -> RegexScope {
    RegexScope::Global
}
fn default_true() -> bool {
    true
}

impl RegexScriptInput {
    /// 저장 전 미리보기/검증용으로 RegexScript 형태로 변환
    pub fn to_script(&self, id: String, now: i64) -> RegexScript {
        RegexScript {
            id,
            name: self.name.clone(),
            find_regex: self.find_regex.clone(),
            replace_string: self.replace_string.clone(),
            trim_strings: self.trim_strings.clone(),
            placement: self.placement.clone(),
            target: self.target,
            scope: self.scope,
            scope_id: self.scope_id.clone(),
            substitute_macros: self.substitute_macros,
            run_on_edit: self.run_on_edit,
            min_depth: self.min_depth,
            max_depth: self.max_depth,
            enabled: self.enabled,
            sort_order: self.sort_order,
            source_format: self.source_format.clone().unwrap_or_else(|| "native".into()),
            extra_json: self.extra_json.clone(),
            created_at: now,
            updated_at: now,
        }
    }
}
