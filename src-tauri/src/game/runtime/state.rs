//! 세션(=책 한 권)마다 바뀌는 상태. GameDefinition과 달리 플레이 중 계속 바뀌고,
//! 매 턴 game_player_sessions에 저장된다. 책갈피(game_saves)는 이 상태의 스냅샷 + 일지 위치(seq).
//! 필드 구성은 models::game::GamePlayerSessionsInput::from_state가 요구하는 모양과 맞춰야 한다.
use crate::models::game::GamePlayerSessions;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    pub inventory_json: serde_json::Value,
    pub relationships_json: serde_json::Value,
    /// 지금 AI 대화창이 열려 있는 NPC(game_character_id). 세션에는 저장하지 않는다 —
    /// 앱을 다시 열면 대화창은 항상 닫힌 상태로 시작한다 (Talk로 다시 열어야 함).
    #[serde(skip)]
    pub active_npc_id: Option<String>,
    /// game_state_defs 에 정의된 값들 (affection_yuna 같은 것들)
    pub flags: serde_json::Value,
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
            inventory_json: session.inventory_json,
            relationships_json: session.relationships_json,
            active_npc_id: None, // 항상 닫힌 상태로 시작 (위 주석 참고)
            flags: session.flags_json,
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

/// iframe에 노출하는 부분만 추린 뷰. 내부 상태를 그대로 노출하지 않는다.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderState {
    pub location_id: Option<String>,
    pub map_id: String,
    pub zone_id: Option<String>,
    pub game_time: i64,
    pub weather_id: String,
    pub time_period_id: String,
    pub current_scene_id: Option<String>,
    pub active_npc_id: Option<String>,
    pub flags: serde_json::Value,
}

impl From<&GameState> for RenderState {
    fn from(s: &GameState) -> Self {
        Self {
            location_id: s.location_id.clone(),
            map_id: s.map_id.clone(),
            zone_id: s.zone_id.clone(),
            game_time: s.game_time,
            weather_id: s.weather_id.clone(),
            time_period_id: s.time_period_id.clone(),
            current_scene_id: s.current_scene_id.clone(),
            active_npc_id: s.active_npc_id.clone(),
            flags: s.flags.clone(),
        }
    }
}
