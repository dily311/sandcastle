//! 씬을 여닫는 유일한 곳(current_scene_id / pending_scenes_json의 단일 writer).
//!   - ChoiceMade: next_scene_id가 Some이면 그 씬으로, None이면 씬 종료(자유 이동 복귀).
//!   - SceneRequested(이벤트 효과 enter_scene): 규칙대로 열거나 대기열에 넣는다.
//!
//! 규칙:
//!   1. 씬은 한 번에 하나만 열려 있다. 열려 있는 동안 요청된 씬은 대기열(pending_scenes_json)에서 기다리다가
//!      현재 씬이 끝나면 앞에서부터 하나씩 열린다.
//!   2. AI NPC와 대화 중에 씬이 열리면 대화를 먼저 닫는다 (DialogueEnded).
//!   3. 엔딩 씬이 열리면 GameEnded.
//!   (씬이 열린 동안 MoveTo 차단은 GameRuntime::handle_command가 한다.)
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::GameSystem;
use rand::rngs::StdRng;

pub struct SceneSystem;

fn open_scene(def: &GameDefinition, state: &mut GameState, scene_id: &str) -> Vec<GameEvent> {
    let Some(scene) = def.scene(scene_id) else {
        return vec![]; // 데이터 정합성 문제 — 조용히 무시 (TODO: 로깅)
    };
    let mut out = vec![];
    // 대화 중이면 먼저 닫는다 (DialogueSystem이 다음 라운드에 active_npc_id를 비운다).
    if let Some(npc) = state.active_npc_id.clone() {
        out.push(GameEvent::DialogueEnded { game_character_id: npc });
    }
    state.current_scene_id = Some(scene.id.clone());
    out.push(GameEvent::SceneChanged { scene_id: scene.id.clone() });
    if scene.is_ending == Some(true) {
        if let Some(ending_id) = &scene.ending_id {
            out.push(GameEvent::GameEnded { ending_id: ending_id.clone() });
        }
    }
    out
}

fn pending(state: &GameState) -> Vec<String> {
    state.pending_scenes_json.as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

fn set_pending(state: &mut GameState, ids: Vec<String>) {
    state.pending_scenes_json = serde_json::json!(ids);
}

impl GameSystem for SceneSystem {
    fn name(&self) -> &'static str {
        "scene"
    }

    fn handle_event(&self, def: &GameDefinition, state: &mut GameState, event: &GameEvent, _rng: &mut StdRng) -> Vec<GameEvent> {
        match event {
            GameEvent::ChoiceMade { next_scene_id: Some(id), .. } => open_scene(def, state, id),
            GameEvent::ChoiceMade { next_scene_id: None, .. } => {
                let Some(prev) = state.current_scene_id.take() else { return vec![] };
                let mut out = vec![GameEvent::SceneExited { scene_id: prev }];
                // 대기 중인 씬이 있으면 앞에서부터 하나 연다.
                let mut queue = pending(state);
                if !queue.is_empty() {
                    let next = queue.remove(0);
                    set_pending(state, queue);
                    out.extend(open_scene(def, state, &next));
                }
                out
            }
            GameEvent::SceneRequested { scene_id } => {
                if state.current_scene_id.is_some() {
                    let mut queue = pending(state);
                    if !queue.contains(scene_id) && state.current_scene_id.as_deref() != Some(scene_id.as_str()) {
                        queue.push(scene_id.clone());
                        set_pending(state, queue);
                    }
                    vec![]
                } else {
                    open_scene(def, state, scene_id)
                }
            }
            _ => vec![],
        }
    }
}
