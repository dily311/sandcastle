//! TODO: 아직 설계 전. 맵의 danger_level은 인카운터 확률 배율로 events.rs의 roll_random에서 이미
//!   쓰인다 (GameDefinition::danger_multiplier). 그 밖의 맵/존 단위 효과가 필요해지면 여기서.
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::GameSystem;
use rand::rngs::StdRng;

pub struct MapSystem;

impl GameSystem for MapSystem {
    fn name(&self) -> &'static str {
        "map"
    }

    fn handle_event(&self, _def: &GameDefinition, _state: &mut GameState, _event: &GameEvent, _rng: &mut StdRng) -> Vec<GameEvent> {
        vec![] // TODO
    }
}
