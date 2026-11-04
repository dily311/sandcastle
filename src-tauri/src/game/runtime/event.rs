//! 시스템들이 주고받는 이벤트. process_events가 "깊이 제한 있는 전파"로 돌린다
//! (runtime/mod.rs의 MAX_EVENT_DEPTH 참고).
//!
//! ⚠ ChoicePresented는 이 enum을 공유하지만 시스템이 아니라 game::npc::say 하나에서만
//!   만들어진다 (AI 응답의 <choice> 태그 파싱 결과). dispatch_command가 아니라
//!   npc_say 응답에 실려 나간다 — 헷갈리기 쉬워서 명시.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum GameEvent {
    GameStarted,
    /// location 단위 이동. 같은 zone 안이든 다른 zone이든 MoveTo가 성공하면 항상 나간다.
    LocationChanged { from: Option<String>, to: String },
    /// 도착 location의 zone이 출발 location의 zone과 다를 때만 LocationChanged와 함께 추가로 나간다.
    ZoneChanged { from: Option<String>, to: String },
    /// handle_command가 내는 내부 요청용 씨앗 이벤트. "시간이 왜 흐르는지"(이동/기다림)와
    /// "시간이 흐르면 뭐가 바뀌는지"(TimeSystem의 계산)를 분리하기 위한 것.
    TimeAdvanceRequested { seconds: i64 },
    /// 시간이 from → to 로 흘렀다 (초). 예약 이벤트가 건너뛴 구간을 판정하려면 from이 필요하다.
    TimeAdvanced { from: i64, to: i64 },
    WeatherChanged { weather_id: String },
    /// 시간대가 바뀌었다 (아침 → 낮 등). NPC 위치를 다시 굴리는 시점이다.
    TimePeriodChanged { from: Option<String>, to: String },
    /// NPC가 한 위치에서 다른 위치로 옮겨졌다 (None = 어디에도 없음). 스폰 재굴림/이벤트 등장이 낸다.
    NpcMoved { game_character_id: String, from: Option<String>, to: Option<String> },
    /// game_events 한 건이 발동됨 (어떤 trigger_type이든 공통). 프론트는 event_type(제작자가 정한
    /// 내용 분류)을 보고 어떤 UI로 보여줄지 고르고, trigger_type은 "어떻게 발동했는가"를 알려준다.
    EventTriggered { event_id: String, event_type: String, trigger_type: String, npc_id: Option<String> },
    /// 이벤트 효과의 enter_scene이 내는 요청. 실제로 열지/대기열에 넣을지는 scene.rs가 정한다.
    SceneRequested { scene_id: String },
    /// 이벤트 효과의 fire_event가 내는 요청. trigger_type이 manual인 이벤트만 이걸로 발동된다.
    EventRequested { event_id: String },
    /// 맵 위 상호작용 대상(game_interactables)이 눌림. 연결된 이벤트가 있으면
    /// EventTriggered가 뒤따른다.
    TriggerActivated { trigger_id: String },
    /// AI NPC 대화창 열기/닫기. 실제 AI 호출은 game::npc::say가 따로 한다 — 여기는
    /// "지금 이 캐릭터와 대화 중이다"라는 상태 북마크일 뿐이다.
    DialogueStarted { game_character_id: String },
    DialogueEnded { game_character_id: String },
    /// AI 응답의 <choice> 태그 파싱 결과. 위 docstring 참고 — npc_say 전용.
    ChoicePresented { choices: Vec<String> },
    /// 씬의 선택지를 고른 결과. next_scene_id가 None이면 "씬 종료"(자유 이동 복귀).
    /// 씬을 실제로 여닫는 건 scene.rs 한 곳이고, effect는 raw JSON이라 변수 반영은 trigger.rs가
    /// VariableChanged로 환원해서 한다.
    ChoiceMade { choice_id: String, next_scene_id: Option<String>, effect: Option<serde_json::Value> },
    SceneChanged { scene_id: String },
    /// 이벤트 효과 stage가 내는 요청. 해석/검증/무대 반영은 stage 시스템이 한다.
    StageRequested { spec: serde_json::Value },
    /// 씬이 열렸을 때 그 씬의 대본 (줄 목록 + 줄마다 실행할 연출). 대본이 없는 씬은 나가지 않는다.
    SceneScript { scene_id: String, lines: Vec<crate::game::stage::StageLine> },
    /// 연출만 있는 줄들 (효과 stage, 위치/시간/날씨에 따른 배경 전환 등).
    StageLines { lines: Vec<crate::game::stage::StageLine> },
    SceneExited { scene_id: String },
    /// 변수 변경 "제안 또는 통지". state.flags에 실제로 쓰는 건 trigger.rs의
    /// VariableChanged 핸들러 한 곳뿐 — 다른 시스템은 이 이벤트를 emit만 하고 쓰기는 안 한다.
    VariableChanged { key: String, old: serde_json::Value, new: serde_json::Value },
    /// 내부 요청: 아이템을 delta만큼 늘리거나(양수) 줄인다(음수). 인벤토리 시스템이 스택/고유 규칙을 적용하고
    /// 실제로 바뀐 만큼만 ItemAdded/ItemRemoved로 알린다 (효과 give_item/take_item, UseItem, DropItem이 내는 요청).
    ItemChangeRequested { item_id: String, delta: i64 },
    ItemUsed { item_id: String, target_id: Option<String> },
    /// 내부 요청: from이 to에게 느끼는 axis를 delta만큼 바꾼다. 관계 시스템이 축/범위를 검증하고 적용한다
    /// (효과 relation, AI의 <updateRel>이 내는 요청).
    RelationChangeRequested { from: String, to: String, axis: String, delta: i64 },
    /// 실제로 바뀐 결과 (범위로 잘린 뒤의 값). 변화량만 game_relationship_snapshots에 쌓인다.
    RelationChanged { from: String, to: String, axis: String, old: i64, new: i64 },
    /// quantity는 요청한 수량이 아니라 실제로 들어온/나간 수량이다 (max_stack/고유 규칙으로 줄 수 있다).
    ItemAdded { item_id: String, quantity: i64 },
    ItemRemoved { item_id: String, quantity: i64 },
    GameEnded { ending_id: String },
}
