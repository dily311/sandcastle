//! 모델 단가 → 호출 금액(달러) 계산 (ADR-007 추가).
//!
//! 단가는 두 곳에서 온다. 사용자가 단가표(model_prices)에 적은 값(달러 `$`만)이 먼저이고,
//! 없으면 앱에 넣어 둔 기본 단가(default_prices.json), 그것도 없으면 모델 목록(default_text_models.json)의 metadata.pricing을 쓴다.
//! 모두 없으면 계산하지 못한다(= 단가 없음).
//! 금액은 어디까지나 추정이다: 캐시된 입력은 더 싸지만 구분해 기록하지 않아 실제보다 조금 많게 나온다.
use std::sync::OnceLock;

use serde::Deserialize;

/// 100만 토큰당 달러
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price {
    pub input: f64,
    pub output: f64,
    /// 한 요청의 입력이 이 토큰 수를 넘으면 (입력, 출력) 단가가 올라가는 모델: (기준 토큰, 입력, 출력)
    pub long: Option<(u64, f64, f64)>,
}

impl Price {
    pub fn cost(self, input_tokens: u64, output_tokens: u64) -> f64 {
        let (input, output) = match self.long {
            Some((above, input, output)) if input_tokens > above => (input, output),
            _ => (self.input, self.output),
        };
        (input_tokens as f64 * input + output_tokens as f64 * output) / 1_000_000.0
    }
}

/// 긴 입력 단가
#[derive(Deserialize)]
struct LongTier {
    above: u64,
    input: f64,
    output: f64,
}

#[derive(Deserialize)]
struct DefaultEntry {
    id: String,
    input: f64,
    output: f64,
    /// 이 날부터 적용(YYYY-MM-DD). 가격 인상이 예고된 모델은 같은 id를 날짜별로 여러 줄 둔다. 없으면 처음부터
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    long: Option<LongTier>,
}

#[derive(Deserialize)]
struct DefaultFile {
    models: Vec<DefaultEntry>,
}

fn defaults() -> &'static [DefaultEntry] {
    static TABLE: OnceLock<Vec<DefaultEntry>> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str::<DefaultFile>(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/info/default_prices.json")))
            .map(|f| f.models)
            .unwrap_or_default()
    })
}

/// "anthropic/claude-opus-4.5", "models/gemini-2.5-pro" 같은 이름을 표의 이름 꼴로 (소문자, 공급자 접두사 제거, 점·공백은 하이픈)
fn normalize(model: &str) -> String {
    let last = model.rsplit('/').next().unwrap_or(model);
    last.trim().to_ascii_lowercase().replace(['.', ' '], "-")
}

/// id 뒤에 올 수 있는 꼬리: 날짜(-20250929, -2024-08-06), -latest, @버전, :태그. 그 밖(-mini, -5 …)은 다른 모델이다
fn is_variant_suffix(rest: &str) -> bool {
    if rest.is_empty() || rest.starts_with('@') || rest.starts_with(':') || rest == "-latest" {
        return true;
    }
    let Some(d) = rest.strip_prefix('-') else { return false };
    let b = d.as_bytes();
    let digits = |r: std::ops::Range<usize>| b.get(r).is_some_and(|s| s.iter().all(u8::is_ascii_digit));
    (b.len() == 8 && digits(0..8)) || (b.len() == 10 && digits(0..4) && b[4] == b'-' && digits(5..7) && b[7] == b'-' && digits(8..10))
}

/// 앱에 넣어 둔 기본 단가(오늘 기준). 이름이 id와 같거나 날짜·latest 꼬리만 붙은 것만 맞춘다 (가장 긴 id가 이김)
pub fn default_price(model: &str) -> Option<Price> {
    default_price_on(model, &chrono::Local::now().format("%Y-%m-%d").to_string())
}

/// `today`(YYYY-MM-DD)에 적용되는 기본 단가. 같은 id의 줄이 여럿이면 `from`이 today 이하인 것 중 가장 늦은 것
pub fn default_price_on(model: &str, today: &str) -> Option<Price> {
    let name = normalize(model);
    defaults()
        .iter()
        .filter(|e| name.strip_prefix(normalize(&e.id).as_str()).is_some_and(is_variant_suffix))
        .filter(|e| e.from.as_deref().is_none_or(|f| f <= today))
        .max_by(|a, b| (a.id.len(), a.from.as_deref()).cmp(&(b.id.len(), b.from.as_deref())))
        .map(|e| Price {
            input: e.input,
            output: e.output,
            long: e.long.as_ref().map(|l| (l.above, l.input, l.output)),
        })
        // 기본 단가표(default_prices.json)에 없으면 모델 목록(default_text_models.json)의 metadata.pricing
        .or_else(|| catalog_price(model))
}

/// 모델 목록에 적어 둔 단가 (id가 정확히 같은 모델만)
fn catalog_price(model: &str) -> Option<Price> {
    let p = crate::adapter::catalog::meta(model)?.pricing.as_ref()?;
    Some(Price {
        input: p.input,
        output: p.output,
        long: p.long.as_ref().map(|l| (l.above, l.input, l.output)),
    })
}

/// 이 모델의 단가: 사용자 단가표(달러) → 기본 단가. 둘 다 없으면 None. 두 번째 값은 앱의 기본 단가를 썼는지
pub async fn lookup(db: &sqlx::SqlitePool, model: &str) -> Option<(Price, bool)> {
    if let Ok(rows) = crate::db::usage::get_all_model_prices(db).await {
        // 이름은 대소문자·"공급자/" 접두사를 무시하고 맞춘다. 입력·출력 칸이 둘 다 비면 단가를 적은 것이 아니므로(그림·음성 단가만 적은 줄 등) 건너뜀
        let want = normalize(model);
        let hit = rows
            .into_iter()
            .find(|p| normalize(&p.model) == want && p.currency.trim() == "$" && (p.input_per_mtok.is_some() || p.output_per_mtok.is_some()));
        if let Some(p) = hit {
            // 비운 칸은 0 (단가표의 규칙)
            return Some((
                Price {
                    input: p.input_per_mtok.unwrap_or(0.0),
                    output: p.output_per_mtok.unwrap_or(0.0),
                    long: None,
                },
                false,
            ));
        }
    }
    default_price(model).map(|p| (p, true))
}

pub async fn price_for(db: &sqlx::SqlitePool, model: &str) -> Option<Price> {
    lookup(db, model).await.map(|(p, _)| p)
}

/// 화면에 보여줄 단가 (슬라이더 아래 환산 문구)
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ModelPriceInfo {
    /// 100만 토큰당 달러
    pub input: f64,
    pub output: f64,
    /// 앱의 기본 단가를 썼으면 true (사용자가 적은 값이면 false)
    pub builtin: bool,
}

pub async fn info(db: &sqlx::SqlitePool, model: &str) -> Option<ModelPriceInfo> {
    lookup(db, model).await.map(|(p, builtin)| ModelPriceInfo {
        input: p.input,
        output: p.output,
        builtin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "2026-11-19";

    #[test]
    fn matches_exact_and_dated_but_not_other_variants() {
        assert_eq!(default_price("claude-opus-5-5").map(|p| p.output), Some(20.0));
        assert_eq!(default_price("claude-opus-5-5-20260922").map(|p| p.output), Some(20.0));
        assert_eq!(default_price("anthropic/claude-sonnet-4.6").map(|p| p.input), Some(3.0));
        // 더 긴 id가 이긴다: opus-5-5는 opus-5 값이 아니다
        assert_eq!(default_price("claude-opus-5").map(|p| p.input), Some(5.0));
        // 모르는 버전·변형은 추정하지 않는다
        assert!(default_price("claude-opus-5-6").is_none());
        assert!(default_price("gpt-4o-audio-preview").is_none());
        assert_eq!(default_price("gpt-4o-mini").map(|p| p.input), Some(0.15));
        assert_eq!(default_price("gpt-4o").map(|p| p.input), Some(2.5));
        assert_eq!(default_price("models/gemini-2.5-flash-lite").map(|p| p.output), Some(0.4));
        assert!(default_price("").is_none());
        // OpenAI식 날짜 꼬리(-2024-08-06)도 같은 모델
        assert_eq!(default_price("gpt-4o-2024-08-06").map(|p| p.input), Some(2.5));
        assert_eq!(default_price("gpt-4o-mini-2024-07-18").map(|p| p.input), Some(0.15));
        assert!(default_price("gpt-4o-2024-08").is_none());
    }

    #[test]
    fn cost_is_tokens_times_price_per_million() {
        let p = Price {
            input: 4.0,
            output: 20.0,
            long: None,
        };
        assert!((p.cost(500_000, 100_000) - 4.0).abs() < 1e-9);
        assert_eq!(p.cost(0, 0), 0.0);
    }

    #[test]
    fn long_inputs_use_the_higher_tier() {
        let haiku = default_price_on("claude-haiku-5-5", TODAY).unwrap();
        // 100K 이하는 $0.10/$0.50, 넘으면 $0.50/$2.50
        assert!((haiku.cost(100_000, 1_000_000) - (0.01 + 0.5)).abs() < 1e-9);
        assert!((haiku.cost(100_001, 1_000_000) - (0.050_000_5 + 2.5)).abs() < 1e-9);
    }

    #[test]
    fn announced_price_changes_apply_from_their_date() {
        assert_eq!(default_price_on("gemini-4-argon", "2026-11-30").map(|p| p.input), Some(2.0));
        assert_eq!(default_price_on("gemini-4-argon", "2026-12-01").map(|p| (p.input, p.output)), Some((4.0, 20.0)));
        assert_eq!(default_price_on("gemini-3.8-flash", "2026-12-31").map(|p| p.input), Some(0.75));
        assert_eq!(default_price_on("gemini-3.8-flash", "2027-01-01").map(|p| p.input), Some(1.5));
    }

    #[test]
    fn dots_and_dashes_are_the_same_in_model_names() {
        assert_eq!(default_price_on("GPT-6.2-Astra", TODAY).map(|p| p.input), Some(8.0));
        assert_eq!(default_price_on("gpt-6-2-sol", TODAY).map(|p| p.input), Some(2.0));
        assert_eq!(default_price_on("deepseek/deepseek-v4.1-flash", TODAY).map(|p| p.output), Some(1.2));
        // 버전 번호가 더 붙은 것은 다른 모델
        assert!(default_price_on("gpt-6-2-astra-2", TODAY).is_none());
    }

    #[test]
    fn falls_back_to_the_model_list_pricing_when_the_default_table_has_none() {
        // default_prices.json에 없는 id: 모델 목록(default_text_models.json)의 단가
        assert!(defaults().iter().all(|e| e.id != "claude-fable-5-2" && e.id != "deepseek-flash"));
        assert_eq!(default_price_on("claude-fable-5-2", TODAY).map(|p| (p.input, p.output)), Some((8.0, 40.0)));
        assert_eq!(default_price_on("deepseek-flash", TODAY).map(|p| (p.input, p.output)), Some((0.3, 1.2)));
        // 기본 단가표에 있으면 그쪽이 먼저 (날짜별 인상도 표의 값)
        assert_eq!(default_price_on("gemini-4-argon", "2026-12-01").map(|p| p.input), Some(4.0));
        // 목록에 단가를 적지 않은 모델은 계속 단가 없음
        assert!(default_price_on("gemini-4-neon", TODAY).is_none());
    }

    #[test]
    fn default_table_parses() {
        assert!(defaults().len() > 20);
        assert!(defaults().iter().all(|e| e.input >= 0.0 && e.output >= 0.0));
    }
}
