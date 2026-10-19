use rand::rngs::StdRng;
use crate::game::{engine::{self, Entry}, runtime::{definition::GameDefinition, event::GameEvent, state::GameState, systems::GameSystem}};
pub struct InventorySystem;

impl GameSystem for InventorySystem {
    fn name(&self) -> &'static str {
        "inventory"
    }
    fn handle_event(&self, def: &GameDefinition, state: &mut GameState, event: &GameEvent, rng: &mut rand::prelude::StdRng) -> Vec<GameEvent> {
        vec![] // todo!()
    }
}

// pub fn move_to(&self,state: &mut GameState,location: &Location,) -> Result<Vec<GameEvent>, AppError> {todo!()}