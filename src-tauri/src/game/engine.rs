//! 순수 엔진: DB, Tauri, async를 전혀 모른다. 이미 조회된 후보를 받아 결정만 내린다.
use rand::{Rng, RngExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineState {
    pub map_id: String,
    pub zone_id: Option<String>,
    pub location_id: Option<String>,
    pub game_time: i64, // 누적 초
    /// 이번에 시간이 흐르기 전의 시각. 시간이 안 흐른 순간에는 game_time과 같다.
    /// AtMinuteOfDay(예약 이벤트)가 "이번에 건너뛴 구간에 그 시각이 포함됐는가"를 판정할 때 쓴다.
    pub time_from: i64,
    pub weather_id: String,
    pub time_period_id: String,
    pub flags: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,       // game_encounter_entries.id
    pub event_id: String, // game_events.id
    pub weight: u32,
    pub time_period_id: Option<String>, // None = 무관
    pub weather_id: Option<String>,
    pub conditions: Vec<Condition>,
}

/// 선언형 조건. game_event_conditions.condition_type/condition_value,
/// game_location_connections.condition 를 이 enum으로 매핑해서 쓴다 (JSON으로 통째로 직렬화).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    FlagGte { key: String, value: i64 },
    FlagEq { key: String, value: serde_json::Value },
    MinTime { seconds: i64 },
    InLocation { location_id: String },
    InZone { zone_id: String },
    /// 하루 중 minute(0~1439)분이 이번 시간 경과 구간 (time_from, game_time]에 포함됐는가.
    /// 정확히 그 시각에 멈춰 있어야 하는 게 아니라, 건너뛰어도(17시→01시) 발동한다.
    AtMinuteOfDay { minute: u32 },
    /// 절대 참이 아니다. 저장된 조건 JSON을 못 읽을 때 "조건 없음(=항상 통과)"으로 취급하면
    /// 의도치 않게 열리므로, 읽지 못한 조건은 이 값으로 대신한다 (fail-closed).
    Never,
    // 새 조건은 variant만 추가
}

/// (from, to] 구간 안에 하루 중 minute분이 한 번이라도 있었는가. 며칠을 건너뛰어도 true 하나.
pub fn crossed_minute_of_day(from_seconds: i64, to_seconds: i64, minute: u32) -> bool {
    const DAY: i64 = 86_400;
    if to_seconds <= from_seconds {
        return false;
    }
    if to_seconds - from_seconds >= DAY {
        return true;
    }
    let target = minute as i64 * 60;
    let mut next = from_seconds - from_seconds.rem_euclid(DAY) + target;
    if next <= from_seconds {
        next += DAY;
    }
    next <= to_seconds
}

impl Condition {
    pub fn eval(&self, s: &EngineState) -> bool {
        match self {
            Condition::FlagGte { key, value } => s.flags.get(key).and_then(|v| v.as_f64()).map_or(false, |v| v >= *value as f64),
            Condition::FlagEq { key, value } => s.flags.get(key) == Some(value),
            Condition::MinTime { seconds } => s.game_time >= *seconds,
            Condition::InLocation { location_id } => s.location_id.as_deref() == Some(location_id.as_str()),
            Condition::InZone { zone_id } => s.zone_id.as_deref() == Some(zone_id.as_str()),
            Condition::AtMinuteOfDay { minute } => crossed_minute_of_day(s.time_from, s.game_time, *minute),
            Condition::Never => false,
        }
    }
}

/// game_time_periods.start_minute/end_minute 기준 범위 조회.
/// end < start면 자정을 넘는 구간으로 취급한다 (예: 22:00~02:00 → start=1320, end=120).
/// 겹치는 구간이 여러 개면 periods 목록에서 먼저 나오는 쪽을 쓴다 — 데이터 설계 책임.
pub fn resolve_time_period(periods: &[(String, u32, u32)], game_time_seconds: i64) -> Option<String> {
    const DAY_MINUTES: i64 = 1440;
    if periods.is_empty() {
        return None;
    }
    let minute_of_day = (game_time_seconds / 60).rem_euclid(DAY_MINUTES) as u32;
    periods
        .iter()
        .find(|(_, start, end)| {
            if start <= end {
                minute_of_day >= *start && minute_of_day < *end
            } else {
                minute_of_day >= *start || minute_of_day < *end
            }
        })
        .map(|(id, _, _)| id.clone())
}

pub fn weighted_index<R: Rng>(weights: &[u32], rng: &mut R) -> Option<usize> {
    let total: u32 = weights.iter().sum();
    if total == 0 {
        return None;
    }
    let mut roll = rng.random_range(0..total);
    for (i, w) in weights.iter().enumerate() {
        if roll < *w {
            return Some(i);
        }
        roll -= *w;
    }
    None
}

/// probabilities가 1.0에 못 미치면 남는 확률만큼 "날씨 유지"가 된다.
pub fn roll_next_weather<R: Rng>(transitions: &[(String, f64)], current_weather_id: &str, rng: &mut R) -> String {
    let total: f64 = transitions.iter().map(|(_, p)| p).sum();
    let roll = rng.random::<f64>();
    if roll >= total.min(1.0) {
        return current_weather_id.to_string();
    }
    let mut acc = 0.0;
    for (to, p) in transitions {
        acc += p;
        if roll < acc {
            return to.clone();
        }
    }
    current_weather_id.to_string()
}

/// 반환값: entries 안의 인덱스. cooling_down은 game_events.id 목록 (쿨다운 중인 것들).
pub fn roll_encounter<R: Rng>(
    entries: &[Entry],
    state: &EngineState,
    base_chance: f64,
    weather_mod: f64,
    cooling_down: &[String],
    rng: &mut R,
) -> Option<usize> {
    if rng.random::<f64>() >= (base_chance * weather_mod).clamp(0.0, 1.0) {
        return None;
    }
    let idx: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.time_period_id.as_deref().map_or(true, |p| p == state.time_period_id))
        .filter(|(_, e)| e.weather_id.as_deref().map_or(true, |w| w == state.weather_id))
        .filter(|(_, e)| !cooling_down.contains(&e.event_id))
        .filter(|(_, e)| e.conditions.iter().all(|c| c.eval(state)))
        .map(|(i, _)| i)
        .collect();
    let weights: Vec<u32> = idx.iter().map(|&i| entries[i].weight).collect();
    weighted_index(&weights, rng).map(|k| idx[k])
}

/// effect JSON 적용. 지원하는 형태: {"set": {"key": value}, "add": {"key": 숫자}}.
/// flags/effect 둘 다 읽기만 하고, 결과를 "바뀐 뒤의 flags"와 "(key, old, new) 변경 목록"으로
/// 돌려준다 — 실제로 state.flags에 쓰는 건 호출부(트리거 시스템) 책임이다. 이 함수 자체는
/// state를 모른다 (engine.rs 원칙: DB/async/GameState 전부 모름).
pub fn apply_effect(flags: &serde_json::Value, effect: &serde_json::Value) -> (serde_json::Value, Vec<(String, serde_json::Value, serde_json::Value)>) {
    let mut result = flags.clone();
    let mut changes = Vec::new();
    if !result.is_object() {
        result = serde_json::json!({});
    }
    let obj = result.as_object_mut().expect("위에서 object로 만들었음");

    if let Some(serde_json::Value::Object(set_map)) = effect.get("set") {
        for (k, v) in set_map {
            let old = obj.get(k).cloned().unwrap_or(serde_json::Value::Null);
            if &old != v {
                obj.insert(k.clone(), v.clone());
                changes.push((k.clone(), old, v.clone()));
            }
        }
    }
    if let Some(serde_json::Value::Object(add_map)) = effect.get("add") {
        for (k, delta) in add_map {
            let old = obj.get(k).cloned().unwrap_or(serde_json::Value::from(0));
            // 둘 다 정수면 정수로 유지한다 (8.0이 되면 as_i64 기반 비교/표시가 깨진다).
            let new_val = match (old.as_i64(), delta.as_i64()) {
                (Some(a), Some(b)) => serde_json::Value::from(a + b),
                _ => serde_json::Value::from(old.as_f64().unwrap_or(0.0) + delta.as_f64().unwrap_or(0.0)),
            };
            obj.insert(k.clone(), new_val.clone());
            changes.push((k.clone(), old, new_val));
        }
    }
    (result, changes)
}

/// game_state_defs.config_json 에서 읽는 "AI가 이 변수를 얼마나 만질 수 있는가".
/// 예: {"ai_readable": true, "ai_writable": true, "min": 0, "max": 100, "max_step": 3}
/// ai_writable이 아니면 AI는 이 변수를 못 바꾼다 (허용 목록).
#[derive(Debug, Clone, Default)]
pub struct VarRule {
    pub readable: bool,
    pub writable: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub max_step: Option<f64>,
}

impl VarRule {
    pub fn from_config(config_json: &str) -> Self {
        let v: serde_json::Value = serde_json::from_str(config_json).unwrap_or(serde_json::Value::Null);
        let writable = v.get("ai_writable").and_then(|x| x.as_bool()).unwrap_or(false);
        Self {
            // 쓸 수 있는 변수는 AI가 현재 값을 알아야 하니 읽기도 허용으로 본다.
            readable: writable || v.get("ai_readable").and_then(|x| x.as_bool()).unwrap_or(false),
            writable,
            min: v.get("min").and_then(|x| x.as_f64()),
            max: v.get("max").and_then(|x| x.as_f64()),
            max_step: v.get("max_step").and_then(|x| x.as_f64()).filter(|s| *s > 0.0),
        }
    }
}

fn num_value(n: f64) -> serde_json::Value {
    if n.fract() == 0.0 && n.abs() < 9.0e15 { serde_json::Value::from(n as i64) } else { serde_json::Value::from(n) }
}

/// AI가 낸 effect를 허용 목록/범위로 걸러서 "(key, old, new) 변경 목록"으로 돌려준다.
/// set/add만 읽고 enter_scene/fire_event 등 다른 동사는 무시한다 — AI는 변수만 만질 수 있다.
///  - 허용 목록(ai_writable)에 없는 키는 버림, 한 번에 max_step 넘게는 못 바꿈, min/max로 자름
///  - set은 숫자/불리언/문자열만 (배열/객체/null 불허), 이미 값이 있으면 같은 타입일 때만
///  - 변경이 없는 항목은 목록에 안 넣음, 최대 max_changes개까지
pub fn plan_ai_changes(
    flags: &serde_json::Value,
    effect: &serde_json::Value,
    rules: &std::collections::HashMap<String, VarRule>,
    max_changes: usize,
) -> Vec<(String, serde_json::Value, serde_json::Value)> {
    let mut out: Vec<(String, serde_json::Value, serde_json::Value)> = Vec::new();
    let clamp = |rule: &VarRule, old: f64, mut new: f64| -> f64 {
        if let Some(step) = rule.max_step {
            new = new.clamp(old - step, old + step);
        }
        if let Some(min) = rule.min {
            new = new.max(min);
        }
        if let Some(max) = rule.max {
            new = new.min(max);
        }
        new
    };

    if let Some(serde_json::Value::Object(set_map)) = effect.get("set") {
        for (k, v) in set_map {
            if out.len() >= max_changes {
                break;
            }
            let Some(rule) = rules.get(k).filter(|r| r.writable) else { continue };
            let old = flags.get(k).cloned().unwrap_or(serde_json::Value::Null);
            let new = match v {
                serde_json::Value::Number(n) => {
                    let Some(target) = n.as_f64().filter(|f| f.is_finite()) else { continue };
                    if !(old.is_null() || old.is_number()) {
                        continue;
                    }
                    num_value(clamp(rule, old.as_f64().unwrap_or(0.0), target))
                }
                serde_json::Value::Bool(_) if old.is_null() || old.is_boolean() => v.clone(),
                serde_json::Value::String(_) if old.is_null() || old.is_string() => v.clone(),
                _ => continue,
            };
            if new != old {
                out.push((k.clone(), old, new));
            }
        }
    }
    if let Some(serde_json::Value::Object(add_map)) = effect.get("add") {
        for (k, d) in add_map {
            if out.len() >= max_changes {
                break;
            }
            let Some(rule) = rules.get(k).filter(|r| r.writable) else { continue };
            let Some(delta) = d.as_f64().filter(|f| f.is_finite()) else { continue };
            let old = flags.get(k).cloned().unwrap_or(serde_json::Value::from(0));
            if !old.is_number() {
                continue;
            }
            let old_n = old.as_f64().unwrap_or(0.0);
            let new = num_value(clamp(rule, old_n, old_n + delta));
            if new != old {
                out.push((k.clone(), old, new));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    fn entry(event_id: &str, w: u32, weather: Option<&str>) -> Entry {
        Entry { id: format!("entry-{event_id}"), event_id: event_id.into(), weight: w, time_period_id: None, weather_id: weather.map(String::from), conditions: vec![] }
    }
    fn state(weather: &str) -> EngineState {
        EngineState { map_id: "map1".into(), zone_id: None, location_id: None, game_time: 0, time_from: 0, weather_id: weather.into(), time_period_id: "day".into(), flags: serde_json::json!({}) }
    }

    #[test]
    fn seeded_rng_is_reproducible() {
        let es = vec![entry("e1", 10, None), entry("e2", 90, Some("rain"))];
        let a = roll_encounter(&es, &state("rain"), 1.0, 1.0, &[], &mut StdRng::seed_from_u64(7));
        let b = roll_encounter(&es, &state("rain"), 1.0, 1.0, &[], &mut StdRng::seed_from_u64(7));
        assert_eq!(a, b);
    }

    #[test]
    fn weather_filter_excludes_entries() {
        let es = vec![entry("e1", 10, Some("snow"))];
        assert_eq!(roll_encounter(&es, &state("rain"), 1.0, 1.0, &[], &mut StdRng::seed_from_u64(1)), None);
    }

    #[test]
    fn cooldown_excludes_entries() {
        let es = vec![entry("e1", 10, None)];
        let cooling = vec!["e1".to_string()];
        assert_eq!(roll_encounter(&es, &state("clear"), 1.0, 1.0, &cooling, &mut StdRng::seed_from_u64(1)), None);
    }

    #[test]
    fn time_period_range_lookup() {
        let periods = vec![
            ("dawn".to_string(), 300u32, 420u32),   // 05:00~07:00
            ("day".to_string(), 420, 1080),         // 07:00~18:00
            ("night".to_string(), 1080, 300),       // 18:00~05:00 (자정 넘김)
        ];
        assert_eq!(resolve_time_period(&periods, 300 * 60), Some("dawn".into()));
        assert_eq!(resolve_time_period(&periods, 600 * 60), Some("day".into()));
        assert_eq!(resolve_time_period(&periods, 23 * 3600), Some("night".into())); // 23:00
        assert_eq!(resolve_time_period(&periods, 1 * 3600), Some("night".into()));  // 01:00 (자정 넘김)
    }

    #[test]
    fn crossing_a_minute_of_day() {
        let h = 3600;
        assert!(crossed_minute_of_day(17 * h, 25 * h, 18 * 60)); // 17시 → 다음날 01시, 18시를 건너뜀
        assert!(!crossed_minute_of_day(19 * h, 23 * h, 18 * 60));
        assert!(!crossed_minute_of_day(18 * h, 20 * h, 18 * 60)); // 이미 18시였던 순간은 포함 안 됨 (from 제외)
        assert!(crossed_minute_of_day(0, 3 * 86_400, 5)); // 며칠 건너뛰어도 true
        assert!(!crossed_minute_of_day(10, 10, 0));
    }

    #[test]
    fn ai_changes_are_whitelisted_and_clamped() {
        let mut rules = std::collections::HashMap::new();
        rules.insert("affection".to_string(), VarRule::from_config(r#"{"ai_writable":true,"min":0,"max":10,"max_step":3}"#));
        rules.insert("secret".to_string(), VarRule::from_config(r#"{"ai_readable":true}"#)); // 읽기만
        let flags = serde_json::json!({ "affection": 9, "secret": 1, "other": 5 });
        let effect = serde_json::json!({
            "add": { "affection": 100, "secret": 5, "other": 5 },
            "set": { "affection": 0, "enter_scene": "x" },
            "enter_scene": "boss"
        });
        let changes = plan_ai_changes(&flags, &effect, &rules, 5);
        // set은 max_step(3) 때문에 9→6, add는 그 뒤가 아니라 원래 flags 기준으로 9+3→10(max)
        assert!(changes.iter().all(|(k, _, _)| k == "affection"));
        assert!(changes.iter().all(|(_, _, new)| new.as_i64().map_or(false, |n| (0..=10).contains(&n))));
    }

    #[test]
    fn apply_effect_set_and_add() {
        let flags = serde_json::json!({ "affection": 5, "met_yuna": false });
        let effect = serde_json::json!({ "add": { "affection": 3 }, "set": { "met_yuna": true } });
        let (new_flags, changes) = apply_effect(&flags, &effect);
        assert_eq!(new_flags["affection"], 8);
        assert_eq!(new_flags["met_yuna"], true);
        assert_eq!(changes.len(), 2);
    }
}