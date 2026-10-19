//! LocationChanged에 반응해서 state.location_id를 갱신하고, 그 location이 속한 zone/map을
//! 거슬러 찾아 state.zone_id/map_id(캐시값)도 같이 갱신한다. zone이 바뀌면 ZoneChanged를 추가로 낸다.
//! (같은 zone 안의 방 사이 이동이면 ZoneChanged는 안 나간다.)
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::GameSystem;
use rand::rngs::StdRng;

pub struct MovementSystem;

impl GameSystem for MovementSystem {
    fn name(&self) -> &'static str {
        "movement"
    }

    fn handle_event(&self, def: &GameDefinition, state: &mut GameState, event: &GameEvent, _rng: &mut StdRng) -> Vec<GameEvent> {
        let GameEvent::LocationChanged { to, .. } = event else {
            return vec![];
        };
        state.location_id = Some(to.clone());

        let mut out = vec![];
        if let Some(zone) = def.zone_of_location(to) {
            if state.zone_id.as_deref() != Some(zone.id.as_str()) {
                out.push(GameEvent::ZoneChanged { from: state.zone_id.clone(), to: zone.id.clone() });
                state.zone_id = Some(zone.id.clone());
            }
            state.map_id = zone.map_id.clone();
        }
        out
    }
}
