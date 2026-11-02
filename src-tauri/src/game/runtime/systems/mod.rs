//! 시스템 7개(time/map/trigger/encounter/relationship/scene/inventory)의 공통 인터페이스와
//! 실행 순서. process_events가 씨앗 이벤트 하나당 이 순서대로 전부 통과시킨다.
mod dialogue;
mod events;
mod inventory;
mod map;
mod movement;
mod npcs;
mod relationship;
mod scene;
mod stage;
mod time;
mod trigger;

use super::definition::GameDefinition;
use super::event::GameEvent;
use super::state::GameState;
use rand::rngs::StdRng;

pub use events::prime_auto_state;

/// 효과 JSON({"set":{}, "add":{}, "enter_scene": "<scene_id>", "fire_event": "<event_id>"})을
/// 이벤트들로 환원한다. state에는 쓰지 않는다 — 쓰기는 trigger.rs(변수)/scene.rs(씬)/events.rs(이벤트)가 한다.
pub(super) fn effect_events(flags: &serde_json::Value, effect: &serde_json::Value) -> Vec<GameEvent> {
    let (_, changes) = crate::game::engine::apply_effect(flags, effect);
    let mut out: Vec<GameEvent> = changes.into_iter().map(|(key, old, new)| GameEvent::VariableChanged { key, old, new }).collect();
    // 아이템: {"give_item": {"key_card": 1}, "take_item": {"potion": 2}}  (item_id -> 수량)
    for (verb, sign) in [("give_item", 1i64), ("take_item", -1i64)] {
        if let Some(serde_json::Value::Object(m)) = effect.get(verb) {
            for (item_id, q) in m {
                if let Some(q) = q.as_i64().filter(|q| *q > 0) {
                    out.push(GameEvent::ItemChangeRequested { item_id: item_id.clone(), delta: sign * q });
                }
            }
        }
    }
    // 관계: {"relation": {"player>yuna": {"affection": 3, "trust": -2}}}  (키는 "from>to")
    if let Some(serde_json::Value::Object(pairs)) = effect.get("relation") {
        for (key, axes) in pairs {
            let (Some((from, to)), Some(axes)) = (key.split_once('>'), axes.as_object()) else { continue };
            for (axis, delta) in axes {
                if let Some(delta) = delta.as_i64().filter(|d| *d != 0) {
                    out.push(GameEvent::RelationChangeRequested { from: from.to_string(), to: to.to_string(), axis: axis.clone(), delta });
                }
            }
        }
    }
    // 연출: {"stage": {"bgm": "romance", "cg": "festival_01", "effects": ["fireworks"], "emotion": {"유나": "happy"}}}
    if let Some(spec) = effect.get("stage").filter(|s| s.is_object()) {
        out.push(GameEvent::StageRequested { spec: spec.clone() });
    }
    if let Some(scene_id) = effect.get("enter_scene").and_then(|v| v.as_str()) {
        out.push(GameEvent::SceneRequested { scene_id: scene_id.to_string() });
    }
    if let Some(event_id) = effect.get("fire_event").and_then(|v| v.as_str()) {
        out.push(GameEvent::EventRequested { event_id: event_id.to_string() });
    }
    out
}

pub trait GameSystem {
    fn name(&self) -> &'static str;

    /// def는 읽기 전용(정적 데이터), state는 이 시스템이 자기 몫만 고쳐야 한다
    /// (다른 시스템의 필드를 건드리지 않기 — 안 그러면 실행 순서에 결과가 좌우됨).
    fn handle_event(
        &self,
        def: &GameDefinition,
        state: &mut GameState,
        event: &GameEvent,
        rng: &mut StdRng,
    ) -> Vec<GameEvent>;
}

/// 실행 순서. 뒤 시스템이 앞 시스템의 결과(이번 턴에 새로 emit된 이벤트)에 반응하지
/// 않는다는 걸 기억할 것 — 전부 "이번 턴의 원래 씨앗 이벤트들"만 본다 (1단계 팬아웃).
pub fn all() -> Vec<Box<dyn GameSystem>> {
    vec![
        Box::new(movement::MovementSystem),
        Box::new(time::TimeSystem),
        Box::new(map::MapSystem),
        Box::new(trigger::TriggerSystem),
        Box::new(npcs::NpcSystem),
        Box::new(events::EventSystem),
        Box::new(relationship::RelationshipSystem),
        Box::new(dialogue::DialogueSystem),
        Box::new(scene::SceneSystem),
        Box::new(stage::StageSystem),
        Box::new(inventory::InventorySystem),
    ]
}
