//! 게임 정의의 일관성 검사 (순수 함수, DB 없음). 제작자가 만든(또는 임포트한) 데이터가
//! "조용히 발동하지 않거나, 반대로 매번 발동하는" 식으로 어긋난 곳을 찾는다.
//! Error가 있으면 새 게임 시작을 막는다 (start_new_game). Warning은 알려주기만 한다.
//! 임포트한 뒤에도 validate_game 커맨드로 돌려서 사용자에게 보여주면 좋다.
use crate::game::runtime::GameDefinition;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
}

fn err(out: &mut Vec<Issue>, m: String) {
    out.push(Issue { severity: Severity::Error, message: m });
}
fn warn(out: &mut Vec<Issue>, m: String) {
    out.push(Issue { severity: Severity::Warning, message: m });
}

/// 효과 JSON이 가리키는 (enter_scene, fire_event) id.
fn effect_refs(effect: &serde_json::Value) -> (Option<&str>, Option<&str>) {
    (effect.get("enter_scene").and_then(|v| v.as_str()), effect.get("fire_event").and_then(|v| v.as_str()))
}

pub fn validate(def: &GameDefinition) -> Vec<Issue> {
    use crate::game::engine::Condition;
    let mut out = Vec::new();

    // ---- 시작에 꼭 필요한 것
    if def.maps.is_empty() {
        err(&mut out, "맵이 하나도 없습니다".into());
    }
    if def.locations.is_empty() {
        err(&mut out, "location이 하나도 없습니다".into());
    }
    if def.weather_types.is_empty() {
        err(&mut out, "날씨 타입이 하나도 없습니다".into());
    }
    if def.time_periods.is_empty() {
        err(&mut out, "시간대가 하나도 없습니다".into());
    } else {
        let uncovered = (0u32..1440)
            .filter(|m| !def.time_periods.iter().any(|p| if p.start_minute <= p.end_minute { *m >= p.start_minute && *m < p.end_minute } else { *m >= p.start_minute || *m < p.end_minute }))
            .count();
        if uncovered > 0 {
            warn(&mut out, format!("시간대가 덮지 못하는 시각이 {uncovered}분 있습니다 (그 시각엔 이전 시간대가 유지됩니다)"));
        }
    }
    if let Some(id) = &def.project.start_location_id {
        if def.location(id).is_none() {
            err(&mut out, format!("시작 location(start_location_id={id})이 존재하지 않습니다"));
        }
    }
    if let Some(key) = &def.project.start_scene_key {
        if !def.scenes.iter().any(|s| &s.scene_key == key) {
            err(&mut out, format!("시작 씬(start_scene_key={key})이 존재하지 않습니다"));
        }
    }

    // ---- 참조 집합
    let interactable_event_ids: HashSet<&str> = def.interactables.values().flatten().filter_map(|i| i.event_id.as_deref()).collect();
    let random_event_ids: HashSet<&str> = def.encounter_entries.iter().map(|e| e.event_id.as_str()).collect();

    // 효과/선택지에서 fire_event로 불리는 이벤트, 그리고 효과의 참조 검증
    let mut fired: HashSet<String> = HashSet::new();
    let mut check_effect = |out: &mut Vec<Issue>, who: &str, effect: &serde_json::Value| {
        let (scene, event) = effect_refs(effect);
        if let Some(id) = scene {
            if def.scene(id).is_none() {
                err(out, format!("{who}: enter_scene이 존재하지 않는 씬({id})을 가리킵니다"));
            }
        }
        if let Some(id) = event {
            match def.events.get(id) {
                None => err(out, format!("{who}: fire_event가 존재하지 않는 이벤트({id})를 가리킵니다")),
                Some(e) if e.trigger_type != "manual" => err(out, format!("{who}: fire_event 대상({id})은 trigger_type이 manual이어야 합니다 (지금 {})", e.trigger_type)),
                Some(_) => {
                    fired.insert(id.to_string());
                }
            }
        }
    };

    for e in def.events.values() {
        check_effect(&mut out, &format!("이벤트 '{}'", e.name), &e.effect_json);
    }
    for scene in &def.scenes {
        for c in scene.choices.iter().flatten() {
            if let Some(next) = &c.next_scene_id {
                if def.scene(next).is_none() {
                    err(&mut out, format!("씬 '{}'의 선택지 '{}'가 존재하지 않는 씬({next})으로 이어집니다", scene.scene_name, c.text));
                }
            }
            if let Some(raw) = &c.effect {
                match serde_json::from_str::<serde_json::Value>(raw) {
                    Ok(v) => check_effect(&mut out, &format!("씬 '{}'의 선택지 '{}'", scene.scene_name, c.text), &v),
                    Err(_) => warn(&mut out, format!("씬 '{}'의 선택지 '{}' effect가 올바른 JSON이 아닙니다", scene.scene_name, c.text)),
                }
            }
        }
        for b in scene.branches.iter().flatten() {
            if let Some(next) = &b.next_scene_id {
                if def.scene(next).is_none() {
                    err(&mut out, format!("씬 '{}'의 분기가 존재하지 않는 씬({next})으로 이어집니다", scene.scene_name));
                }
            }
        }
        if scene.is_ending == Some(true) && scene.ending_id.is_none() {
            warn(&mut out, format!("엔딩 씬 '{}'에 ending_id가 없어 GameEnded가 나가지 않습니다", scene.scene_name));
        }
    }

    // ---- 이벤트 종류별 연결/조건
    for e in def.events.values() {
        let conds: &[Condition] = def.event_conditions.get(&e.id).map(|v| v.as_slice()).unwrap_or(&[]);
        let name = &e.name;
        match e.trigger_type.as_str() {
            "interaction" if !interactable_event_ids.contains(e.id.as_str()) => {
                warn(&mut out, format!("이벤트 '{name}'(interaction)을 가리키는 오브젝트가 없어 발동하지 않습니다"));
            }
            "random" if !random_event_ids.contains(e.id.as_str()) => {
                warn(&mut out, format!("이벤트 '{name}'(random)이 어느 인카운터 테이블에도 없어 발동하지 않습니다"));
            }
            "scheduled" if !conds.iter().any(|c| matches!(c, Condition::AtMinuteOfDay { .. })) => {
                warn(&mut out, format!("이벤트 '{name}'(scheduled)에 AtMinuteOfDay 조건이 없어 시간이 흐를 때마다 발동합니다"));
            }
            "location" if !conds.iter().any(|c| matches!(c, Condition::InLocation { .. } | Condition::InZone { .. })) => {
                warn(&mut out, format!("이벤트 '{name}'(location)에 InLocation/InZone 조건이 없어 모든 이동마다 발동합니다"));
            }
            "manual" if !fired.contains(&e.id) => {
                warn(&mut out, format!("이벤트 '{name}'(manual)을 fire_event로 부르는 곳이 없어 발동하지 않습니다"));
            }
            _ => {}
        }
        if e.requires_npc && def.event_npc_pool.get(&e.id).map_or(true, |p| p.is_empty()) {
            warn(&mut out, format!("이벤트 '{name}'은 NPC가 필요한데 후보 풀이 비어 있습니다"));
        }
    }

    // ---- 연결 테이블이 가리키는 이벤트
    for i in def.interactables.values().flatten() {
        if let Some(event_id) = &i.event_id {
            match def.events.get(event_id) {
                None => err(&mut out, format!("오브젝트 '{}'가 존재하지 않는 이벤트({event_id})를 가리킵니다", i.name)),
                Some(e) if e.trigger_type != "interaction" => {
                    warn(&mut out, format!("오브젝트 '{}'에 연결된 이벤트의 trigger_type이 interaction이 아니라 발동하지 않습니다 (지금 {})", i.name, e.trigger_type))
                }
                Some(_) => {}
            }
        }
    }
    for en in &def.encounter_entries {
        match def.events.get(&en.event_id) {
            None => err(&mut out, format!("인카운터 항목이 존재하지 않는 이벤트({})를 가리킵니다", en.event_id)),
            Some(e) if e.trigger_type != "random" => {
                warn(&mut out, format!("인카운터 테이블에 있는 이벤트 '{}'의 trigger_type이 random이 아니라 무시됩니다 (지금 {})", e.name, e.trigger_type))
            }
            Some(_) => {}
        }
    }

    // ---- AI 변수 설정
    for d in &def.state_defs {
        if !d.config_json.trim().is_empty() && serde_json::from_str::<serde_json::Value>(&d.config_json).is_err() {
            warn(&mut out, format!("변수 '{}'의 config_json이 올바른 JSON이 아니라 AI 접근이 모두 막힙니다", d.key));
        }
    }
    out
}

/// Error만 모아 한 문장으로. start_new_game이 시작을 막을 때 쓴다.
pub fn errors_summary(issues: &[Issue]) -> Option<String> {
    let errors: Vec<&str> = issues.iter().filter(|i| i.severity == Severity::Error).map(|i| i.message.as_str()).collect();
    if errors.is_empty() { None } else { Some(errors.join("; ")) }
}
