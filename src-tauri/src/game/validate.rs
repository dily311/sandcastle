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
        for verb in ["give_item", "take_item"] {
            if let Some(serde_json::Value::Object(m)) = effect.get(verb) {
                for id in m.keys() {
                    if !def.items.contains_key(id) {
                        err(out, format!("{who}: {verb}가 존재하지 않는 아이템({id})을 가리킵니다"));
                    }
                }
            }
        }
        if let Some(serde_json::Value::Object(pairs)) = effect.get("relation") {
            for (key, axes) in pairs {
                match key.split_once('>') {
                    None => err(out, format!("{who}: relation 키 '{key}'는 \"from>to\" 형식이어야 합니다")),
                    Some((from, to)) => {
                        for end in [from, to] {
                            if end != "player" && !def.characters.contains_key(end) {
                                err(out, format!("{who}: relation이 존재하지 않는 캐릭터({end})를 가리킵니다 ('player'가 아니면 캐릭터 id)"));
                            }
                        }
                    }
                }
                for axis in axes.as_object().into_iter().flat_map(|m| m.keys()) {
                    if !crate::game::engine::RELATION_AXES.contains(&axis.as_str()) {
                        err(out, format!("{who}: 알 수 없는 관계 축 '{axis}' (affection/trust/tension만 가능)"));
                    }
                }
            }
        }
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

    // ---- 아이템 / 관계
    let mut newly_fired_items = Vec::new();
    for i in def.items.values() {
        check_effect(&mut out, &format!("아이템 '{}'의 use_effect", i.name), &i.use_effect);
        if !(i.use_condition.is_null() || i.use_condition.as_object().map_or(false, |o| o.is_empty()))
            && serde_json::from_value::<Condition>(i.use_condition.clone()).is_err()
        {
            warn(&mut out, format!("아이템 '{}'의 use_condition을 읽을 수 없어 아무도 사용할 수 없습니다", i.name));
        }
        if let Some (acquire) = i.on_acquire_event_id.as_deref() {
            if !acquire.trim().is_empty() {
                match def.events.get(acquire) {
                    None => err(&mut out, format!("아이템 '{}'의 획득 이벤트({acquire})가 존재하지 않습니다", i.name)),
                    Some(e) if e.trigger_type != "manual" => err(&mut out, format!("아이템 '{}'의 획득 이벤트는 trigger_type이 manual이어야 합니다 (지금 {})", i.name, e.trigger_type)),
                    Some(_) => {
                        newly_fired_items.push(acquire.to_string()); // 이 아이템이 부르는 manual 이벤트는 "호출됨"으로 센다
                    }
                }
            }
        }
    }
    fired.extend(newly_fired_items);

    for conds in def.event_conditions.values() {
        for c in conds {
            if let Condition::HasItem { item_id, .. } = c {
                if !def.items.contains_key(item_id) {
                    err(&mut out, format!("조건 HasItem이 존재하지 않는 아이템({item_id})을 가리킵니다"));
                }
            }
        }
    }
    for conds in def.event_conditions.values() {
        for c in conds {
            if let Condition::RelationAtLeast { from, to, axis, .. } | Condition::RelationAtMost { from, to, axis, .. } = c {
                if !crate::game::engine::RELATION_AXES.contains(&axis.as_str()) {
                    err(&mut out, format!("관계 조건의 축 '{axis}'을 알 수 없습니다 (affection/trust/tension만 가능)"));
                }
                for end in [from, to] {
                    if end != "player" && !def.characters.contains_key(end.as_str()) {
                        err(&mut out, format!("관계 조건이 존재하지 않는 캐릭터({end})를 가리킵니다"));
                    }
                }
            }
        }
    }
    for s in def.spawn_locations.values().flatten() {
        if def.location(&s.location_id).is_none() {
            err(&mut out, format!("NPC 스폰 규칙이 존재하지 않는 location({})을 가리킵니다", s.location_id));
        }
        let c = &s.condition_json;
        if !(c.is_null() || c.as_object().map_or(false, |o| o.is_empty())) && serde_json::from_value::<Condition>(c.clone()).is_err() {
            warn(&mut out, format!("NPC 스폰 규칙(location {})의 조건을 읽을 수 없어 그 위치에는 절대 나오지 않습니다", s.location_id));
        }
    }
    for r in &def.relationship_defs {
        for end in [&r.from_id, &r.to_id] {
            if end != "player" && !def.characters.contains_key(end.as_str()) {
                err(&mut out, format!("초기 관계가 존재하지 않는 캐릭터({end})를 가리킵니다 ('player'가 아니면 캐릭터 id여야 합니다)"));
            }
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

    validate_assets(def, &mut out);

    // ---- AI 변수 설정
    for d in &def.state_defs {
        if !d.config_json.trim().is_empty() && serde_json::from_str::<serde_json::Value>(&d.config_json).is_err() {
            warn(&mut out, format!("변수 '{}'의 config_json이 올바른 JSON이 아니라 AI 접근이 모두 막힙니다", d.key));
        }
    }
    out
}

/// 에셋 데이터의 일관성 (DB만 본다). 파일이 실제로 있는지는 validate_asset_files가 본다.
fn validate_assets(def: &GameDefinition, out: &mut Vec<Issue>) {
    use crate::game::stage::{self, Allowed, BGM_MOODS};
    use rand::SeedableRng;
    let sub_ids = |ids: Vec<&str>| -> HashSet<String> { ids.into_iter().map(str::to_string).collect() };
    let audio_ids = sub_ids(def.assets.audio.iter().map(|a| a.base.id.as_str()).collect());
    let char_ids = sub_ids(def.assets.characters.iter().map(|a| a.base.id.as_str()).collect());
    let cg_ids = sub_ids(def.assets.cgs.iter().map(|a| a.base.id.as_str()).collect());
    let bg_ids = sub_ids(def.assets.backgrounds.iter().map(|a| a.base.id.as_str()).collect());

    for a in &def.assets.all {
        let valid_key = !a.key.is_empty() && a.key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'));
        if !valid_key {
            err(out, format!("에셋 키 '{}'가 올바르지 않습니다 (영문 소문자, 숫자, . _ - 만 가능)", a.key));
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
            err(out, format!("에셋 '{}': 카테고리 {}는 종류가 {expected_kind}여야 합니다 (지금 {})", a.key, a.category, a.kind));
        }
        if !has_sub {
            warn(out, format!("에셋 '{}'({})에 종류별 정보가 없어 쓰이지 않습니다 (고아 에셋)", a.key, a.category));
        }
    }
    let bad_chars = |s: &str| s.contains(['<', '>', '"', ',', '\n']);
    for a in &def.assets.audio {
        match a.base.category.as_str() {
            "sfx" => match a.tag.as_deref().map(str::trim) {
                None | Some("") => err(out, format!("효과음 '{}'에 tag(소리 그룹 이름)가 없습니다", a.base.key)),
                Some(t) if bad_chars(t) => err(out, format!("효과음 '{}'의 tag에 < > \" , 줄바꿈은 쓸 수 없습니다", a.base.key)),
                _ => {}
            },
            "bgm" => match a.tag.as_deref() {
                Some(t) if BGM_MOODS.contains(&t) => {}
                other => err(out, format!("배경음 '{}'의 tag(분위기)가 올바르지 않습니다: {:?} (가능: {})", a.base.key, other, BGM_MOODS.join(", "))),
            },
            _ => {}
        }
    }
    for a in &def.assets.characters {
        if a.usage == "standing" {
            match a.emotion.as_deref().map(str::trim) {
                None | Some("") => err(out, format!("스탠딩 '{}'에 감정명(emotion)이 없습니다", a.base.key)),
                Some(e) if bad_chars(e) || e.contains(':') => err(out, format!("스탠딩 '{}'의 감정명에 < > : \" , 줄바꿈은 쓸 수 없습니다", a.base.key)),
                _ => {}
            }
        }
        if a.game_character_id.is_none() {
            warn(out, format!("캐릭터 이미지 '{}'에 캐릭터가 없어 쓰이지 않습니다 (캐릭터가 지워졌을 수 있습니다)", a.base.key));
        }
    }

    // 대본의 `이름:`이 모호하지 않게: 이름이 겹치거나 구분자를 포함하면 안 된다.
    let mut seen: HashSet<String> = HashSet::new();
    for c in def.characters.values() {
        if c.name.contains([':', '：', '<', '>']) {
            err(out, format!("캐릭터 이름 '{}'에 : < > 는 쓸 수 없습니다 (대본의 이름: 접두와 연출 태그에 쓰입니다)", c.name));
        }
        if c.name.chars().count() > 20 {
            warn(out, format!("캐릭터 이름 '{}'이 20자를 넘어 대본에서 화자로 인식되지 않습니다", c.name));
        }
        if !seen.insert(c.name.trim().to_lowercase()) {
            warn(out, format!("캐릭터 이름 '{}'이 겹쳐서 대본의 이름: 접두가 모호합니다", c.name));
        }
    }

    // 씬 대본에서 쓸 수 없는 태그(키 없음 등)
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    for s in &def.scenes {
        if let Some(script) = s.script.as_deref().filter(|t| !t.trim().is_empty()) {
            let mut r = crate::game::runtime::DefResolver { def, rng: &mut rng, ai: false, player_name: None };
            for p in stage::lint(script, &mut r, &Allowed::author()) {
                warn(out, format!("씬 '{}' 대본: 쓸 수 없는 연출 태그 {p} (키/감정/효과 이름을 확인하세요)", s.scene_name));
            }
        }
    }
}

/// 에셋 파일이 게임 폴더에 실제로 있는지 (파일시스템이 필요해서 validate와 따로 둔다).
pub fn validate_asset_files(def: &GameDefinition, game_dir: &std::path::Path) -> Vec<Issue> {
    let mut out = Vec::new();
    for a in &def.assets.all {
        if crate::game::package::check_asset_path(&a.file_path).is_err() {
            err(&mut out, format!("에셋 '{}'의 경로가 올바르지 않습니다: {}", a.key, a.file_path));
            continue;
        }
        let path = game_dir.join(&a.file_path);
        if !path.is_file() {
            err(&mut out, format!("에셋 '{}'의 파일이 없습니다: {}", a.key, a.file_path));
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !crate::game::protocol::is_servable(ext) {
            warn(&mut out, format!("에셋 '{}'의 확장자 .{ext}는 지원하지 않아 재생/표시되지 않습니다", a.key));
        }
    }
    out
}

/// Error만 모아 한 문장으로. start_new_game이 시작을 막을 때 쓴다.
pub fn errors_summary(issues: &[Issue]) -> Option<String> {
    let errors: Vec<&str> = issues.iter().filter(|i| i.severity == Severity::Error).map(|i| i.message.as_str()).collect();
    if errors.is_empty() { None } else { Some(errors.join("; ")) }
}
