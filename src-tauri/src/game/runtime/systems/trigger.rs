//! 변수 쓰기의 단일 writer.
//!   1) VariableChanged: state.flags에 실제로 쓰는 유일한 곳. SetVariable 커맨드, 선택지 effect,
//!      이벤트 effect_json 전부 결국 이 이벤트로 환원돼서 여기서 쓰인다 (멱등).
//!   2) ChoiceMade: 선택지의 effect(raw JSON)를 VariableChanged/SceneRequested 등으로 환원해서 emit.
//! (오브젝트 상호작용으로 이벤트를 발동하는 건 events.rs가 한다.)
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::{effect_events, GameSystem};
use rand::rngs::StdRng;

pub struct TriggerSystem;

impl GameSystem for TriggerSystem {
    fn name(&self) -> &'static str {
        "trigger"
    }

    fn handle_event(&self, _def: &GameDefinition, state: &mut GameState, event: &GameEvent, _rng: &mut StdRng) -> Vec<GameEvent> {
        match event {
            GameEvent::VariableChanged { key, new, .. } => {
                if !state.flags.is_object() {
                    state.flags = serde_json::json!({});
                }
                if let Some(obj) = state.flags.as_object_mut() {
                    obj.insert(key.clone(), new.clone());
                }
                vec![]
            }
            GameEvent::ChoiceMade { effect: Some(effect), .. } => effect_events(&state.flags, effect),
            _ => vec![],
        }
    }
}
