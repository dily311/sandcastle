//! GameRuntime은 이 trait만 안다. 실제 쿼리는 여기서만 한다.
//! GameRuntime::load() 이후로는 이 trait가 다시 안 불림 (정적 데이터는 메모리에 있음).
//! 매 턴 부르는 건 persist_session / append_event_log 뿐.
use super::definition::GameDefinition;
use crate::db::game::{
    get_all_event_characters, get_all_event_conditions, get_all_game_characters, get_all_game_branches, get_all_game_choices,
    get_all_game_encounter_table_entries, get_all_game_encounter_tables, get_all_game_endings,
    get_all_game_event_logs, get_all_game_events, get_all_game_interactables, get_all_game_location_connections,
    get_all_audio_assets, get_all_background_assets, get_all_cg_assets, get_all_character_assets, get_all_game_assets, get_all_game_character_spawn_locations, get_all_game_items, get_all_game_locations, get_all_game_relationship_defs, get_all_game_map_zones, get_all_game_maps, get_all_game_state_defs,
    get_all_game_time_periods, get_all_game_weather_transitions, get_all_game_weather_types, get_all_game_scenes,
    get_game_player_sessions, get_game_project, get_session_game_character_state_snapshots, upsert_game_event_log,
    upsert_game_event_log_character, upsert_game_player_sessions,
};
use crate::db::conversation::{get_all_worldinfo_entries, get_system_default_persona};
use crate::game::engine::Condition;
use crate::models::conversation::WorldInfoEntry;
use crate::models::game::{
    GameEventLogCharactersInput, GameEventLogInput, GamePlayerSessions, GamePlayerSessionsInput, GameScene,
};
use anyhow::Result;
use sqlx::SqlitePool;
use std::collections::HashMap;

// 앱 안에서만 쓰는 트레이트라 Send 경계를 따로 적지 않는다
#[allow(async_fn_in_trait)]
pub trait GameRepository {
    async fn load_definition(&self, game_id: &str) -> Result<GameDefinition>;
    async fn load_session(&self, session_id: &str) -> Result<GamePlayerSessions>;
    /// 세션 쿨다운 계산용. GameDefinition에 안 넣은 이유: 세션마다 다르기 때문.
    async fn load_cooldowns(&self, game_id: &str, session_id: &str, now: i64) -> Result<HashMap<String, i64>>;
    async fn persist_session(&self, input: GamePlayerSessionsInput) -> Result<()>;
    async fn append_event_log(&self, entry: GameEventLogInput) -> Result<()>;
    /// game_event_log에서 game_character_id가 빠지고 독립된 junction 테이블이 됐다.
    /// NPC가 있는 로그는 append_event_log로 로그를 쓴 다음 이것도 같이 불러야 한다.
    async fn append_event_log_character(&self, entry: GameEventLogCharactersInput) -> Result<()>;
    /// 세션의 NPC 상태: 캐릭터별 가장 최근(seq가 가장 큰) 기록.
    async fn load_npc_states(&self, session_id: &str) -> Result<HashMap<String, super::state::NpcState>>;
}

pub struct SqlxGameRepository<'a> {
    pub pool: &'a SqlitePool,
}

impl<'a> SqlxGameRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }
}

impl<'a> GameRepository for SqlxGameRepository<'a> {
    async fn load_definition(&self, game_id: &str) -> Result<GameDefinition> {
        let pool = self.pool;
        // futures::try_join!로 동시 실행 (engine_move에서 쓰던 것과 같은 이유 —
        // tokio::try_join!은 android/ios 빌드가 깨질 수 있어서 안 씀)
        let (
            project,
            state_defs,
            maps,
            zones,
            locations,
            location_connections,
            interactables_flat,
            periods,
            weather_types,
            weather_transitions,
            events,
            conditions,
            npc_pool,
            tables,
            entries,
            endings,
            characters,
        ) = futures::try_join!(
            get_game_project(pool, game_id),
            get_all_game_state_defs(pool, game_id),
            get_all_game_maps(pool, game_id),
            get_all_game_map_zones(pool, game_id),
            get_all_game_locations(pool, game_id),
            get_all_game_location_connections(pool, game_id),
            get_all_game_interactables(pool, game_id),
            get_all_game_time_periods(pool, game_id),
            get_all_game_weather_types(pool, game_id),
            get_all_game_weather_transitions(pool, game_id),
            get_all_game_events(pool, game_id),
            get_all_event_conditions(pool, game_id),
            get_all_event_characters(pool, game_id),
            get_all_game_encounter_tables(pool, game_id),
            get_all_game_encounter_table_entries(pool, game_id),
            get_all_game_endings(pool, game_id),
            get_all_game_characters(pool, game_id),
        )?;

        let (items, relationship_defs, spawn_rows) = futures::try_join!(
            get_all_game_items(pool, game_id),
            get_all_game_relationship_defs(pool, game_id),
            get_all_game_character_spawn_locations(pool, game_id),
        )?;
        let mut spawn_locations: HashMap<String, Vec<_>> = HashMap::new();
        for s in spawn_rows {
            spawn_locations.entry(s.game_character_id.clone()).or_default().push(s);
        }

        let (all_assets, audio, characters_assets, cgs, backgrounds) = futures::try_join!(
            get_all_game_assets(pool, game_id),
            get_all_audio_assets(pool, game_id),
            get_all_character_assets(pool, game_id),
            get_all_cg_assets(pool, game_id),
            get_all_background_assets(pool, game_id),
        )?;

        // 씬 + 선택지/분기. 선택지와 분기는 scene_id로 씬에 붙는다 (노드는 없어졌다).
        let (scene_rows, choices, branches) = futures::try_join!(
            get_all_game_scenes(pool, game_id),
            get_all_game_choices(pool, game_id),
            get_all_game_branches(pool, game_id),
        )?;
        let mut choices_by_scene: HashMap<String, Vec<_>> = HashMap::new();
        for c in choices {
            choices_by_scene.entry(c.scene_id.clone()).or_default().push(c);
        }
        let mut branches_by_scene: HashMap<String, Vec<_>> = HashMap::new();
        for b in branches {
            branches_by_scene.entry(b.scene_id.clone()).or_default().push(b);
        }
        let scenes: Vec<GameScene> = scene_rows
            .into_iter()
            .map(|row| {
                let cs = choices_by_scene.remove(&row.id);
                let bs = branches_by_scene.remove(&row.id);
                GameScene::from_row(row, cs, bs)
            })
            .collect();

        let conditions_by_event: HashMap<String, Vec<Condition>> = {
            let mut m: HashMap<String, Vec<Condition>> = HashMap::new();
            for c in conditions {
                // 조건 JSON을 못 읽으면 "조건 없음"으로 버리지 않고 절대 참이 아닌 조건(Never)으로 둔다 —
                // 버리면 오염된 데이터가 오히려 이벤트를 항상 통과시킨다 (fail-closed).
                let parsed = c.condition_value.as_ref().and_then(|s| serde_json::from_str::<Condition>(s).ok()).unwrap_or(Condition::Never);
                m.entry(c.event_id).or_default().push(parsed);
            }
            m
        };

        // 월드인포: 게임 전체용(project.world_info_ids)과 캐릭터별(characters[].world_info_ids).
        // 같은 북을 여러 곳이 공유할 수 있어서 북 단위로 한 번만 읽고(enabled만) 나눠 준다.
        let mut book_entries: HashMap<String, Vec<WorldInfoEntry>> = HashMap::new();
        let wanted: Vec<String> = project
            .world_info_ids
            .iter()
            .chain(characters.iter().flat_map(|c| c.world_info_ids.iter()))
            .cloned()
            .collect();
        for book_id in wanted {
            if !book_entries.contains_key(&book_id) {
                let entries = get_all_worldinfo_entries(pool, &book_id).await?;
                book_entries.insert(book_id, entries.into_iter().filter(|e| e.enabled).collect());
            }
        }
        let entries_of = |ids: &[String]| -> Vec<WorldInfoEntry> { ids.iter().filter_map(|id| book_entries.get(id)).flatten().cloned().collect() };
        let game_world_info = entries_of(&project.world_info_ids);
        let character_world_info: HashMap<String, Vec<WorldInfoEntry>> = characters
            .iter()
            .filter(|c| !c.world_info_ids.is_empty())
            .map(|c| (c.id.clone(), entries_of(&c.world_info_ids)))
            .collect();

        Ok(GameDefinition {
            game_id: game_id.to_string(),
            project,
            state_defs,
            maps,
            zones,
            locations,
            location_connections,
            interactables: {
                let mut m: HashMap<String, Vec<_>> = HashMap::new();
                for i in interactables_flat {
                    m.entry(i.location_id.clone()).or_default().push(i);
                }
                m
            },
            time_periods: periods,
            weather_types,
            weather_transitions,
            events: events.into_iter().map(|e| (e.id.clone(), e)).collect(),
            event_conditions: conditions_by_event,
            event_npc_pool: {
                let mut m: HashMap<String, Vec<_>> = HashMap::new();
                for p in npc_pool {
                    m.entry(p.event_id.clone()).or_default().push(p);
                }
                m
            },
            encounter_tables: tables,
            encounter_entries: entries,
            scenes,
            endings,
            items: items.into_iter().map(|i| (i.id.clone(), i)).collect(),
            relationship_defs,
            spawn_locations,
            assets: super::definition::Assets { all: all_assets, audio, characters: characters_assets, cgs, backgrounds },
            characters: characters.into_iter().map(|c| (c.id.clone(), c)).collect(),
            game_world_info,
            character_world_info,
        })
    }

    async fn load_session(&self, session_id: &str) -> Result<GamePlayerSessions> {
        let mut session = get_game_player_sessions(self.pool, session_id).await?;
        // player_id가 NULL(예: 지워진 페르소나)이면 Row→모델 변환이 빈 문자열로 만든다.
        // 페르소나는 항상 있어야 하므로 여기서 시스템 기본 페르소나로 바꿔 넘긴다.
        if session.player_id.is_empty() {
            session.player_id = get_system_default_persona(self.pool).await?;
        }
        Ok(session)
    }

    async fn load_cooldowns(&self, game_id: &str, session_id: &str, now: i64) -> Result<HashMap<String, i64>> {
        let logs = get_all_game_event_logs(self.pool, game_id).await?;
        Ok(logs
            .into_iter()
            .filter(|l| l.session_id == session_id)
            .map(|l| (l.event_id, l.triggered_at as i64))
            .collect::<HashMap<_, _>>()
            .into_iter()
            .filter(|(_, t)| *t <= now) // 미래 타임스탬프 방지용 방어적 필터
            .collect())
    }

    async fn persist_session(&self, input: GamePlayerSessionsInput) -> Result<()> {
        Ok(upsert_game_player_sessions(self.pool, input).await?)
    }

    async fn append_event_log(&self, entry: GameEventLogInput) -> Result<()> {
        Ok(upsert_game_event_log(self.pool, entry).await?)
    }

    async fn load_npc_states(&self, session_id: &str) -> Result<HashMap<String, super::state::NpcState>> {
        let rows = get_session_game_character_state_snapshots(self.pool, session_id).await?;
        let mut latest: HashMap<String, (u32, super::state::NpcState)> = HashMap::new();
        for r in rows {
            let newer = latest.get(&r.game_character_id).map_or(true, |(seq, _)| r.seq >= *seq);
            if newer {
                latest.insert(r.game_character_id, (r.seq, super::state::NpcState { location_id: r.current_location_id, status: r.status }));
            }
        }
        Ok(latest.into_iter().map(|(k, (_, v))| (k, v)).collect())
    }

    async fn append_event_log_character(&self, entry: GameEventLogCharactersInput) -> Result<()> {
        Ok(upsert_game_event_log_character(self.pool, entry).await?)
    }
}
