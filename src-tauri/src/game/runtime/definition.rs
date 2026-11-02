//! load() 시점에 한 번 통째로 읽어와서 플레이 내내 메모리에 들고 있는 정적 데이터.
//! 여기 담긴 건 전부 "게임 디자이너가 만든 것"이고, 플레이 중엔 안 바뀐다
//! (바뀌는 건 GameState 쪽).
//!
//! 공간 계층: 맵(GameMaps) > 존(GameMapsZone) > 위치(GameLocations). 이동은 location
//! 단위(game_location_connections)로 일어나고, zone/map은 그 location이 속한 곳을
//! 거슬러 찾아서 derive한다.
use crate::game::engine::Condition;
use crate::models::game::{
    GameCharacters, GameEncounterEntries, GameEncounterTables, GameEndings, GameEventCharacterPool,
    AudioAssets, BackgroundAssets, CgAssets, CharacterAssets, GameAssets, GameCharacterSpawnLocations, GameEvents, GameInteractables, GameItems, GameRelationshipDefs, GameLocationConnections, GameLocations, GameMaps, GameMapsZone,
    GameProject, GameScene, GameStateDef, GameTimePeriods, GameWeatherTransitions, GameWeatherTypes,
};
use crate::models::conversation::WorldInfoEntry;
use std::collections::HashMap;

/// 에셋 전체. all은 종류별 하위 정보가 없는 기본 행(폰트/기타 포함)까지 전부, 나머지는 종류별 하위 정보가 붙은 것.
#[derive(Default)]
pub struct Assets {
    pub all: Vec<GameAssets>,
    pub audio: Vec<AudioAssets>,
    pub characters: Vec<CharacterAssets>,
    pub cgs: Vec<CgAssets>,
    pub backgrounds: Vec<BackgroundAssets>,
}

pub struct GameDefinition {
    pub game_id: String,
    pub project: GameProject,
    pub state_defs: Vec<GameStateDef>,
    pub maps: Vec<GameMaps>,
    pub zones: Vec<GameMapsZone>,
    pub locations: Vec<GameLocations>,
    pub location_connections: Vec<GameLocationConnections>,
    /// location_id -> 그 자리에 있는 상호작용 대상들 (문/상자/스위치...)
    pub interactables: HashMap<String, Vec<GameInteractables>>,
    pub time_periods: Vec<GameTimePeriods>,
    pub weather_types: Vec<GameWeatherTypes>,
    pub weather_transitions: Vec<GameWeatherTransitions>,
    /// event_id -> GameEvents
    pub events: HashMap<String, GameEvents>,
    /// event_id -> 파싱된 조건 목록 (조건_value에 Condition JSON 통째로 저장하는 규칙, engine.rs 참고)
    pub event_conditions: HashMap<String, Vec<Condition>>,
    /// event_id -> NPC 후보 풀
    pub event_npc_pool: HashMap<String, Vec<GameEventCharacterPool>>,
    pub encounter_tables: Vec<GameEncounterTables>,
    pub encounter_entries: Vec<GameEncounterEntries>,
    pub scenes: Vec<GameScene>,
    pub endings: Vec<GameEndings>,
    /// item_id -> 아이템 정의. 수량은 정의가 아니라 세션 상태(GameState.inventory_json)에 있다.
    pub items: HashMap<String, GameItems>,
    /// 제작자가 심어둔 초기 관계. 새 게임을 시작할 때 세션의 relationships_json으로 복사된다.
    pub relationship_defs: Vec<GameRelationshipDefs>,
    /// game_character_id -> 그 NPC가 나올 수 있는 위치와 가중치/조건. 여기에 없는 캐릭터는 "위치 추적 안 함"
    /// (항상 말 걸 수 있다 — 스폰을 안 쓰는 기존 게임이 안 깨지게).
    pub spawn_locations: HashMap<String, Vec<GameCharacterSpawnLocations>>,
    /// 에셋 (파일 자체는 games/<id>/assets/에 있고 여기엔 키/종류/경로/조건만 있다).
    pub assets: Assets,
    /// game_character_id -> GameCharacters. 상태창(캐릭터별 상태/관계 표시)과 npc_say가
    /// 둘 다 자주 조회해서 여기 넣었다 — DB 왕복 없이 메모리에서 바로 찾는다.
    pub characters: HashMap<String, GameCharacters>,
    /// 이 게임 전체에만 적용되는 월드인포 (game_projects.world_info_ids의 북들, enabled entry만).
    pub game_world_info: Vec<WorldInfoEntry>,
    /// game_character_id -> 그 캐릭터에 연결된 월드인포 북들의 entry (enabled만).
    /// 키 매칭/예산 배분은 PromptManager가 한다 — 여기선 후보만 들고 있는다.
    /// ⚠ 북/엔트리는 게임 정의가 아니라 전역 테이블(worldinfo_*)이라 export 때 따로 챙겨야 한다.
    pub character_world_info: HashMap<String, Vec<WorldInfoEntry>>,
    // TODO: game_assets, game_character_worldinfo — export 완전판 만들 때 필요해지면 추가
}

impl GameDefinition {
    pub fn map_by_id(&self, map_id: &str) -> Option<&GameMaps> {
        self.maps.iter().find(|m| m.id == map_id)
    }

    pub fn zone_by_id(&self, zone_id: &str) -> Option<&GameMapsZone> {
        self.zones.iter().find(|z| z.id == zone_id)
    }

    pub fn location(&self, location_id: &str) -> Option<&GameLocations> {
        self.locations.iter().find(|l| l.id == location_id)
    }

    pub fn zone_of_location(&self, location_id: &str) -> Option<&GameMapsZone> {
        let loc = self.location(location_id)?;
        self.zone_by_id(&loc.zone_id)
    }

    pub fn map_of_location(&self, location_id: &str) -> Option<&GameMaps> {
        let zone = self.zone_of_location(location_id)?;
        self.map_by_id(&zone.map_id)
    }

    pub fn connection(&self, from_id: &str, to_id: &str) -> Option<&GameLocationConnections> {
        self.location_connections.iter().find(|c| c.from_id == from_id && c.to_id == to_id)
    }

    pub fn interactable(&self, entity_id: &str) -> Option<&GameInteractables> {
        self.interactables.values().flatten().find(|i| i.id == entity_id)
    }

    pub fn encounter_tables_for_map<'a>(&'a self, map_id: &'a str) -> impl Iterator<Item = &'a GameEncounterTables> {
        self.encounter_tables.iter().filter(move |t| t.map_id == map_id)
    }

    pub fn entries_for_table<'a>(&'a self, table_id: &'a str) -> impl Iterator<Item = &'a GameEncounterEntries> {
        self.encounter_entries.iter().filter(move |e| e.encounter_table_id == table_id)
    }

    pub fn weather_transitions_for(&self, map_id: &str, from_weather_id: &str) -> Vec<&GameWeatherTransitions> {
        self.weather_transitions.iter().filter(|t| t.map_id == map_id && t.from_weather_id == from_weather_id).collect()
    }

    pub fn character(&self, character_id: &str) -> Option<&GameCharacters> {
        self.characters.get(character_id)
    }

    /// game_state_defs.config_json에서 읽은 AI 접근 규칙 (key -> 규칙).
    pub fn ai_var_rules(&self) -> HashMap<String, crate::game::engine::VarRule> {
        self.state_defs.iter().map(|d| (d.key.clone(), crate::game::engine::VarRule::from_config(&d.config_json))).collect()
    }

    /// 이 맵의 위험도가 인카운터 확률에 곱해지는 배율. 곡선은 게임마다 다를 수 있어서
    /// game_projects.settings_json의 {"dangerEncounterScale": 0.25}로 정하고, 없으면 0.25(레벨 1당 +25%).
    pub fn danger_multiplier(&self, map_id: &str) -> f64 {
        let scale = self
            .project
            .settings_json
            .as_ref()
            .and_then(|v| v.get("dangerEncounterScale").and_then(|x| x.as_f64()))
            .unwrap_or(0.25);
        crate::game::engine::danger_multiplier(self.map_by_id(map_id).map_or(0, |m| m.danger_level), scale)
    }

    pub fn scene(&self, scene_id: &str) -> Option<&GameScene> {
        self.scenes.iter().find(|s| s.id == scene_id)
    }

    /// choice_id만으로 찾는다. (scene, choice)를 같이 준다.
    /// ⚠ 선택지의 표시 조건(choice.condition)은 여기서 평가하지 않는다 — 프론트가 보여줄 때/
    ///   엔진이 받아들일 때 따로 확인해야 한다 (TODO).
    pub fn find_choice(&self, choice_id: &str) -> Option<(&GameScene, &crate::models::game::GameChoiceRow)> {
        for scene in &self.scenes {
            if let Some(choice) = scene.choices.as_ref().and_then(|cs| cs.iter().find(|c| c.id == choice_id)) {
                return Some((scene, choice));
            }
        }
        None
    }

    /// 시작 지점. location은 game_projects.start_location_id를 쓴다. 날씨는 전용 컬럼이 없어서
    /// settings_json의 {"startWeatherId": "..."}에서 읽고, 없으면 호출부가 첫 날씨로 폴백한다.
    /// (start_map_id/start_scene_key 컬럼도 있지만 map은 location에서 derive되므로 안 쓴다.)
    pub fn start_point(&self) -> StartPoint {
        let parsed: Option<serde_json::Value> = self.project.settings_json.clone();
        StartPoint {
            location_id: self.project.start_location_id.clone(),
            weather_id: parsed.as_ref().and_then(|v| v.get("startWeatherId")).and_then(|v| v.as_str()).map(str::to_string),
            start_minute: parsed.as_ref().and_then(|v| v.get("startMinute")).and_then(|v| v.as_i64()),
        }
    }
}

#[derive(Default)]
pub struct StartPoint {
    pub location_id: Option<String>,
    pub weather_id: Option<String>,
    /// 하루 중 시작 시각(분, 0~1439). 없으면 0시. settings_json의 {"startMinute": 480} = 08:00.
    pub start_minute: Option<i64>,
}
