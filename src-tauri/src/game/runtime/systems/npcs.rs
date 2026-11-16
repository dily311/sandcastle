//! NPC가 지금 어느 위치에 있는지를 정한다. 스폰 규칙(game_character_spawn_locations)이 정의된 캐릭터만 다룬다.
//!
//! 다시 굴리는 시점은 두 번뿐이다: 게임 시작(GameStarted)과 시간대가 바뀔 때(TimePeriodChanged).
//! 매 이동마다 굴리면 같은 방에 들어갔다 나올 때마다 NPC가 바뀌기 때문이다.
//! 굴림: 캐릭터마다 조건(condition_json)을 통과한 위치만 후보로 두고 spawn_weight대로 하나를 고른다.
//! 후보가 하나도 없으면 어디에도 없는 상태(absent)가 된다.
//! 위치가 바뀐 NPC마다 NpcMoved를 내고, journal이 그걸 game_character_state_snapshots에 쌓는다.
//! 플레이어가 대화 중인 NPC가 다른 곳으로 가버리면 대화를 닫는다 (DialogueEnded).
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::{GameState, NpcState};
use super::events::engine_state;
use super::GameSystem;
use crate::game::engine::{self, Condition};
use rand::rngs::StdRng;

pub struct NpcSystem;

fn reroll_all(def: &GameDefinition, state: &mut GameState, rng: &mut StdRng) -> Vec<GameEvent> {
    let es = engine_state(state, state.game_time);
    // 캐릭터 순서를 고정해서 같은 시드면 같은 결과가 나오게 한다 (HashMap 순회 순서는 매번 다르다).
    let mut ids: Vec<&String> = def.spawn_locations.keys().collect();
    ids.sort();

    let mut out = Vec::new();
    for cid in ids {
        if def.character(cid).is_none() {
            continue;
        }
        let candidates: Vec<(String, u32)> = def.spawn_locations[cid]
            .iter()
            .filter(|row| {
                // 조건이 적혀 있는데 못 읽으면 후보에서 뺀다 (fail-closed). null/빈 객체는 조건 없음.
                let c = &row.condition_json;
                if c.is_null() || c.as_object().map_or(false, |o| o.is_empty()) {
                    return true;
                }
                serde_json::from_value::<Condition>(c.clone()).unwrap_or(Condition::Never).eval(&es)
            })
            .filter(|row| def.location(&row.location_id).is_some())
            .map(|row| (row.location_id.clone(), row.spawn_weight.max(0) as u32))
            .collect();
        let new_loc = engine::pick_weighted_location(&candidates, rng);

        let known = state.npc_states.get(cid);
        let old_loc = known.and_then(|n| n.location_id.clone());
        if known.is_some() && old_loc == new_loc {
            continue; // 그대로면 알리지도 기록하지도 않는다
        }
        state.npc_states.insert(
            cid.clone(),
            NpcState {
                location_id: new_loc.clone(),
                status: if new_loc.is_some() { "active" } else { "absent" }.into(),
            },
        );
        out.push(GameEvent::NpcMoved {
            game_character_id: cid.clone(),
            from: old_loc,
            to: new_loc.clone(),
        });

        // 대화 중이던 NPC가 이 자리를 떠났으면 대화를 닫는다.
        if state.active_npc_id.as_deref() == Some(cid.as_str()) && new_loc != state.location_id {
            out.push(GameEvent::DialogueEnded {
                game_character_id: cid.clone(),
            });
        }
    }
    out
}

impl GameSystem for NpcSystem {
    fn name(&self) -> &'static str {
        "npcs"
    }

    fn handle_event(&self, def: &GameDefinition, state: &mut GameState, event: &GameEvent, rng: &mut StdRng) -> Vec<GameEvent> {
        match event {
            GameEvent::GameStarted | GameEvent::TimePeriodChanged { .. } => reroll_all(def, state, rng),
            GameEvent::CharactersAppeared { character_ids } => {
                let Some(here) = state.location_id.clone() else { return vec![] };
                let mut out = Vec::new();
                for id in character_ids {
                    if def.character(id).is_none() {
                        continue;
                    }
                    let old = state.npc_states.get(id).and_then(|n| n.location_id.clone());
                    if old.as_deref() != Some(here.as_str()) {
                        state.npc_states.insert(
                            id.clone(),
                            NpcState {
                                location_id: Some(here.clone()),
                                status: "active".into(),
                            },
                        );
                        out.push(GameEvent::NpcMoved {
                            game_character_id: id.clone(),
                            from: old,
                            to: Some(here.clone()),
                        });
                    }
                }
                out
            }
            _ => vec![],
        }
    }
}
