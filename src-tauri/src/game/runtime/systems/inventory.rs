//! 인벤토리의 유일한 writer (state.inventory_json). 규칙은 전부 여기서 적용한다:
//!   - max_stack: 넘치는 만큼은 버려진다. 알림(ItemAdded)에는 "실제로 들어온 수량"이 실린다.
//!   - is_unique: 이미 가지고 있으면 더 못 받는다 (알림도 안 나감).
//!   - 처음 들어온 순간(0 → 1 이상) on_acquire_event_id가 있으면 EventRequested로 발동 (manual 이벤트).
//!   - 소모성(consumable) 아이템은 사용하면 하나 줄고, use_effect는 평소 효과 경로로 흘러간다.
//! 이 시스템은 요청(ItemChangeRequested/ItemUsed)에만 반응한다 — 효과/커맨드는 상태를 직접 안 쓴다.
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::{effect_events, GameSystem};
use rand::rngs::StdRng;

pub struct InventorySystem;

fn qty(state: &GameState, item_id: &str) -> i64 {
    state.inventory_json.get(item_id).and_then(|v| v.as_i64()).unwrap_or(0)
}

fn set_qty(state: &mut GameState, item_id: &str, n: i64) {
    if !state.inventory_json.is_object() {
        state.inventory_json = serde_json::json!({});
    }
    if let Some(obj) = state.inventory_json.as_object_mut() {
        if n <= 0 {
            obj.remove(item_id);
        } else {
            obj.insert(item_id.to_string(), serde_json::Value::from(n));
        }
    }
}

fn change(def: &GameDefinition, state: &mut GameState, item_id: &str, delta: i64) -> Vec<GameEvent> {
    let Some(item) = def.items.get(item_id) else {
        return vec![]; // 정의에 없는 아이템 — 조용히 무시 (validate가 미리 잡는다)
    };
    let owned = qty(state, item_id);
    if delta > 0 {
        let cap = if item.is_unique { 1 } else { item.max_stack as i64 };
        let actual = (cap - owned).max(0).min(delta);
        if actual == 0 {
            return vec![];
        }
        set_qty(state, item_id, owned + actual);
        let mut out = vec![GameEvent::ItemAdded { item_id: item_id.to_string(), quantity: actual }];
        if owned == 0 && !item.on_acquire_event_id.trim().is_empty() {
            out.push(GameEvent::EventRequested { event_id: item.on_acquire_event_id.clone() });
        }
        out
    } else if delta < 0 {
        let actual = (-delta).min(owned);
        if actual == 0 {
            return vec![];
        }
        set_qty(state, item_id, owned - actual);
        vec![GameEvent::ItemRemoved { item_id: item_id.to_string(), quantity: actual }]
    } else {
        vec![]
    }
}

impl GameSystem for InventorySystem {
    fn name(&self) -> &'static str {
        "inventory"
    }

    fn handle_event(&self, def: &GameDefinition, state: &mut GameState, event: &GameEvent, _rng: &mut StdRng) -> Vec<GameEvent> {
        match event {
            GameEvent::ItemChangeRequested { item_id, delta } => change(def, state, item_id, *delta),
            GameEvent::ItemUsed { item_id, .. } => {
                let Some(item) = def.items.get(item_id) else { return vec![] };
                let mut out = Vec::new();
                if item.consumable {
                    out.extend(change(def, state, item_id, -1));
                }
                out.extend(effect_events(&state.flags, &item.use_effect));
                out
            }
            _ => vec![],
        }
    }
}
