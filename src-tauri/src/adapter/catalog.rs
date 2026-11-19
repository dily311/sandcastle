//! 앱에 내장한 텍스트 모델 목록(`resources/info/default_text_models.json`)의 모델별 정보(metadata).
//!
//! 모델이 무엇을 할 수 있는지(도구·구조화 출력·추론 강도)와 단가를 **JSON에 적어 둔 것만** 읽는다.
//! 이름 규칙으로 짐작하는 것은 `capabilities.rs`가 맡고, 여기 없는 모델은 "모른다"로 돌려준다 (막지 않음).
//! 사람이 고치는 파일이라 모델이 새로 나오면 JSON만 고치면 된다.
use std::{collections::HashMap, sync::OnceLock};

use serde::Deserialize;

/// 추론 강도를 조절할 수 있는 모델의 정보
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Reasoning {
    /// 고를 수 있는 값 (낮은 것부터)
    pub levels: Vec<String>,
    /// 정하지 않았을 때 공급자 기본값
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct LongPricing {
    pub above: u64,
    pub input: f64,
    pub output: f64,
}

/// 100만 토큰당 달러
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Pricing {
    pub input: f64,
    pub output: f64,
    #[serde(default)]
    pub long: Option<LongPricing>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelMeta {
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub input_modalities: Vec<String>,
    /// tools · structuredOutput · reasoning. 비어 있으면 적지 않은 것
    pub features: Vec<String>,
    pub reasoning: Option<Reasoning>,
    pub pricing: Option<Pricing>,
}

impl ModelMeta {
    pub fn has(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }
}

#[derive(Deserialize)]
struct Model {
    #[serde(rename = "modelId")]
    model_id: String,
    #[serde(default)]
    metadata: Option<ModelMeta>,
}
#[derive(Deserialize)]
struct Group {
    model: Vec<Model>,
}
#[derive(Deserialize)]
struct Provider {
    #[serde(rename = "modelList")]
    model_list: Vec<Group>,
}
#[derive(Deserialize)]
struct File {
    data: Vec<Provider>,
}

/// "models/gemini-3-flash", "anthropic/claude-opus-5-5" 같은 이름을 목록의 id 꼴로 (소문자, 공급자 접두사 제거)
fn key(model: &str) -> String {
    let last = model.rsplit('/').next().unwrap_or(model);
    last.trim().to_ascii_lowercase()
}

fn table() -> &'static HashMap<String, ModelMeta> {
    static TABLE: OnceLock<HashMap<String, ModelMeta>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let file = serde_json::from_str::<File>(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/info/default_text_models.json")))
            .map(|f| f.data)
            .unwrap_or_default();
        let mut out = HashMap::new();
        for p in file {
            for g in p.model_list {
                for m in g.model {
                    // 같은 모델이 공급자마다 나오면(Google·Vertex) 정보가 있는 쪽을 쓴다
                    if let Some(meta) = m.metadata {
                        out.entry(key(&m.model_id)).or_insert(meta);
                    }
                }
            }
        }
        out
    })
}

/// 모델 id(정확히 같은 것만)로 찾는다. 목록에 없거나 정보를 적지 않았으면 None
pub fn meta(model: &str) -> Option<&'static ModelMeta> {
    table().get(&key(model))
}

/// 이 모델에서 보낼 수 있는 추론 강도인가 (목록의 levels에 있어야 함)
pub fn accepts_effort(model: &str, level: &str) -> bool {
    meta(model)
        .and_then(|m| m.reasoning.as_ref())
        .is_some_and(|r| r.levels.iter().any(|l| l == level))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_metadata_by_exact_id_ignoring_case_and_provider_prefix() {
        let m = meta("Anthropic/claude-opus-5-5").expect("claude-opus-5-5");
        assert!(m.has("reasoning") && m.has("tools"));
        assert_eq!(m.reasoning.as_ref().unwrap().default.as_deref(), Some("medium"));
        assert_eq!(m.pricing.as_ref().map(|p| (p.input, p.output)), Some((4.0, 20.0)));
        assert!(meta("claude-opus").is_none(), "앞부분만 같은 이름은 다른 모델");
        assert!(meta("my-finetune").is_none());
    }

    #[test]
    fn effort_is_accepted_only_for_listed_levels() {
        assert!(accepts_effort("claude-opus-5-5", "high"));
        assert!(accepts_effort("claude-opus-5-5", "max"));
        assert!(!accepts_effort("claude-opus-5-5", "minimal"));
        assert!(!accepts_effort("my-finetune", "high"), "정보가 없으면 보내지 않음");
        assert!(accepts_effort("gemini-4-argon", "low"));
        assert!(!accepts_effort("gemini-4-argon", "xhigh"), "그 모델의 levels에 없는 값");
        assert!(accepts_effort("deepseek-v4-pro", "max") && !accepts_effort("deepseek-v4-pro", "medium"));
    }

    #[test]
    fn long_pricing_is_read() {
        let p = meta("claude-haiku-5-5").and_then(|m| m.pricing.clone()).unwrap();
        assert_eq!(
            p.long,
            Some(LongPricing {
                above: 100_000,
                input: 0.5,
                output: 2.5
            })
        );
    }

    #[test]
    fn every_listed_reasoning_has_levels_and_a_listed_default() {
        for (id, m) in table() {
            if let Some(r) = &m.reasoning {
                assert!(!r.levels.is_empty(), "{id}");
                if let Some(d) = &r.default {
                    assert!(r.levels.contains(d), "{id}: 기본값 {d}이 levels에 없음");
                }
            }
        }
    }
}
