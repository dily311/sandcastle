//! 세션(=책 한 권)마다 바뀌는 상태. GameDefinition과 달리 플레이 중 계속 바뀌고,
//! 매 턴 game_player_sessions에 저장된다. 책갈피(game_saves)는 이 상태의 스냅샷 + 일지 위치(seq).
//! 필드 구성은 models::game::GamePlayerSessionsInput::from_state가 요구하는 모양과 맞춰야 한다.
use crate::models::game::GamePlayerSessions;
use std::collections::{HashMap, HashSet};

/// NPC 하나의 지금 상태. 위치는 스폰 규칙(시간대가 바뀔 때)이나 이벤트 등장으로 정해진다.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct NpcState {
    /// None이면 지금 어디에도 없다 (스폰 조건을 통과한 위치가 하나도 없음).
    pub location_id: Option<String>,
    /// "active" / "absent". (dead, traveling 등은 나중에.)
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct GameState {
    pub session_id: String,
    /// 세션에 고정된 페르소나. 페르소나는 항상 기본값을 가지므로 Option이 아니다
    /// (DB에서 NULL이면 repository.load_session이 기본 페르소나로 바꿔서 넘긴다).
    pub player_id: String,
    /// 분기(fork)로 만들어진 세션이면 원본 세션과 그 시점의 seq.
    pub parent_session_id: Option<String>,
    pub forked_from_seq: Option<u32>,
    /// 이 세션에서 마지막으로 발급한 seq. 일지/메시지/이벤트 로그가 모두 이 하나의 번호를 쓴다
    /// (분기할 때 "seq N까지"를 한 번에 자르려는 것). 발급은 GameRuntime::next_seq 한 곳.
    pub head_seq: u32,
    /// 실제 "지금 있는 곳"의 기준. map_id/zone_id는 이걸 보고 derive해서 캐시해둔 값
    /// (MovementSystem이 LocationChanged를 받을 때마다 같이 갱신한다).
    pub location_id: Option<String>,
    pub map_id: String,
    pub zone_id: Option<String>,
    pub game_time: i64,
    pub weather_id: String,
    pub time_period_id: String,
    pub current_scene_id: Option<String>,
    /// 대기 중인 씬 id 배열(JSON). 한 번에 하나만 열고 나머지는 여기서 기다린다.
    pub pending_scenes_json: serde_json::Value,
    /// 지금 AI 대화창이 열려 있는 NPC(game_character_id). 세션에는 저장하지 않는다 —
    /// 앱을 다시 열면 대화창은 항상 닫힌 상태로 시작한다 (Talk로 다시 열어야 함).
    #[serde(skip)]
    pub active_npc_id: Option<String>,
    /// game_state_defs 에 정의된 값들 (affection_yuna 같은 것들)
    pub flags: serde_json::Value,
    /// 인벤토리: item_id -> 수량 (0이 되면 키를 지운다). 쓰는 곳은 InventorySystem 하나.
    pub inventory_json: serde_json::Value,
    /// 관계: "보낸쪽>받는쪽" -> {affection, trust, tension, tags}. 보낸쪽/받는쪽은 'player' 또는 game_character_id.
    pub relationships_json: serde_json::Value,
    /// 무대 상태: 지금 배경/BGM 분위기/CG/서 있는 캐릭터와 감정 (이어하기/분기 때 같은 화면을 복원한다).
    pub stage_json: serde_json::Value,
    /// game_character_id -> 지금 상태/위치. 세션에는 컬럼이 없고 game_character_state_snapshots에 seq별로 쌓인다
    /// (불러올 때 캐릭터별 가장 최근 행을 읽는다). 스폰 규칙이 정의된 캐릭터만 여기에 생긴다.
    #[serde(skip)]
    pub npc_states: HashMap<String, NpcState>,
    /// event_id -> 마지막으로 발생한 game_time. 매 턴 event_log를 다시 안 읽으려고 메모리에 들고 있음.
    #[serde(skip)]
    pub cooldowns: HashMap<String, i64>,
    /// auto 이벤트 중 "지금 조건이 참인 것". 조건이 거짓→참으로 바뀌는 순간에만 발동하려는 것.
    /// 저장하지 않는다 — 불러온 직후에는 GameRuntime::load가 현재 참인 것들로 채워서(발동 없이)
    /// 이어하기 때 다시 터지지 않게 한다.
    #[serde(skip)]
    pub auto_active: HashSet<String>,
    /// 이번 dispatch에서 random을 뺀 이벤트가 하나라도 발동했는가. true면 random은 안 굴린다
    /// (중요한 장면 직후에 랜덤 전투가 겹치지 않게). dispatch 시작마다 false로 되돌린다.
    #[serde(skip)]
    pub turn_fired: bool,
    // TODO: inventory, relationships — 해당 시스템 설계되면 구체 타입으로 교체.
}

/// DB에서 NULL이거나 객체가 아닌 값이 오면 빈 객체로 바꾼다.
fn as_object(v: serde_json::Value) -> serde_json::Value {
    if v.is_object() { v } else { serde_json::json!({}) }
}

impl GameState {
    pub fn from_session(
        session: GamePlayerSessions,
        cooldowns: HashMap<String, i64>,
        time_period_id: String,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            session_id: session.id,
            player_id: session.player_id,
            parent_session_id: session.parent_session_id,
            forked_from_seq: session.forked_from_seq,
            head_seq: session.head_seq,
            location_id: session.current_location_id,
            map_id: session.current_map_id.ok_or("세션에 현재 맵이 없습니다")?,
            zone_id: session.current_zone_id,
            game_time: session.game_time,
            weather_id: session.current_weather_id.ok_or("세션에 현재 날씨가 없습니다")?,
            time_period_id,
            current_scene_id: session.current_scene_id,
            pending_scenes_json: if session.pending_scenes_json.is_array() { session.pending_scenes_json } else { serde_json::json!([]) },
            active_npc_id: None, // 항상 닫힌 상태로 시작 (위 주석 참고)
            flags: session.flags_json,
            inventory_json: as_object(session.inventory_json),
            relationships_json: as_object(session.relationships_json),
            stage_json: as_object(session.stage_json),
            npc_states: HashMap::new(), // GameRuntime::load가 DB에서 채운다
            cooldowns,
            auto_active: HashSet::new(),
            turn_fired: false,
        })
    }

    /// models::game::GamePlayerSessionsInput::from_state가 이 변환을 한다.
    pub fn to_session_input(&self, game_id: String) -> crate::models::game::GamePlayerSessionsInput {
        crate::models::game::GamePlayerSessionsInput::from_state(self.clone(), game_id)
    }
}

/// 지금 위치에서 갈 수 있는 곳 하나. available=false면 막혀 있다 (조건이 안 맞거나 씬이 진행 중).
/// 막힌 이유(조건)는 알려주지 않는다 — 문이 잠겨 있다는 것까지만 보인다.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AvailableMove {
    pub to_id: String,
    pub travel_seconds: u32,
    pub available: bool,
}

/// iframe에 노출하는 부분만 추린 뷰. 내부 상태를 그대로 노출하지 않는다.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RenderState {
    /// 지금 플레이 중인 세션(책). 분기로 불러오면 새 세션 id가 여기로 온다.
    pub session_id: String,
    pub location_id: Option<String>,
    pub map_id: String,
    pub zone_id: Option<String>,
    pub game_time: i64,
    pub weather_id: String,
    pub time_period_id: String,
    pub current_scene_id: Option<String>,
    pub active_npc_id: Option<String>,
    /// 지금 열려 있는 씬에서 고를 수 있는 선택지 id (표시 조건을 통과한 것만). GameRuntime::render_state가 채운다.
    pub available_choices: Vec<String>,
    /// 지금 위치에서 연결된 곳들과 갈 수 있는지 (MoveTo의 조건을 엔진이 미리 평가한 결과). get_game_view의 connections는 전체 지도다.
    pub available_moves: Vec<AvailableMove>,
    /// 지금 씬 입력(sceneSay)과 씬 종료 버튼을 쓸 수 있는가 (씬이 열려 있고 엔딩이 아니며 AI 이어가기가 켜져 있다).
    pub scene_chat: bool,
    pub flags: serde_json::Value,
    /// 지금 가진 아이템 (item_id -> 수량). 아이템 정의(이름/아이콘)는 get_game_view에 있다.
    pub inventory: serde_json::Value,
    pub relationships: serde_json::Value,
    /// 현재 무대(배경/BGM/CG/스탠딩). 지금은 저장된 그대로의 값이다.
    pub stage: serde_json::Value,
    /// 지금 플레이어가 있는 위치에 있는 NPC들 (game_character_id). GameRuntime::render_state가 채운다.
    pub present_npcs: Vec<String>,
}

impl From<&GameState> for RenderState {
    fn from(s: &GameState) -> Self {
        Self {
            session_id: s.session_id.clone(),
            location_id: s.location_id.clone(),
            map_id: s.map_id.clone(),
            zone_id: s.zone_id.clone(),
            game_time: s.game_time,
            weather_id: s.weather_id.clone(),
            time_period_id: s.time_period_id.clone(),
            current_scene_id: s.current_scene_id.clone(),
            active_npc_id: s.active_npc_id.clone(),
            available_choices: Vec::new(),
            available_moves: Vec::new(),
            scene_chat: false,
            flags: s.flags.clone(),
            inventory: s.inventory_json.clone(),
            relationships: s.relationships_json.clone(),
            stage: s.stage_json.clone(),
            present_npcs: Vec::new(),
        }
    }
}
