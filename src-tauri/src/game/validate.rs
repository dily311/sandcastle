//! 게임 정의의 일관성 검사 (순수 함수, DB 없음). 제작자가 만든(또는 임포트한) 데이터가
//! "조용히 발동하지 않거나, 반대로 매번 발동하는" 식으로 어긋난 곳을 찾는다.
//! Error가 있으면 새 게임 시작을 막는다 (start_new_game). Warning은 알려주기만 한다.
//!
//! 진단은 "같은 문구끼리 묶어서" 돌려준다: `{severity, message, targets, text}`.
//!   message = 문제의 문구(항목마다 달라지는 부분은 없다), targets = 문제가 있는 곳들, text = "문구: [곳1, 곳2]".
//! 코딩을 모르는 제작자도 고칠 수 있게 targets에는 id가 아니라 이름(이벤트 이름, 위치 이름, 캐릭터 이름 등)을 쓴다.
//! id밖에 알 수 없는 경우(가리키는 항목이 지워진 경우 등)에만 id를 괄호 안에 덧붙인다.
use crate::game::runtime::GameDefinition;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub severity: Severity,
    /// 문제의 문구. 같은 문구는 한 항목으로 묶인다.
    pub message: String,
    /// 문제가 있는 곳들 (이름). 데이터 전체에 대한 문제면 비어 있다.
    pub targets: Vec<String>,
    /// 바로 보여줄 수 있는 문장: "문구: [곳1, 곳2]" (targets가 없으면 문구만).
    pub text: String,
}

/// 묶기 전의 진단 한 건.
struct Raw {
    severity: Severity,
    message: String,
    target: Option<String>,
}

fn err(out: &mut Vec<Raw>, message: &str, target: impl Into<String>) {
    out.push(Raw {
        severity: Severity::Error,
        message: message.to_string(),
        target: Some(target.into()),
    });
}
fn warn(out: &mut Vec<Raw>, message: &str, target: impl Into<String>) {
    out.push(Raw {
        severity: Severity::Warning,
        message: message.to_string(),
        target: Some(target.into()),
    });
}
fn err_all(out: &mut Vec<Raw>, message: &str) {
    out.push(Raw {
        severity: Severity::Error,
        message: message.to_string(),
        target: None,
    });
}

/// 같은 (심각도, 문구)를 하나로 묶고 곳들은 중복 없이 처음 나온 순서로 모은다. 에러가 경고보다 앞에 온다.
fn group(raw: Vec<Raw>) -> Vec<Issue> {
    let mut order: Vec<(Severity, String)> = Vec::new();
    let mut targets: std::collections::HashMap<(bool, String), Vec<String>> = std::collections::HashMap::new();
    for r in raw {
        let key = (r.severity == Severity::Error, r.message.clone());
        let list = targets.entry(key).or_insert_with(|| {
            order.push((r.severity, r.message.clone()));
            Vec::new()
        });
        if let Some(t) = r.target {
            if !list.contains(&t) {
                list.push(t);
            }
        }
    }
    let mut issues: Vec<Issue> = order
        .into_iter()
        .map(|(severity, message)| {
            let t = targets.remove(&(severity == Severity::Error, message.clone())).unwrap_or_default();
            let text = if t.is_empty() {
                message.clone()
            } else {
                format!("{message}: [{}]", t.join(", "))
            };
            Issue {
                severity,
                message,
                targets: t,
                text,
            }
        })
        .collect();
    issues.sort_by_key(|i| i.severity != Severity::Error); // 안정 정렬: 에러가 먼저
    issues
}

/// 효과 JSON이 가리키는 (enter_scene, fire_event) id.
fn effect_refs(effect: &serde_json::Value) -> (Option<&str>, Option<&str>) {
    (
        effect.get("enter_scene").and_then(|v| v.as_str()),
        effect.get("fire_event").and_then(|v| v.as_str()),
    )
}

fn char_name(def: &GameDefinition, id: &str) -> String {
    if id == "player" {
        "플레이어".to_string()
    } else {
        def.characters
            .get(id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("알 수 없는 캐릭터(id: {id})"))
    }
}

fn location_name(def: &GameDefinition, id: &str) -> String {
    def.location(id).map(|l| l.name.clone()).unwrap_or_else(|| format!("알 수 없는 위치(id: {id})"))
}

fn asset_label(key: &str, name: &str) -> String {
    if name.trim().is_empty() || name == key {
        format!("에셋 '{key}'")
    } else {
        format!("에셋 '{name}' ({key})")
    }
}

/// 데이터 일관성 진단 (DB/파일 접근 없음).
pub fn validate(def: &GameDefinition) -> Vec<Issue> {
    group(raw_validate(def))
}

fn raw_validate(def: &GameDefinition) -> Vec<Raw> {
    use crate::game::engine::Condition;
    let mut out: Vec<Raw> = Vec::new();

    // 이름순으로 훑어서 같은 데이터면 같은 순서로 나오게 한다 (HashMap 순회 순서는 매번 다르다).
    let mut events: Vec<_> = def.events.values().collect();
    events.sort_by(|a, b| a.name.cmp(&b.name));
    let mut items: Vec<_> = def.items.values().collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    let mut interactables: Vec<_> = def.interactables.values().flatten().collect();
    interactables.sort_by(|a, b| a.name.cmp(&b.name));

    // ---- 시작에 꼭 필요한 것
    if def.maps.is_empty() {
        err_all(&mut out, "맵이 하나도 없습니다");
    }
    if def.locations.is_empty() {
        err_all(&mut out, "위치(location)가 하나도 없습니다");
    }
    if def.weather_types.is_empty() {
        err_all(&mut out, "날씨 타입이 하나도 없습니다");
    }
    if def.time_periods.is_empty() {
        err_all(&mut out, "시간대가 하나도 없습니다");
    } else {
        let uncovered = (0u32..1440)
            .filter(|m| {
                !def.time_periods.iter().any(|p| {
                    if p.start_minute <= p.end_minute {
                        *m >= p.start_minute && *m < p.end_minute
                    } else {
                        *m >= p.start_minute || *m < p.end_minute
                    }
                })
            })
            .count();
        if uncovered > 0 {
            warn(
                &mut out,
                "시간대가 덮지 못하는 시각이 있습니다 (그 시각엔 이전 시간대가 유지됩니다)",
                format!("하루 중 {uncovered}분"),
            );
        }
    }
    if let Some(id) = &def.project.start_location_id {
        if def.location(id).is_none() {
            err(
                &mut out,
                "시작 위치 설정이 존재하지 않는 위치를 가리킵니다",
                format!("프로젝트 설정 (id: {id})"),
            );
        }
    }
    if let Some(key) = &def.project.start_scene_key {
        if !def.scenes.iter().any(|s| &s.scene_key == key) {
            err(
                &mut out,
                "시작 씬 설정이 존재하지 않는 씬을 가리킵니다",
                format!("프로젝트 설정 (씬 키: {key})"),
            );
        }
    }

    // ---- 참조 집합
    let interactable_event_ids: HashSet<&str> = def.interactables.values().flatten().filter_map(|i| i.event_id.as_deref()).collect();
    let random_event_ids: HashSet<&str> = def.encounter_entries.iter().map(|e| e.event_id.as_str()).collect();

    // 효과/선택지에서 fire_event로 불리는 이벤트, 그리고 효과의 참조 검증. who = 효과가 있는 곳의 이름.
    let mut fired: HashSet<String> = HashSet::new();
    let mut check_effect = |out: &mut Vec<Raw>, who: &str, effect: &serde_json::Value| {
        for verb in ["give_item", "take_item"] {
            if let Some(serde_json::Value::Object(m)) = effect.get(verb) {
                for id in m.keys() {
                    if !def.items.contains_key(id) {
                        err(out, "효과가 존재하지 않는 아이템을 가리킵니다", format!("{who} ({verb}, id: {id})"));
                    }
                }
            }
        }
        if let Some(serde_json::Value::Object(pairs)) = effect.get("relation") {
            for (key, axes) in pairs {
                match key.split_once('>') {
                    None => err(out, "relation 키가 '보낸쪽>받는쪽' 형식이 아닙니다", format!("{who} ('{key}')")),
                    Some((from, to)) => {
                        for end in [from, to] {
                            if end != "player" && !def.characters.contains_key(end) {
                                err(
                                    out,
                                    "relation이 존재하지 않는 캐릭터를 가리킵니다 ('player'가 아니면 캐릭터여야 합니다)",
                                    format!("{who} (id: {end})"),
                                );
                            }
                        }
                    }
                }
                for axis in axes.as_object().into_iter().flat_map(|m| m.keys()) {
                    if !crate::game::engine::RELATION_AXES.contains(&axis.as_str()) {
                        err(out, "알 수 없는 관계 축입니다 (affection/trust/tension만 가능)", format!("{who} ('{axis}')"));
                    }
                }
            }
        }
        let (scene, event) = effect_refs(effect);
        if let Some(id) = scene {
            if def.scene(id).is_none() {
                err(out, "효과 enter_scene이 존재하지 않는 씬을 가리킵니다", format!("{who} (id: {id})"));
            }
        }
        if let Some(id) = event {
            match def.events.get(id) {
                None => err(out, "효과 fire_event가 존재하지 않는 이벤트를 가리킵니다", format!("{who} (id: {id})")),
                Some(e) if e.trigger_type != "manual" => err(
                    out,
                    "효과 fire_event의 대상 이벤트는 trigger_type이 manual이어야 합니다",
                    format!("{who} → 이벤트 '{}' (지금 {})", e.name, e.trigger_type),
                ),
                Some(_) => {
                    fired.insert(id.to_string());
                }
            }
        }
    };

    for e in &events {
        check_effect(&mut out, &format!("이벤트 '{}'", e.name), &e.effect_json);
    }
    for scene in &def.scenes {
        for c in scene.choices.iter().flatten() {
            let who = format!("씬 '{}' › 선택지 '{}'", scene.scene_name, c.text);
            if let Some(next) = &c.next_scene_id {
                if def.scene(next).is_none() {
                    err(&mut out, "선택지가 존재하지 않는 씬으로 이어집니다", format!("{who} (id: {next})"));
                }
            }
            if let Some(raw) = &c.effect {
                match serde_json::from_str::<serde_json::Value>(raw) {
                    Ok(v) => check_effect(&mut out, &who, &v),
                    Err(_) => warn(&mut out, "선택지 effect가 올바른 JSON이 아닙니다", who),
                }
            }
        }
        for b in scene.branches.iter().flatten() {
            if let Some(next) = &b.next_scene_id {
                if def.scene(next).is_none() {
                    err(
                        &mut out,
                        "분기가 존재하지 않는 씬으로 이어집니다",
                        format!("씬 '{}' (id: {next})", scene.scene_name),
                    );
                }
            }
        }
        if scene.is_ending == Some(true) && scene.ending_id.is_none() {
            warn(
                &mut out,
                "엔딩 씬에 ending_id가 없어 엔딩 알림이 나가지 않습니다",
                format!("씬 '{}'", scene.scene_name),
            );
        }
    }

    // ---- 아이템 / 관계
    let mut newly_fired_items = Vec::new();
    for i in &items {
        check_effect(&mut out, &format!("아이템 '{}' (사용 효과)", i.name), &i.use_effect);
        if !(i.use_condition.is_null() || i.use_condition.as_object().map_or(false, |o| o.is_empty()))
            && serde_json::from_value::<Condition>(i.use_condition.clone()).is_err()
        {
            warn(
                &mut out,
                "아이템의 사용 조건을 읽을 수 없어 아무도 사용할 수 없습니다",
                format!("아이템 '{}'", i.name),
            );
        }
        if let Some(acquire) = i.on_acquire_event_id.as_deref() {
            if !acquire.trim().is_empty() {
                match def.events.get(acquire) {
                    None => err(
                        &mut out,
                        "아이템의 획득 이벤트가 존재하지 않습니다",
                        format!("아이템 '{}' (id: {acquire})", i.name),
                    ),
                    Some(e) if e.trigger_type != "manual" => err(
                        &mut out,
                        "아이템의 획득 이벤트는 trigger_type이 manual이어야 합니다",
                        format!("아이템 '{}' → 이벤트 '{}' (지금 {})", i.name, e.name, e.trigger_type),
                    ),
                    Some(_) => {
                        newly_fired_items.push(acquire.to_string()); // 이 아이템이 부르는 manual 이벤트는 "호출됨"으로 센다
                    }
                }
            }
        }
    }
    fired.extend(newly_fired_items);

    // 이벤트 조건이 가리키는 아이템/캐릭터/축
    let mut cond_owners: Vec<(&String, &Vec<Condition>)> = def.event_conditions.iter().collect();
    cond_owners.sort_by(|a, b| {
        let na = def.events.get(a.0).map(|e| e.name.as_str()).unwrap_or("");
        let nb = def.events.get(b.0).map(|e| e.name.as_str()).unwrap_or("");
        na.cmp(nb)
    });
    for (event_id, conds) in cond_owners {
        let owner = def
            .events
            .get(event_id)
            .map(|e| format!("이벤트 '{}'", e.name))
            .unwrap_or_else(|| format!("이벤트(id: {event_id})"));
        for c in conds {
            match c {
                Condition::HasItem { item_id, .. } if !def.items.contains_key(item_id) => {
                    err(&mut out, "조건 HasItem이 존재하지 않는 아이템을 가리킵니다", format!("{owner} (id: {item_id})"));
                }
                Condition::RelationAtLeast { from, to, axis, .. } | Condition::RelationAtMost { from, to, axis, .. } => {
                    if !crate::game::engine::RELATION_AXES.contains(&axis.as_str()) {
                        err(
                            &mut out,
                            "관계 조건의 축을 알 수 없습니다 (affection/trust/tension만 가능)",
                            format!("{owner} ('{axis}')"),
                        );
                    }
                    for end in [from, to] {
                        if end != "player" && !def.characters.contains_key(end.as_str()) {
                            err(&mut out, "관계 조건이 존재하지 않는 캐릭터를 가리킵니다", format!("{owner} (id: {end})"));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // NPC 스폰 규칙: "캐릭터 → 위치"
    let mut spawn_rows: Vec<(String, &crate::models::game::GameCharacterSpawnLocations)> = def
        .spawn_locations
        .iter()
        .flat_map(|(cid, rows)| rows.iter().map(move |r| (cid.clone(), r)))
        .map(|(cid, r)| (char_name(def, &cid), r))
        .collect();
    spawn_rows.sort_by(|a, b| a.0.cmp(&b.0));
    for (cname, s) in spawn_rows {
        let target = format!("{cname} → {}", location_name(def, &s.location_id));
        if def.location(&s.location_id).is_none() {
            err(&mut out, "NPC 스폰 규칙이 존재하지 않는 위치를 가리킵니다", target.clone());
        }
        let c = &s.condition_json;
        if !(c.is_null() || c.as_object().map_or(false, |o| o.is_empty())) && serde_json::from_value::<Condition>(c.clone()).is_err() {
            warn(&mut out, "NPC 스폰 규칙의 조건을 읽을 수 없어 그 위치에는 절대 나오지 않습니다", target);
        }
    }
    for r in &def.relationship_defs {
        for end in [&r.from_id, &r.to_id] {
            if end != "player" && !def.characters.contains_key(end.as_str()) {
                err(
                    &mut out,
                    "초기 관계가 존재하지 않는 캐릭터를 가리킵니다 ('player'가 아니면 캐릭터여야 합니다)",
                    format!("{} → {} (id: {end})", char_name(def, &r.from_id), char_name(def, &r.to_id)),
                );
            }
        }
    }

    // ---- StoryPlan 표시
    let plan_on = crate::game::story_plan::settings(def).enabled;
    for e in &events {
        if e.config_json.get("planUsable").and_then(|v| v.as_bool()) == Some(true) && e.trigger_type != "manual" {
            warn(
                &mut out,
                "StoryPlan이 올릴 수 있는 이벤트는 trigger_type이 manual이어야 합니다 (planUsable이 무시됩니다)",
                format!("이벤트 '{}' (지금 {})", e.name, e.trigger_type),
            );
        }
    }
    if !plan_on && (events.iter().any(|e| crate::game::story_plan::event_plan_usable(e)) || def.scenes.iter().any(crate::game::story_plan::scene_plan_usable)) {
        warn(
            &mut out,
            "planUsable로 표시한 이벤트/씬이 있지만 StoryPlan이 꺼져 있습니다 (settings.storyPlan.enabled)",
            "프로젝트 설정",
        );
    }

    // ---- 이벤트 종류별 연결/조건
    for e in &events {
        let conds: &[Condition] = def.event_conditions.get(&e.id).map(|v| v.as_slice()).unwrap_or(&[]);
        let target = format!("이벤트 '{}'", e.name);
        match e.trigger_type.as_str() {
            "interaction" if !interactable_event_ids.contains(e.id.as_str()) => {
                warn(&mut out, "interaction 이벤트를 가리키는 오브젝트가 없어 발동하지 않습니다", target.clone());
            }
            "random" if !random_event_ids.contains(e.id.as_str()) => {
                warn(&mut out, "random 이벤트가 어느 인카운터 테이블에도 없어 발동하지 않습니다", target.clone());
            }
            "scheduled" if !conds.iter().any(|c| matches!(c, Condition::AtMinuteOfDay { .. })) => {
                warn(
                    &mut out,
                    "scheduled 이벤트에 AtMinuteOfDay(시각) 조건이 없어 시간이 흐를 때마다 발동합니다",
                    target.clone(),
                );
            }
            "location" if !conds.iter().any(|c| matches!(c, Condition::InLocation { .. } | Condition::InZone { .. })) => {
                warn(
                    &mut out,
                    "location 이벤트에 InLocation/InZone 조건이 없어 모든 이동마다 발동합니다",
                    target.clone(),
                );
            }
            // StoryPlan을 켠 게임에서 제작자가 맡긴(planUsable) manual 이벤트는 각본가가 올릴 수 있다.
            "manual" if !fired.contains(&e.id) && !(plan_on && crate::game::story_plan::event_plan_usable(e)) => {
                warn(&mut out, "manual 이벤트를 fire_event로 부르는 곳이 없어 발동하지 않습니다", target.clone());
            }
            _ => {}
        }
        if e.requires_npc && def.event_npc_pool.get(&e.id).map_or(true, |p| p.is_empty()) {
            warn(&mut out, "NPC가 필요한 이벤트인데 후보 풀이 비어 있습니다", target);
        }
    }

    // ---- 연결 테이블이 가리키는 이벤트
    for i in &interactables {
        if let Some(event_id) = &i.event_id {
            match def.events.get(event_id) {
                None => err(
                    &mut out,
                    "오브젝트가 존재하지 않는 이벤트를 가리킵니다",
                    format!("오브젝트 '{}' (id: {event_id})", i.name),
                ),
                Some(e) if e.trigger_type != "interaction" => warn(
                    &mut out,
                    "오브젝트에 연결된 이벤트의 trigger_type이 interaction이 아니라 발동하지 않습니다",
                    format!("오브젝트 '{}' → 이벤트 '{}' (지금 {})", i.name, e.name, e.trigger_type),
                ),
                Some(_) => {}
            }
        }
    }
    for en in &def.encounter_entries {
        // 항목이 속한 테이블을 "맵 › 존" 이름으로 보여준다.
        let table = def.encounter_tables.iter().find(|t| t.id == en.encounter_table_id).map(|t| {
            let map = def.maps.iter().find(|m| m.id == t.map_id).map(|m| m.name.clone()).unwrap_or_default();
            match t.zone_id.as_ref().and_then(|z| def.zones.iter().find(|zz| &zz.id == z)) {
                Some(z) => format!("{map} › {}", z.name),
                None => map,
            }
        });
        let table = table.unwrap_or_else(|| "알 수 없는 테이블".to_string());
        match def.events.get(&en.event_id) {
            None => err(
                &mut out,
                "인카운터 항목이 존재하지 않는 이벤트를 가리킵니다",
                format!("인카운터 테이블 '{table}' (id: {})", en.event_id),
            ),
            Some(e) if e.trigger_type != "random" => warn(
                &mut out,
                "인카운터 테이블에 있는 이벤트의 trigger_type이 random이 아니라 무시됩니다",
                format!("인카운터 테이블 '{table}' → 이벤트 '{}' (지금 {})", e.name, e.trigger_type),
            ),
            Some(_) => {}
        }
    }

    validate_assets(def, &mut out);
    validate_capabilities(def, &mut out);

    // ---- AI 변수 설정
    for d in &def.state_defs {
        if !d.config_json.trim().is_empty() && serde_json::from_str::<serde_json::Value>(&d.config_json).is_err() {
            let label = match d.description.as_deref().filter(|s| !s.trim().is_empty()) {
                Some(desc) => format!("변수 '{desc}' ({})", d.key),
                None => format!("변수 '{}'", d.key),
            };
            warn(&mut out, "변수의 config_json이 올바른 JSON이 아니라 AI 접근이 모두 막힙니다", label);
        }
    }
    out
}

/// 꺼진 기능(capabilities)을 쓰는 데이터. 발동할 수 없는 이벤트(scheduled/random/location/interaction)는 Error,
/// 시작 값에 고정되어 의미가 없어지는 조건과 만날 수 없는 NPC는 Warning이다.
fn validate_capabilities(def: &GameDefinition, out: &mut Vec<Raw>) {
    use crate::game::engine::Condition;
    let caps = def.capabilities();
    if caps.time && caps.movement() && caps.location {
        return;
    }
    let time_cond = |c: &Condition| {
        matches!(
            c,
            Condition::MinTime { .. } | Condition::AtMinuteOfDay { .. } | Condition::TimePeriod { .. } | Condition::Weather { .. }
        )
    };
    let place_cond = |c: &Condition| matches!(c, Condition::InLocation { .. } | Condition::InZone { .. });
    // 조건이 놓인 곳(이름)과 조건들. 저장된 문자열 조건은 읽을 수 있는 것만 본다 (못 읽는 건 다른 진단이 알린다).
    let parse = |raw: &str| serde_json::from_str::<Condition>(raw).ok();
    let mut conds: Vec<(String, Condition)> = Vec::new();
    let mut events: Vec<_> = def.events.values().collect();
    events.sort_by(|a, b| a.name.cmp(&b.name));
    for e in &events {
        for c in def.event_conditions.get(&e.id).into_iter().flatten() {
            conds.push((format!("이벤트 '{}'", e.name), c.clone()));
        }
    }
    for c in &def.location_connections {
        if let Some(cond) = c.condition.as_deref().and_then(parse) {
            conds.push((format!("이동 경로 {} → {}", location_name(def, &c.from_id), location_name(def, &c.to_id)), cond));
        }
    }
    for scene in &def.scenes {
        for ch in scene.choices.iter().flatten() {
            if let Some(cond) = ch.condition.as_deref().and_then(parse) {
                conds.push((format!("씬 '{}' › 선택지 '{}'", scene.scene_name, ch.text), cond));
            }
        }
    }
    let mut items: Vec<_> = def.items.values().collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    for i in items {
        if let Ok(cond) = serde_json::from_value::<Condition>(i.use_condition.clone()) {
            conds.push((format!("아이템 '{}' (사용 조건)", i.name), cond));
        }
    }

    if !caps.time {
        for e in &events {
            if matches!(e.trigger_type.as_str(), "scheduled" | "random") {
                err(
                    out,
                    "시간 기능이 꺼진 게임이라 시간이 흐를 때 발동하는 이벤트(scheduled/random)는 발동하지 않습니다",
                    format!("이벤트 '{}' ({})", e.name, e.trigger_type),
                );
            }
        }
        for (who, _) in conds.iter().filter(|(_, c)| time_cond(c)) {
            warn(out, "시간 기능이 꺼진 게임이라 시간/시간대/날씨 조건은 시작 값에 고정됩니다", who.clone());
        }
    }
    if !caps.movement() {
        for e in &events {
            if e.trigger_type == "location" {
                err(
                    out,
                    "지도/이동 기능이 꺼진 게임이라 location 이벤트는 발동하지 않습니다",
                    format!("이벤트 '{}'", e.name),
                );
            }
        }
        for (who, _) in conds.iter().filter(|(_, c)| place_cond(c)) {
            warn(out, "지도/이동 기능이 꺼진 게임이라 위치 조건은 시작 위치에 고정됩니다", who.clone());
        }
        // 플레이어는 시작 위치를 떠날 수 없으니, 거기에 나올 수 없는 NPC는 만날 수 없다.
        let start = def.project.start_location_id.clone().or_else(|| def.locations.first().map(|l| l.id.clone()));
        let mut unreachable: Vec<String> = def
            .spawn_locations
            .iter()
            .filter(|(_, rows)| !rows.is_empty() && !rows.iter().any(|r| Some(&r.location_id) == start.as_ref()))
            .map(|(cid, _)| char_name(def, cid))
            .collect();
        unreachable.sort();
        for name in unreachable {
            warn(out, "지도/이동 기능이 꺼진 게임인데 시작 위치에 나오지 않는 NPC라 만날 수 없습니다", name);
        }
    }
    if !caps.location {
        for e in &events {
            if e.trigger_type == "interaction" {
                err(
                    out,
                    "위치 기능이 꺼진 게임이라 오브젝트 상호작용(interaction) 이벤트는 발동하지 않습니다",
                    format!("이벤트 '{}'", e.name),
                );
            }
        }
    }
}

/// 게임의 진단 결과 전체: 데이터 일관성 + 에셋 파일 존재(game_dir를 주면). 같은 문구는 하나로 묶인다.
/// open_game / save_package / validate_game이 모두 이 함수 하나를 쓰고, 이미 읽어둔 정의와 폴더를 값으로 받는다
/// (호출 쪽이 DB에서 다시 읽지 않아도 된다).
pub fn diagnose(def: &GameDefinition, game_dir: Option<&std::path::Path>) -> Vec<Issue> {
    let mut all = raw_validate(def);
    if let Some(dir) = game_dir {
        all.extend(raw_asset_files(def, dir));
    }
    group(all)
}

/// 에셋 데이터의 일관성 (DB만 본다). 파일이 실제로 있는지는 validate_asset_files가 본다.
fn validate_assets(def: &GameDefinition, out: &mut Vec<Raw>) {
    use crate::game::stage::{self, Allowed, BGM_MOODS};
    use rand::SeedableRng;
    let sub_ids = |ids: Vec<&str>| -> HashSet<String> { ids.into_iter().map(str::to_string).collect() };
    let audio_ids = sub_ids(def.assets.audio.iter().map(|a| a.base.id.as_str()).collect());
    let char_ids = sub_ids(def.assets.characters.iter().map(|a| a.base.id.as_str()).collect());
    let cg_ids = sub_ids(def.assets.cgs.iter().map(|a| a.base.id.as_str()).collect());
    let bg_ids = sub_ids(def.assets.backgrounds.iter().map(|a| a.base.id.as_str()).collect());

    for a in &def.assets.all {
        let label = asset_label(&a.key, &a.name);
        let valid_key = !a.key.is_empty()
            && a.key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'));
        if !valid_key {
            err(out, "에셋 키가 올바르지 않습니다 (영문 소문자, 숫자, . _ - 만 가능)", label.clone());
        }
        let (expected_kind, has_sub) = match a.category.as_str() {
            "voice" | "sfx" | "bgm" => ("audio", audio_ids.contains(&a.id)),
            "character" => ("image", char_ids.contains(&a.id)),
            "cg" => ("image", cg_ids.contains(&a.id)),
            "background" => ("image", bg_ids.contains(&a.id)),
            "font" => ("font", true),
            _ => (a.kind.as_str(), true),
        };
        if a.kind != expected_kind {
            err(
                out,
                "에셋의 종류(kind)가 카테고리와 맞지 않습니다",
                format!("{label} (카테고리 {}는 {expected_kind}여야 하는데 지금 {})", a.category, a.kind),
            );
        }
        if !has_sub {
            warn(
                out,
                "종류별 정보가 없어 쓰이지 않는 에셋입니다 (고아 에셋)",
                format!("{label} ({})", a.category),
            );
        }
    }
    let bad_chars = |s: &str| s.contains(['<', '>', '"', ',', '\n']);
    for a in &def.assets.audio {
        let label = asset_label(&a.base.key, &a.base.name);
        match a.base.category.as_str() {
            "sfx" => match a.tag.as_deref().map(str::trim) {
                None | Some("") => err(out, "효과음에 tag(소리 그룹 이름)가 없습니다", label),
                Some(t) if bad_chars(t) => err(out, "효과음 tag에 < > \" , 줄바꿈은 쓸 수 없습니다", label),
                _ => {}
            },
            "bgm" => match a.tag.as_deref() {
                Some(t) if BGM_MOODS.contains(&t) => {}
                other => err(
                    out,
                    &format!("배경음 tag(분위기)가 올바르지 않습니다 (가능: {})", BGM_MOODS.join(", ")),
                    format!("{label} (지금 {})", other.unwrap_or("없음")),
                ),
            },
            _ => {}
        }
    }
    for a in &def.assets.characters {
        let label = asset_label(&a.base.key, &a.base.name);
        if a.usage == "standing" {
            match a.emotion.as_deref().map(str::trim) {
                None | Some("") => err(out, "스탠딩에 감정명(emotion)이 없습니다", label.clone()),
                Some(e) if bad_chars(e) || e.contains(':') => err(out, "스탠딩의 감정명에 < > : \" , 줄바꿈은 쓸 수 없습니다", label.clone()),
                _ => {}
            }
        }
        if a.game_character_id.is_none() {
            warn(out, "캐릭터가 없어 쓰이지 않는 캐릭터 이미지입니다 (캐릭터가 지워졌을 수 있습니다)", label);
        }
    }

    // 대본의 `이름:`이 모호하지 않게: 이름이 겹치거나 구분자를 포함하면 안 된다.
    let mut names: Vec<_> = def.characters.values().collect();
    names.sort_by(|a, b| a.name.cmp(&b.name));
    let mut seen: HashSet<String> = HashSet::new();
    for c in names {
        let label = format!("캐릭터 '{}'", c.name);
        if c.name.contains([':', '：', '<', '>']) {
            err(
                out,
                "캐릭터 이름에 : < > 는 쓸 수 없습니다 (대본의 이름: 접두와 연출 태그에 쓰입니다)",
                label.clone(),
            );
        }
        if c.name.chars().count() > 20 {
            warn(out, "캐릭터 이름이 20자를 넘어 대본에서 화자로 인식되지 않습니다", label.clone());
        }
        if !seen.insert(c.name.trim().to_lowercase()) {
            warn(out, "캐릭터 이름이 겹쳐서 대본의 이름: 접두가 모호합니다", label);
        }
    }

    // 씬 대본에서 쓸 수 없는 태그(키 없음 등)
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    for s in &def.scenes {
        if let Some(script) = s.script.as_deref().filter(|t| !t.trim().is_empty()) {
            let mut r = crate::game::runtime::DefResolver {
                def,
                rng: &mut rng,
                ai: false,
                player_name: None,
            };
            for p in stage::lint(script, &mut r, &Allowed::author()) {
                warn(
                    out,
                    "씬 대본에 쓸 수 없는 연출 태그가 있습니다 (키/감정/효과 이름을 확인하세요)",
                    format!("씬 '{}': {p}", s.scene_name),
                );
            }
        }
    }
}

/// 에셋 파일이 게임 폴더에 실제로 있는지 (파일시스템이 필요해서 validate와 따로 둔다).
pub fn validate_asset_files(def: &GameDefinition, game_dir: &std::path::Path) -> Vec<Issue> {
    group(raw_asset_files(def, game_dir))
}

fn raw_asset_files(def: &GameDefinition, game_dir: &std::path::Path) -> Vec<Raw> {
    let mut out = Vec::new();
    for a in &def.assets.all {
        let label = asset_label(&a.key, &a.name);
        if crate::game::package::check_asset_path(&a.file_path).is_err() {
            err(&mut out, "에셋 경로가 올바르지 않습니다", format!("{label} ({})", a.file_path));
            continue;
        }
        let path = game_dir.join(&a.file_path);
        if !path.is_file() {
            err(&mut out, "에셋 파일이 없습니다", format!("{label} ({})", a.file_path));
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !crate::game::protocol::is_servable(ext) {
            warn(&mut out, "지원하지 않는 확장자라 재생/표시되지 않는 에셋입니다", format!("{label} (.{ext})"));
        }
    }
    out
}

/// Error만 모아 한 문장으로. start_new_game이 시작을 막을 때 쓴다.
pub fn errors_summary(issues: &[Issue]) -> Option<String> {
    let errors: Vec<&str> = issues.iter().filter(|i| i.severity == Severity::Error).map(|i| i.text.as_str()).collect();
    if errors.is_empty() {
        None
    } else {
        Some(errors.join("; "))
    }
}
