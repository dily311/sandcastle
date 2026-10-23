//! 이벤트 결정기. game_events를 trigger_type별로 후보로 모아 한곳에서 순서/정책/효과를 처리한다.
//! (예전의 EncounterSystem + trigger.rs의 TriggerActivated 처리를 합친 것.)
//!
//! "어떤 순간인가"는 이벤트가 알려준다:
//!   TriggerActivated(Interact)   → interaction  (눌린 오브젝트에 연결된 이벤트 하나)
//!   LocationChanged              → location
//!   TimeAdvanced{from,to}        → scheduled → auto → random   (같은 순간 안의 실행 순서)
//!   VariableChanged              → auto
//!   EventRequested(fire_event)   → manual
//! 라운드 순서상 location은 이동 직후(1라운드), 시간 기반은 TimeAdvanced가 나오는 2라운드에서 돈다.
//! 그래서 interaction > location > scheduled > auto > random 순서가 자연히 지켜진다.
//!
//! random은 같은 dispatch에서 다른 타입이 하나라도 발동했으면 안 굴린다(state.turn_fired).
//! 같은 타입 안에서는 config_json의 priority(클수록 먼저) → id 순으로 정렬해서 실행마다
//! 순서가 달라지지 않게 한다 (def.events가 HashMap이라 정렬이 없으면 순회 순서가 매번 다르다).
//!
//! 효과는 여기서 state에 쓰지 않는다 — VariableChanged / SceneRequested / EventRequested로
//! 환원해서 알리기만 하고, 쓰기는 trigger.rs(변수)와 scene.rs(씬)가 한다 (단일 writer).
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::{effect_events, GameSystem};
use crate::game::engine::{self, EngineState, Entry};
use crate::models::game::GameEvents;
use rand::rngs::StdRng;

pub struct EventSystem;

fn priority(e: &GameEvents) -> i64 {
    e.config_json.get("priority").and_then(|v| v.as_i64()).unwrap_or(0)
}

pub(super) fn engine_state(state: &GameState, time_from: i64) -> EngineState {
    EngineState {
        map_id: state.map_id.clone(),
        zone_id: state.zone_id.clone(),
        location_id: state.location_id.clone(),
        game_time: state.game_time,
        time_from,
        weather_id: state.weather_id.clone(),
        time_period_id: state.time_period_id.clone(),
        flags: state.flags.clone(),
        inventory: state.inventory_json.clone(),
        relationships: state.relationships_json.clone(),
    }
}

fn conditions_pass(def: &GameDefinition, event_id: &str, es: &EngineState) -> bool {
    def.event_conditions.get(event_id).map_or(true, |cs| cs.iter().all(|c| c.eval(es)))
}

/// repeat_policy/쿨다운 확인. cooldowns에는 이 세션에서 한 번이라도 발동한 이벤트가 전부 들어 있다
/// (load_cooldowns가 이벤트 로그에서 채운다) — 그래서 once는 "키가 있는가"만 보면 된다.
fn can_fire(state: &GameState, e: &GameEvents) -> bool {
    match e.repeat_policy.as_str() {
        "once" => !state.cooldowns.contains_key(&e.id),
        _ => state.cooldowns.get(&e.id).map_or(true, |t| state.game_time - *t >= e.cooldown_seconds),
    }
}

fn fire(def: &GameDefinition, state: &mut GameState, e: &GameEvents, rng: &mut StdRng) -> Vec<GameEvent> {
    let npc_id = if e.requires_npc {
        let pool = def.event_npc_pool.get(&e.id).cloned().unwrap_or_default();
        let weights: Vec<u32> = pool.iter().map(|p| p.spawn_weight.max(0) as u32).collect();
        engine::weighted_index(&weights, rng).map(|i| pool[i].game_character_id.clone())
    } else {
        None
    };
    state.cooldowns.insert(e.id.clone(), state.game_time);
    if e.trigger_type != "random" {
        state.turn_fired = true;
    }
    let mut out = vec![GameEvent::EventTriggered {
        event_id: e.id.clone(),
        event_type: e.event_type.clone(),
        trigger_type: e.trigger_type.clone(),
        npc_id: npc_id.clone(),
    }];
    // 이벤트로 등장한 NPC는 플레이어가 있는 위치에 나타난 것으로 기록한다 (Talk의 "같은 위치" 검사와 맞추려는 것).
    if let Some(npc) = &npc_id {
        let prev = state.npc_states.get(npc).and_then(|n| n.location_id.clone());
        if prev != state.location_id {
            state.npc_states.insert(npc.clone(), super::super::state::NpcState { location_id: state.location_id.clone(), status: "active".into() });
            out.push(GameEvent::NpcMoved { game_character_id: npc.clone(), from: prev, to: state.location_id.clone() });
        }
    }
    out.extend(effect_events(&state.flags, &e.effect_json));
    out
}

/// trigger_type 하나의 후보들을 정렬해서 차례로 검사/발동한다.
/// edge=true(auto)면 조건이 "거짓→참"으로 바뀐 순간에만 발동한다 — 참인 동안 매 턴 터지지 않게.
fn run_type(def: &GameDefinition, state: &mut GameState, trigger_type: &str, es: &EngineState, rng: &mut StdRng, edge: bool) -> Vec<GameEvent> {
    let mut cands: Vec<&GameEvents> = def.events.values().filter(|e| e.trigger_type == trigger_type).collect();
    cands.sort_by(|a, b| priority(b).cmp(&priority(a)).then_with(|| a.id.cmp(&b.id)));

    let mut out = vec![];
    for e in cands {
        let pass = conditions_pass(def, &e.id, es);
        if edge {
            if !pass {
                state.auto_active.remove(&e.id);
                continue;
            }
            if !state.auto_active.insert(e.id.clone()) {
                continue; // 이미 참인 상태 — 상승 순간이 아니다
            }
        } else if !pass {
            continue;
        }
        if can_fire(state, e) {
            out.extend(fire(def, state, e, rng));
        }
    }
    out
}

fn roll_random(def: &GameDefinition, state: &mut GameState, es: &EngineState, rng: &mut StdRng) -> Vec<GameEvent> {
    if state.turn_fired {
        return vec![];
    }
    let Some(zone_id) = state.zone_id.clone() else { return vec![] };
    let Some(weather_mod) = def.weather_types.iter().find(|w| w.id == state.weather_id).map(|w| w.encounter_rate_modifier) else {
        return vec![];
    };
    let table = def
        .encounter_tables_for_map(&state.map_id)
        .find(|t| t.zone_id.as_deref() == Some(zone_id.as_str()))
        .or_else(|| def.encounter_tables_for_map(&state.map_id).find(|t| t.zone_id.is_none()));
    let Some(table) = table else { return vec![] };

    // random 이벤트만 후보. 쿨다운/once로 못 나가는 이벤트는 cooling_down 목록으로 넘겨서 제외시킨다.
    let entries: Vec<Entry> = def
        .entries_for_table(&table.id)
        .filter(|row| def.events.get(&row.event_id).map_or(false, |e| e.trigger_type == "random"))
        .map(|row| Entry {
            id: row.id.clone(),
            event_id: row.event_id.clone(),
            weight: row.weight.max(0) as u32,
            time_period_id: row.time_period_id.clone(),
            weather_id: row.weather_id.clone(),
            conditions: def.event_conditions.get(&row.event_id).cloned().unwrap_or_default(),
        })
        .collect();
    let cooling_down: Vec<String> = entries
        .iter()
        .filter(|en| def.events.get(&en.event_id).map_or(true, |e| !can_fire(state, e)))
        .map(|en| en.event_id.clone())
        .collect();

    let Some(idx) = engine::roll_encounter(&entries, es, table.base_trigger_chance * def.danger_multiplier(&state.map_id), weather_mod, &cooling_down, rng) else {
        return vec![];
    };
    let Some(e) = def.events.get(&entries[idx].event_id) else { return vec![] };
    fire(def, state, e, rng)
}

/// 불러온 직후, 지금 조건이 이미 참인 auto 이벤트들을 "참인 상태"로 표시해둔다(발동은 안 함).
/// 안 그러면 이어하기 직후 repeatable auto 이벤트가 상승 순간으로 오인돼서 다시 터진다.
pub fn prime_auto_state(def: &GameDefinition, state: &mut GameState) {
    let es = engine_state(state, state.game_time);
    for e in def.events.values().filter(|e| e.trigger_type == "auto") {
        if conditions_pass(def, &e.id, &es) {
            state.auto_active.insert(e.id.clone());
        }
    }
}

impl GameSystem for EventSystem {
    fn name(&self) -> &'static str {
        "events"
    }

    fn handle_event(&self, def: &GameDefinition, state: &mut GameState, event: &GameEvent, rng: &mut StdRng) -> Vec<GameEvent> {
        match event {
            GameEvent::TriggerActivated { trigger_id } => {
                let Some(event_id) = def.interactable(trigger_id).and_then(|i| i.event_id.clone()) else {
                    return vec![]; // 연결된 이벤트가 없는 오브젝트 (TriggerActivated 알림만 나가고 끝)
                };
                let Some(e) = def.events.get(&event_id) else { return vec![] };
                let es = engine_state(state, state.game_time);
                if e.trigger_type == "interaction" && conditions_pass(def, &e.id, &es) && can_fire(state, e) {
                    fire(def, state, e, rng)
                } else {
                    vec![]
                }
            }
            GameEvent::LocationChanged { .. } => {
                let es = engine_state(state, state.game_time);
                run_type(def, state, "location", &es, rng, false)
            }
            GameEvent::TimeAdvanced { from, .. } => {
                let es = engine_state(state, *from);
                let mut out = run_type(def, state, "scheduled", &es, rng, false);
                out.extend(run_type(def, state, "auto", &es, rng, true));
                out.extend(roll_random(def, state, &es, rng));
                out
            }
            GameEvent::VariableChanged { .. } => {
                // trigger.rs가 같은 라운드에서 먼저 flags를 반영한다 (systems::all() 순서).
                let es = engine_state(state, state.game_time);
                run_type(def, state, "auto", &es, rng, true)
            }
            GameEvent::EventRequested { event_id } => {
                let Some(e) = def.events.get(event_id) else { return vec![] };
                let es = engine_state(state, state.game_time);
                if e.trigger_type == "manual" && conditions_pass(def, &e.id, &es) && can_fire(state, e) {
                    fire(def, state, e, rng)
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}