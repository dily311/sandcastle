//! 관계의 유일한 writer (state.relationships_json). 키는 "from>to" = "from이 to에게 느끼는 것"이고
//! from/to는 'player' 또는 game_character_id다. 축은 engine::RELATION_AXES(고정), 범위는 -100..=100.
//! RelationChangeRequested(효과 relation, AI의 <updateRel>)에만 반응하고, 범위로 자른 뒤 실제로 바뀐 만큼만
//! RelationChanged로 알린다 (journal이 그 변화량을 game_relationship_snapshots에 쌓는다).
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::GameSystem;
use crate::game::engine::{relation_value, RELATION_AXES, RELATION_MAX, RELATION_MIN};
use rand::rngs::StdRng;

pub struct RelationshipSystem;

fn valid_end(def: &GameDefinition, id: &str) -> bool {
    id == "player" || def.character(id).is_some()
}

impl GameSystem for RelationshipSystem {
    fn name(&self) -> &'static str {
        "relationship"
    }

    fn handle_event(&self, def: &GameDefinition, state: &mut GameState, event: &GameEvent, _rng: &mut StdRng) -> Vec<GameEvent> {
        let GameEvent::RelationChangeRequested { from, to, axis, delta } = event else {
            return vec![];
        };
        // 정의에 없는 캐릭터, 자기 자신에 대한 관계, 모르는 축은 조용히 무시한다 (validate가 미리 잡는다).
        if from == to || !valid_end(def, from) || !valid_end(def, to) || !RELATION_AXES.contains(&axis.as_str()) {
            return vec![];
        }
        let old = relation_value(&state.relationships_json, from, to, axis);
        let new = (old + delta).clamp(RELATION_MIN, RELATION_MAX);
        if new == old {
            return vec![];
        }
        if !state.relationships_json.is_object() {
            state.relationships_json = serde_json::json!({});
        }
        if let Some(obj) = state.relationships_json.as_object_mut() {
            let entry = obj.entry(format!("{from}>{to}")).or_insert_with(|| serde_json::json!({ "tags": [] }));
            entry[axis.as_str()] = serde_json::Value::from(new);
        }
        vec![GameEvent::RelationChanged { from: from.clone(), to: to.clone(), axis: axis.clone(), old, new }]
    }
}
