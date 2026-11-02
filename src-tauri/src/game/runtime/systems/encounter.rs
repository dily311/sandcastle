//! TimeAdvanced에 반응해서 인카운터를 굴린다. 이동이든 기다리기(AdvanceTime)든
//! 시간이 흐르면 똑같이 굴러간다. 어느 zone 기준인지는 이벤트가 아니라 그 시점의
//! state.zone_id를 읽는다 — 이동으로 시간이 흐른 경우, MovementSystem이 이전 라운드에서
//! 이미 갱신해뒀다 (TimeAdvanced는 한 라운드 뒤에야 존재하므로 순서가 자연히 보장됨).
//!
//! 효과(effect_json)는 여기서 state.flags에 직접 쓰지 않는다 — VariableChanged로 알리기만
//! 하고, 실제 쓰기는 trigger.rs의 VariableChanged 핸들러 한 곳이 한다 (단일 writer).
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::GameSystem;
use crate::game::engine::{self, Entry};
use rand::rngs::StdRng;

pub struct EncounterSystem;

impl GameSystem for EncounterSystem {
    fn name(&self) -> &'static str {
        "encounter"
    }

    fn handle_event(&self, def: &GameDefinition, state: &mut GameState, event: &GameEvent, rng: &mut StdRng) -> Vec<GameEvent> {
        let GameEvent::TimeAdvanced { .. } = event else {
            return vec![];
        };
        let Some(zone_id) = state.zone_id.clone() else {
            return vec![];
        };

        let Some(weather_mod) = def.weather_types.iter().find(|w| w.id == state.weather_id).map(|w| w.encounter_rate_modifier) else {
            return vec![];
        };

        let table = def
            .encounter_tables_for_map(&state.map_id)
            .find(|t| t.zone_id.as_deref() == Some(zone_id.as_str()))
            .or_else(|| def.encounter_tables_for_map(&state.map_id).find(|t| t.zone_id.is_none()));
        let Some(table) = table else {
            return vec![];
        };

        let entries: Vec<Entry> = def
            .entries_for_table(&table.id)
            .map(|row| Entry {
                id: row.id.clone(),
                event_id: row.event_id.clone(),
                weight: row.weight.max(0) as u32,
                time_period_id: row.time_period_id.clone(),
                weather_id: row.weather_id.clone(),
                conditions: def.event_conditions.get(&row.event_id).cloned().unwrap_or_default(),
            })
            .collect();

        let cooling_down: Vec<String> = state
            .cooldowns
            .iter()
            .filter(|(event_id, triggered_at)| def.events.get(*event_id).map_or(false, |e| state.game_time - **triggered_at < e.cooldown_seconds))
            .map(|(id, _)| id.clone())
            .collect();

        let engine_state = engine::EngineState {
            map_id: state.map_id.clone(),
            zone_id: Some(zone_id),
            game_time: state.game_time,
            weather_id: state.weather_id.clone(),
            time_period_id: state.time_period_id.clone(),
            flags: state.flags.clone(),
            location_id: state.location_id.clone(),
            time_from: state.game_time.clone(),
            inventory: state.inventory_json.clone(),
            relationships: state.relationships_json.clone(),
        };

        let Some(idx) = engine::roll_encounter(&entries, &engine_state, table.base_trigger_chance, weather_mod, &cooling_down, rng) else {
            return vec![];
        };
        let Some(game_event) = def.events.get(&entries[idx].event_id) else {
            return vec![]; // 데이터 정합성 문제 — 조용히 무시 (TODO: 로깅)
        };

        let npc_id = if game_event.requires_npc {
            let pool = def.event_npc_pool.get(&game_event.id).cloned().unwrap_or_default();
            let weights: Vec<u32> = pool.iter().map(|p| p.spawn_weight.max(0) as u32).collect();
            engine::weighted_index(&weights, rng).map(|i| pool[i].game_character_id.clone())
        } else {
            None
        };

        state.cooldowns.insert(game_event.id.clone(), state.game_time);

        let mut out = vec![GameEvent::EventTriggered {event_id:game_event.id.clone(),npc_id, event_type: game_event.event_type.clone(), trigger_type: game_event.trigger_type.clone() }];
        let (_, changes) = engine::apply_effect(&state.flags, &game_event.effect_json);
        out.extend(changes.into_iter().map(|(key, old, new)| GameEvent::VariableChanged { key, old, new }));
        out
    }
}
