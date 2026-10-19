
// pub fn can_move_to(&self, definition: &GameDefinition, state: &GameState, location_id: &str) -> bool {todo!()}
// pub fn move_to(&self, definition: &GameDefinition, state: &GameState, location_id: &str) -> Result<Vec<GameEvent>> {todo!()}
// pub fn available_locations(&self, definition: &GameDefinition, state: &GameState, location_id: &str) -> Vec<&Location> {todo!()}

use rand::rngs::StdRng;
use crate::game::{engine::{self, Entry}, runtime::{definition::GameDefinition, event::GameEvent, state::GameState, systems::GameSystem}};
pub struct MapSystem;
impl GameSystem for MapSystem {
    fn name(&self) -> &'static str {
        "map"
    }

    fn handle_event(&self, _def: &GameDefinition, _state: &mut GameState, _event: &GameEvent, _rng: &mut StdRng) -> Vec<GameEvent> {
        vec![] // TODO
    }
}
