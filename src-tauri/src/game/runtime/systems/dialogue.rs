//! AI NPC 대화창 열기/닫기 상태 북마크(DialogueStarted/DialogueEnded)만 다룬다.
//! 실제 AI 호출은 game::npc::say가 따로 하고, 거기서 이 active_npc_id를 확인해서
//! "열린 대화와 일치하는 캐릭터에게만" 응답한다. (씬 이동은 scene.rs, 선택지 효과는 trigger.rs.)
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::GameSystem;
use rand::rngs::StdRng;

pub struct DialogueSystem;

impl GameSystem for DialogueSystem {
    fn name(&self) -> &'static str {
        "dialogue"
    }

    fn handle_event(&self, _def: &GameDefinition, state: &mut GameState, event: &GameEvent, _rng: &mut StdRng) -> Vec<GameEvent> {
        match event {
            GameEvent::DialogueStarted { game_character_id } => {
                state.active_npc_id = Some(game_character_id.clone());
                vec![]
            }
            GameEvent::DialogueEnded { .. } => {
                state.active_npc_id = None;
                vec![]
            }
            _ => vec![],
        }
    }
}
