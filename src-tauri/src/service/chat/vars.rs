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
}
