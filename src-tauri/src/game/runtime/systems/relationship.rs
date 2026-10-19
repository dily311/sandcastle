use rand::rngs::StdRng;
use crate::game::{engine::{self, Entry}, runtime::{definition::GameDefinition, event::GameEvent, state::GameState, systems::GameSystem}};
pub struct RelationshipSystem;
impl GameSystem for RelationshipSystem {
    fn name(&self) -> &'static str {
        "relationship"
    }
    fn handle_event(&self, def: &GameDefinition, state: &mut GameState, event: &GameEvent, rng: &mut rand::prelude::StdRng) ->Vec<GameEvent> {
        vec![] // todo!()
    }
}