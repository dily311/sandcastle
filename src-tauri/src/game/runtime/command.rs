//! iframe SDK가 보낼 수 있는 커맨드.
use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum GameCommand {
    MoveTo { location_id: String },
    /// 맵 위 상호작용 대상(game_interactables.id). TriggerSystem이 처리.
    Interact { entity_id: String },
    Talk { game_character_id: String },
    StopTalk,
    /// 프론트가 직접 변수를 바꾼다. 타입 검증(game_state_defs.type 기준)은 지금은 생략 —
    /// 필요해지면 handle_command에서 추가.
    SetVariable { key: String, value: Value },
    /// 분기 노드 대화에서 선택지를 고름. "기록을 다시 볼 때 분기된 씬 불러오기"도
    /// 이 메커니즘 그대로 동작한다 (choice_id가 state.current_scene_id를 바꾸고,
    /// 그게 곧 AI 대화의 branch_key가 되므로).
    Choose { choice_id: String },
    AdvanceTime { minutes: i32 },
    UseItem { item_id: String, target_id: Option<String> },
    /// 버리기. 핵심 아이템(is_key)은 못 버린다. quantity가 없으면 1개.
    DropItem { item_id: String, quantity: Option<u32> },
}