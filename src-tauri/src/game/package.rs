//! .tgame 패키지 포맷 (zip):
//!   manifest.json   — 이름/제작자/format_version/엔진 최소 버전
//!   data/definition.json — PackageData (각 테이블의 *Input 타입 모음, 세이브/세션/로그/대화 제외)
//!   index.html, scripts/, assets/ — games/<game_id>/ 아래 전체를 그대로 복사
//!
//! 내부 엔티티 id(맵/씬/노드/이벤트...)는 import해도 안 바뀐다 — 그래야 서로를 가리키는
//! next_node_id 같은 참조가 그대로 유효하다. 바뀌는 건 game_id와 월드인포 북 id다
//! (북은 전역 테이블이라 새로 만들고, project/characters의 world_info_ids를 새 id로 다시 이어준다).
use crate::db::conversation::{create_worldinfo_book, get_all_worldinfo_entries, get_worldinfo_book, upsert_worldinfo_entry};
use crate::db::game::{
    delete_game_branches, delete_game_character, delete_game_choice, delete_game_encounter_table,
    delete_game_character_spawn_location, delete_game_encounter_table_entry, delete_game_item, delete_game_relationship_def, delete_game_ending, delete_game_event, delete_game_event_character,
    delete_game_event_conditions, delete_game_interactables, delete_game_location_connection, delete_game_locations,
    delete_game_map, delete_game_scene, delete_game_state_def, delete_game_time_period, delete_game_weather_transition,
    delete_game_weather_type, delete_game_zone,
    upsert_game_character, upsert_game_encounter_table, upsert_game_encounter_table_entry,
    upsert_game_ending, upsert_game_event, upsert_game_event_character, upsert_game_event_condition,
    upsert_audio_asset, upsert_background_assets, upsert_cg_asset, upsert_character_asset, upsert_game_asset, delete_game_asset, upsert_game_character_spawn_location, upsert_game_interactables, upsert_game_item, upsert_game_relationship_def, upsert_game_location_connections, upsert_game_locations,
    upsert_game_branches, upsert_game_choice, upsert_game_map, upsert_game_project, upsert_game_scene, upsert_game_state_defs,
    upsert_game_time_period, upsert_game_weather_transition, upsert_game_weather_type,
    upsert_game_zone,
};
use crate::error::AppError;
use crate::game::runtime::GameDefinition;
use crate::models::conversation::{WorldInfoBookInput, WorldInfoEntryInput};
use crate::models::game::{
    GameCharactersInput, GameEncounterEntriesInput, GameEncounterTablesInput, GameEndingsInput,
    GameEventCharacterPoolInput, GameEventConditionsInput, AudioAssetsInput, BackgroundAssetsInput, CgAssetsInput, CharacterAssetsInput, GameAssetsInput, GameCharacterSpawnLocationsInput, GameEventsInput, GameInteractablesInput, GameItemsInput, GameRelationshipDefsInput,
    GameLocationConnectionsInput, GameLocationsInput, GameMapsInput, GameMapsZoneInput,
    GameProjectInput, GameSceneInput, GameStateDefInput, GameTimePeriodsInput,
    GameWeatherTransitionsInput, GameWeatherTypesInput,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use sqlx::{Sqlite, Transaction};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: String,
    pub title: String,
    pub author: Option<String>,
    pub min_engine_version: String,
}

/// 게임이 쓰는 월드인포 북 하나(+ 그 entry 전부, 비활성 포함). id는 "원본 DB에서의 id"로,
/// import 때 새 북을 만들고 {원본 id → 새 id} 맵을 만드는 용도로만 쓴다.
/// ⚠ WorldInfoBookInput/WorldInfoEntryInput에 Serialize derive가 필요하다 (지금은 Deserialize만).
#[derive(Serialize, Deserialize)]
pub struct PackageWorldInfoBook {
    pub id: String,
    pub book: WorldInfoBookInput,
    pub entries: Vec<WorldInfoEntryInput>,
}

/// data/definition.json의 모양. 전부 *Input 타입이라 export도 import도 이걸 거친다 —
/// Output(Row) 타입에는 Deserialize가 없어서 그대로는 못 돌려받는다.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageData {
    pub project: GameProjectInput,
    pub state_defs: Vec<GameStateDefInput>,
    pub maps: Vec<GameMapsInput>,
    pub zones: Vec<GameMapsZoneInput>,
    pub locations: Vec<GameLocationsInput>,
    pub location_connections: Vec<GameLocationConnectionsInput>,
    pub interactables: Vec<GameInteractablesInput>,
    pub time_periods: Vec<GameTimePeriodsInput>,
    pub weather_types: Vec<GameWeatherTypesInput>,
    pub weather_transitions: Vec<GameWeatherTransitionsInput>,
    pub events: Vec<GameEventsInput>,
    pub event_conditions: Vec<GameEventConditionsInput>,
    pub event_character_pool: Vec<GameEventCharacterPoolInput>,
    pub encounter_tables: Vec<GameEncounterTablesInput>,
    pub encounter_entries: Vec<GameEncounterEntriesInput>,
    pub scenes: Vec<GameSceneInput>,
    pub endings: Vec<GameEndingsInput>,
    pub characters: Vec<GameCharactersInput>,
    #[serde(default)]
    pub spawn_locations: Vec<GameCharacterSpawnLocationsInput>,
    #[serde(default)]
    pub items: Vec<GameItemsInput>,
    #[serde(default)]
    pub relationship_defs: Vec<GameRelationshipDefsInput>,
    /// 에셋: 기본 행(폰트/기타)과 종류별 하위 정보가 붙은 것들. 파일 자체는 zip의 assets/ 아래에 있다.
    #[serde(default)]
    pub assets: Vec<GameAssetsInput>,
    #[serde(default)]
    pub audio_assets: Vec<AudioAssetsInput>,
    #[serde(default)]
    pub character_assets: Vec<CharacterAssetsInput>,
    #[serde(default)]
    pub cg_assets: Vec<CgAssetsInput>,
    #[serde(default)]
    pub background_assets: Vec<BackgroundAssetsInput>,
    /// project/characters의 world_info_ids가 가리키는 북들. 페르소나 쪽 북은 개인 설정이라 안 담는다.
    #[serde(default)]
    pub world_info_books: Vec<PackageWorldInfoBook>,
    // TODO: game_assets — GameDefinition에 추가되면 여기도.
}

const MAX_PACKAGE_BYTES: u64 = 200 * 1024 * 1024; // TODO: 실제 상한 정하기

/// GameDefinition(조회용 Output 타입들)을 PackageData(Input 타입들)로 바꾼다. export 전용.
/// 각 Output → Input 변환은 models::game의 From 구현을 쓴다.
fn to_package_data(def: &GameDefinition, world_info_books: Vec<PackageWorldInfoBook>) -> PackageData {
    PackageData {
        project: def.project.clone().into(),
        state_defs: def.state_defs.clone().into_iter().map(Into::into).collect(),
        maps: def.maps.clone().into_iter().map(Into::into).collect(),
        zones: def.zones.clone().into_iter().map(Into::into).collect(),
        time_periods: def.time_periods.clone().into_iter().map(Into::into).collect(),
        weather_types: def.weather_types.clone().into_iter().map(Into::into).collect(),
        weather_transitions: def.weather_transitions.clone().into_iter().map(Into::into).collect(),
        events: def.events.values().cloned().map(Into::into).collect(),
        // event_conditions는 GameDefinition엔 "파싱된 Condition"만 있어서(engine.rs::Condition),
        // 되돌릴 때 Condition을 다시 JSON으로 직렬화해서 condition_value에 넣는다.
        // id는 잃어버렸지만(해시맵이라) 조건은 다른 엔티티가 id로 참조하지 않으니 새로 발급돼도 무해하다.
        event_conditions: def
            .event_conditions
            .iter()
            .flat_map(|(event_id, conditions)| conditions.iter().filter_map(move |c| GameEventConditionsInput::from_condition(event_id, c)))
            .collect(),
        event_character_pool: def
            .event_npc_pool
            .iter()
            .flat_map(|(event_id, pool)| pool.iter().map(move |c| GameEventCharacterPoolInput::from_condition(event_id, c)))
            .collect(),
        encounter_tables: def.encounter_tables.clone().into_iter().map(Into::into).collect(),
        encounter_entries: def.encounter_entries.clone().into_iter().map(Into::into).collect(),
        scenes: def.scenes.clone().into_iter().map(Into::into).collect(),
        endings: def.endings.clone().into_iter().map(Into::into).collect(),
        characters: def.characters.values().cloned().map(Into::into).collect(),
        spawn_locations: def.spawn_locations.values().flatten().cloned().map(Into::into).collect(),
        // 폰트/기타는 기본 행만, 나머지는 종류별 하위 정보를 포함한 입력으로 (기본 행이 두 번 들어가지 않게 나눈다)
        assets: def.assets.all.iter().filter(|a| matches!(a.category.as_str(), "font" | "other")).cloned().map(Into::into).collect(),
        audio_assets: def.assets.audio.iter().cloned().map(Into::into).collect(),
        character_assets: def.assets.characters.iter().cloned().map(Into::into).collect(),
        cg_assets: def.assets.cgs.iter().cloned().map(Into::into).collect(),
        background_assets: def.assets.backgrounds.iter().cloned().map(Into::into).collect(),
        items: def.items.values().cloned().map(GameItemsInput::from).collect(),
        relationship_defs: def.relationship_defs.clone().into_iter().map(Into::into).collect(),
        locations: def.locations.clone().into_iter().map(Into::into).collect(),
        location_connections: def.location_connections.clone().into_iter().map(Into::into).collect(),
        interactables: def
            .interactables
            .iter()
            .flat_map(|(location_id, inter)| inter.iter().map(move |i| GameInteractablesInput::from_condition(location_id, i)))
            .collect(),
        world_info_books,
    }
}

/// 에디터가 편집하려고 여는 데이터. save_package가 받는 PackageData와 같은 모양이라 왕복이 된다.
/// 월드인포 북은 에디터가 안 다루므로(전역 테이블, 별도 화면에서 관리) 비워서 준다.
pub fn to_editor_data(def: &GameDefinition) -> PackageData {
    to_package_data(def, Vec::new())
}

/// 게임이 참조하는 월드인포 북(project + characters)을 DB에서 읽어서 패키지용으로 만든다.
/// GameDefinition에는 enabled entry만 있고 북 이름/설명도 없어서, 여기서 DB를 직접 읽는다.
async fn collect_world_info_books(pool: &SqlitePool, def: &GameDefinition) -> Result<Vec<PackageWorldInfoBook>, AppError> {
    let mut ids: Vec<String> = def.project.world_info_ids.clone();
    for c in def.characters.values() {
        for id in &c.world_info_ids {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
    }
    let mut out = Vec::new();
    for id in ids {
        let book = match get_worldinfo_book(pool, &id).await {
            Ok(b) => b,
            Err(sqlx::Error::RowNotFound) => continue, // 이미 지워진 북을 가리키는 dangling id는 건너뜀
            Err(e) => return Err(e.into()),
        };
        let entries = get_all_worldinfo_entries(pool, &id).await?;
        out.push(PackageWorldInfoBook {
            id: id.clone(),
            book: WorldInfoBookInput { id: None, name: book.name, description: book.description },
            entries: entries
                .into_iter()
                .map(|e| WorldInfoEntryInput {
                    id: None, // import 때 새로 발급
                    book_id: id.clone(),
                    keys: e.keys,
                    secondary_keys: e.secondary_keys,
                    content: e.content,
                    enabled: e.enabled,
                    priority: e.priority,
                    constant: e.constant,
                })
                .collect(),
        });
    }
    Ok(out)
}

/// 정의 데이터를 부모가 먼저 오는 순서로 upsert한다 (FK 순서). import와 save_package가 같이 쓴다.
/// game_id는 호출자가 정한 값으로 전부 덮어쓴다 (패키지/에디터가 준 값을 믿지 않는다).
/// replace_conditions=true면 이벤트의 조건을 "그 이벤트 것 전부 지우고 다시 넣는다" —
/// 조건은 GameDefinition에서 id를 잃은 채 다뤄지고 다른 곳이 id로 참조하지 않아서 통째 교체가 안전하다.
/// world_info_books는 여기서 안 다룬다 (전역 테이블이라 import만 따로 처리).
pub(crate) async fn write_definition(tx: &mut Transaction<'_, Sqlite>, game_id: &str, mut data: PackageData, replace_conditions: bool) -> Result<(), AppError> {
    data.project.id = Some(game_id.to_string());
    for m in &mut data.maps {
        m.game_id = game_id.to_string();
    }
    for p in &mut data.time_periods {
        p.game_id = game_id.to_string();
    }
    for w in &mut data.weather_types {
        w.game_id = game_id.to_string();
    }
    for e in &mut data.events {
        e.game_id = game_id.to_string();
    }
    for e in &mut data.endings {
        e.game_id = game_id.to_string();
    }
    for c in &mut data.characters {
        c.game_id = game_id.to_string();
    }
    for i in &mut data.items {
        i.game_id = game_id.to_string();
    }
    for r in &mut data.relationship_defs {
        r.game_id = game_id.to_string();
    }
    for a in &mut data.assets {
        a.game_id = game_id.to_string();
    }
    for a in &mut data.audio_assets {
        a.base.game_id = game_id.to_string();
    }
    for a in &mut data.character_assets {
        a.base.game_id = game_id.to_string();
    }
    for a in &mut data.cg_assets {
        a.base.game_id = game_id.to_string();
    }
    for a in &mut data.background_assets {
        a.base.game_id = game_id.to_string();
    }

    upsert_game_project(&mut **tx, &data.project).await?;
    for s in &data.state_defs {
        upsert_game_state_defs(&mut **tx, game_id, s).await?;
    }
    for m in data.maps {
        upsert_game_map(&mut **tx, game_id, m).await?;
    }
    for z in data.zones {
        let map_id = z.map_id.clone();
        upsert_game_zone(&mut **tx, &map_id, z).await?;
    }
    for l in data.locations {
        let zone_id = l.zone_id.clone();
        upsert_game_locations(&mut **tx, &zone_id, l).await?;
    }
    for c in data.location_connections {
        upsert_game_location_connections(&mut **tx, c).await?;
    }
    for p in data.time_periods {
        upsert_game_time_period(&mut **tx, game_id, p).await?;
    }
    for w in data.weather_types {
        upsert_game_weather_type(&mut **tx, game_id, w).await?;
    }
    for t in data.weather_transitions {
        upsert_game_weather_transition(&mut **tx, t).await?;
    }
    // 캐릭터는 event_character_pool이 FK로 가리키므로 풀보다 먼저 와야 한다.
    for c in data.characters {
        upsert_game_character(&mut **tx, game_id, c).await?;
    }
    // 스폰 규칙은 캐릭터와 location 둘 다 FK로 가리키므로 둘 뒤에 온다.
    for s in data.spawn_locations {
        upsert_game_character_spawn_location(&mut **tx, s).await?;
    }
    // 에셋은 캐릭터/location/시간대/날씨(FK)와 이벤트 뒤에 온다. 파일 경로는 게임 폴더 밖으로 나갈 수 없다.
    for a in &data.assets {
        check_asset_path(&a.file_path)?;
    }
    for a in &data.audio_assets {
        check_asset_path(&a.base.file_path)?;
    }
    for a in &data.character_assets {
        check_asset_path(&a.base.file_path)?;
    }
    for a in &data.cg_assets {
        check_asset_path(&a.base.file_path)?;
    }
    for a in &data.background_assets {
        check_asset_path(&a.base.file_path)?;
    }
    for a in data.assets {
        upsert_game_asset(&mut **tx, a, game_id).await?;
    }
    // 종류별 정보는 기본 행을 먼저 쓰고, 돌려받은 id(assets_id)로 이어서 쓴다.
    for a in data.audio_assets {
        let id = upsert_game_asset(&mut **tx, a.base.clone(), game_id).await?;
        upsert_audio_asset(&mut **tx, a, &id).await?;
    }
    for a in data.character_assets {
        let id = upsert_game_asset(&mut **tx, a.base.clone(), game_id).await?;
        upsert_character_asset(&mut **tx, a, &id).await?;
    }
    for a in data.cg_assets {
        let id = upsert_game_asset(&mut **tx, a.base.clone(), game_id).await?;
        upsert_cg_asset(&mut **tx, a, &id).await?;
    }
    for a in data.background_assets {
        let id = upsert_game_asset(&mut **tx, a.base.clone(), game_id).await?;
        upsert_background_assets(&mut **tx, a, &id).await?;
    }
    let event_ids: Vec<String> = data.events.iter().filter_map(|e| e.id.clone()).collect();
    for e in data.events {
        upsert_game_event(&mut **tx, game_id, e).await?;
    }
    // items(획득 이벤트 FK)와 interactables(event_id FK)는 events 뒤에 와야 한다.
    for i in data.items {
        upsert_game_item(&mut **tx, i).await?;
    }
    for r in data.relationship_defs {
        upsert_game_relationship_def(&mut **tx, r).await?;
    }
    for i in data.interactables {
        upsert_game_interactables(&mut **tx, i).await?;
    }
    if replace_conditions {
        for event_id in &event_ids {
            delete_game_event_conditions(&mut **tx, event_id).await?;
        }
    }
    for c in data.event_conditions {
        let event_id = c.event_id.clone();
        upsert_game_event_condition(&mut **tx, &event_id, c).await?;
    }
    for p in data.event_character_pool {
        let event_id = p.event_id.clone();
        upsert_game_event_character(&mut **tx, &event_id, p).await?;
    }
    for t in data.encounter_tables {
        upsert_game_encounter_table(&mut **tx, t).await?;
    }
    for e in data.encounter_entries {
        upsert_game_encounter_table_entry(&mut **tx, e).await?;
    }
    for (idx, scene) in data.scenes.iter().enumerate() {
        upsert_game_scene(&mut **tx, game_id, scene, idx as i64).await?;
    }
    // 선택지/분기는 씬 행과 따로 쓴다. 다른 씬을 next_scene_id로 가리키므로(FK) 모든 씬을 쓴 뒤에 온다.
    for scene in &data.scenes {
        for c in scene.choices.iter().flatten() {
            upsert_game_choice(&mut **tx, c).await?;
        }
        for b in scene.branches.iter().flatten() {
            upsert_game_branches(&mut **tx, b).await?;
        }
    }
    for e in data.endings {
        upsert_game_ending(&mut **tx, e).await?;
    }
    Ok(())
}

/// 에셋의 file_path는 게임 폴더 안의 상대경로여야 한다 (절대경로, `..`, 역슬래시, 빈 값 금지).
/// import/저장이 받은 데이터를 그대로 믿지 않는다 — 이 값으로 파일을 읽고 지우기 때문이다.
pub fn check_asset_path(p: &str) -> Result<(), AppError> {
    let bad = p.trim().is_empty()
        || p.starts_with('/')
        || p.contains('\\')
        || p.contains(':')
        || p.split('/').any(|seg| seg == ".." || seg.is_empty());
    if bad {
        return Err(AppError::Validation(format!("에셋 경로가 올바르지 않습니다: {p}")));
    }
    Ok(())
}

/// 지워진 에셋의 파일을 게임 폴더에서 지운다 (실패해도 무시 — DB 쪽이 이미 정리됐다).
pub fn remove_asset_files(games_root: &Path, game_id: &str, rel_paths: &[String]) {
    let base = games_root.join(game_id);
    for p in rel_paths {
        if check_asset_path(p).is_ok() {
            let _ = std::fs::remove_file(base.join(p));
        }
    }
}

fn ids<T>(items: &[T], get: impl for<'a> Fn(&'a T) -> Option<&'a str>) -> HashSet<String> {
    items.iter().filter_map(|i| get(i).map(str::to_string)).collect()
}

/// 에디터가 편집한 정의를 DB에 반영한다. **id를 보존**하는 방식이다:
///   1) 받은 데이터를 id 기준으로 upsert (기존 행은 그대로 갱신 → 세이브/세션/이벤트 로그가 가리키는 id 유지)
///   2) 받은 데이터에 더는 없는 행만 삭제 (에디터에서 지운 것)
/// 전부 한 트랜잭션이다. "통째로 지우고 다시 쓰기"를 안 하는 이유: 세션/세이브/로그가 정의 행의 id를
/// 참조하고 있어서(FK의 ON DELETE SET NULL/CASCADE), 지웠다 쓰면 이어하기 데이터가 끊기거나 지워진다.
///
/// 규칙: 기존 행은 반드시 id를 달아서 보내야 한다 (id 없는 행은 "새 행"으로 취급된다).
/// 반환값은 테이블별 삭제 개수다.
pub async fn save_definition(pool: &SqlitePool, game_id: &str, existing: &GameDefinition, data: PackageData) -> Result<(BTreeMap<&'static str, usize>, Vec<String>), AppError> {
    // 삭제 대상 계산에 쓸 "새 데이터에 남아 있는 id/키"를 write_definition이 데이터를 소비하기 전에 모아둔다.
    let keep_state_defs = ids(&data.state_defs, |x| x.id.as_deref());
    let keep_maps = ids(&data.maps, |x| x.id.as_deref());
    let keep_zones = ids(&data.zones, |x| x.id.as_deref());
    let keep_locations = ids(&data.locations, |x| x.id.as_deref());
    let keep_time_periods = ids(&data.time_periods, |x| x.id.as_deref());
    let keep_weather_types = ids(&data.weather_types, |x| x.id.as_deref());
    let keep_events = ids(&data.events, |x| x.id.as_deref());
    let keep_interactables = ids(&data.interactables, |x| x.id.as_deref());
    let keep_tables = ids(&data.encounter_tables, |x| x.id.as_deref());
    let keep_entries = ids(&data.encounter_entries, |x| x.id.as_deref());
    let keep_endings = ids(&data.endings, |x| x.id.as_deref());
    let keep_characters = ids(&data.characters, |x| x.id.as_deref());
    let keep_items = ids(&data.items, |x| x.id.as_deref());
    // 에셋: 데이터에 더는 없는 것, 그리고 지워질 location/시간대/날씨/캐릭터에 묶인 것은 에셋째로(파일 포함) 지운다.
    // FK가 SET NULL이라 그냥 두면 지워진 location의 배경이 "전체 기본 배경"으로 바뀌어버린다.
    let mut keep_assets: HashSet<String> = HashSet::new();
    keep_assets.extend(data.assets.iter().filter_map(|a| a.id.clone()));
    keep_assets.extend(data.audio_assets.iter().filter_map(|a| a.base.id.clone()));
    keep_assets.extend(data.character_assets.iter().filter_map(|a| a.base.id.clone()));
    keep_assets.extend(data.cg_assets.iter().filter_map(|a| a.base.id.clone()));
    keep_assets.extend(data.background_assets.iter().filter_map(|a| a.base.id.clone()));
    let keep_loc_ids = ids(&data.locations, |x| x.id.as_deref());
    let keep_time_ids = ids(&data.time_periods, |x| x.id.as_deref());
    let keep_weather_ids = ids(&data.weather_types, |x| x.id.as_deref());
    let keep_char_ids = ids(&data.characters, |x| x.id.as_deref());
    let mut dependent_assets: HashSet<String> = HashSet::new();
    for b in &data.background_assets {
        let dangling = b.location_id.as_ref().map_or(false, |l| !keep_loc_ids.contains(l))
            || b.time_id.as_ref().map_or(false, |t| !keep_time_ids.contains(t))
            || b.weather_id.as_ref().map_or(false, |w| !keep_weather_ids.contains(w));
        if dangling {
            dependent_assets.extend(b.base.id.clone());
        }
    }
    for c in &data.character_assets {
        if c.game_character_id.as_ref().map_or(false, |id| !keep_char_ids.contains(id)) {
            dependent_assets.extend(c.base.id.clone());
        }
    }
    let keep_spawns = ids(&data.spawn_locations, |x| x.id.as_deref());
    let keep_rel_defs = ids(&data.relationship_defs, |x| x.id.as_deref());
    let keep_scenes: HashSet<String> = data.scenes.iter().map(|s| s.id.clone()).collect();
    let keep_choices: HashSet<String> = data.scenes.iter().flat_map(|s| s.choices.iter().flatten().map(|c| c.id.clone())).collect();
    let keep_branches: HashSet<String> = data.scenes.iter().flat_map(|s| s.branches.iter().flatten().map(|b| b.id.clone())).collect();
    let keep_transitions: HashSet<(String, String, String)> =
        data.weather_transitions.iter().map(|t| (t.from_weather_id.clone(), t.to_weather_id.clone(), t.map_id.clone())).collect();
    let keep_connections: HashSet<(String, String)> = data.location_connections.iter().map(|c| (c.from_id.clone(), c.to_id.clone())).collect();
    let keep_pool: HashSet<(String, String)> = data.event_character_pool.iter().map(|p| (p.event_id.clone(), p.game_character_id.clone())).collect();

    let mut tx = pool.begin().await?;
    write_definition(&mut tx, game_id, data, true).await?;

    let mut deleted: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut removed_files: Vec<String> = Vec::new();
    // 에셋을 가장 먼저 지운다 (기본 행을 지우면 종류별 하위 행은 CASCADE로 같이 지워진다).
    for a in &existing.assets.all {
        if !keep_assets.contains(&a.id) || dependent_assets.contains(&a.id) {
            if delete_game_asset(&mut *tx, &a.id).await? {
                *deleted.entry("assets").or_default() += 1;
                removed_files.push(a.file_path.clone());
            }
        }
    }
    // 자식 쪽부터 지운다. (FK가 CASCADE/SET NULL이라 순서가 틀려도 대부분 되지만, 명시적으로 의존 순서를 지킨다.)
    macro_rules! prune {
        ($label:expr, $existing:expr, $keep:expr, $delete:path) => {
            for id in $existing {
                if !$keep.contains(id.as_str()) {
                    if $delete(&mut *tx, id.as_str()).await? {
                        *deleted.entry($label).or_default() += 1;
                    }
                }
            }
        };
    }
    prune!("interactables", existing.interactables.values().flatten().map(|i| &i.id), keep_interactables, delete_game_interactables);
    prune!("encounter_entries", existing.encounter_entries.iter().map(|e| &e.id), keep_entries, delete_game_encounter_table_entry);
    prune!("encounter_tables", existing.encounter_tables.iter().map(|t| &t.id), keep_tables, delete_game_encounter_table);
    for (event_id, pool_rows) in &existing.event_npc_pool {
        for p in pool_rows {
            if !keep_pool.contains(&(event_id.clone(), p.game_character_id.clone())) && delete_game_event_character(&mut *tx, event_id, &p.game_character_id).await? {
                *deleted.entry("event_character_pool").or_default() += 1;
            }
        }
    }
    prune!("spawn_locations", existing.spawn_locations.values().flatten().map(|s| &s.id), keep_spawns, delete_game_character_spawn_location);
    prune!("events", existing.events.keys(), keep_events, delete_game_event);
    prune!("items", existing.items.keys(), keep_items, delete_game_item);
    prune!("relationship_defs", existing.relationship_defs.iter().map(|r| &r.id), keep_rel_defs, delete_game_relationship_def);
    for t in &existing.weather_transitions {
        if !keep_transitions.contains(&(t.from_weather_id.clone(), t.to_weather_id.clone(), t.map_id.clone()))
            && delete_game_weather_transition(&mut *tx, &t.from_weather_id, &t.to_weather_id, &t.map_id).await?
        {
            *deleted.entry("weather_transitions").or_default() += 1;
        }
    }
    prune!("weather_types", existing.weather_types.iter().map(|w| &w.id), keep_weather_types, delete_game_weather_type);
    prune!("time_periods", existing.time_periods.iter().map(|p| &p.id), keep_time_periods, delete_game_time_period);
    for c in &existing.location_connections {
        if !keep_connections.contains(&(c.from_id.clone(), c.to_id.clone())) && delete_game_location_connection(&mut *tx, &c.from_id, &c.to_id).await? {
            *deleted.entry("location_connections").or_default() += 1;
        }
    }
    prune!("locations", existing.locations.iter().map(|l| &l.id), keep_locations, delete_game_locations);
    prune!("zones", existing.zones.iter().map(|z| &z.id), keep_zones, delete_game_zone);
    prune!("maps", existing.maps.iter().map(|m| &m.id), keep_maps, delete_game_map);
    let existing_choice_ids: Vec<String> = existing.scenes.iter().flat_map(|s| s.choices.iter().flatten()).map(|c| c.id.clone()).collect();
    prune!("choices", existing_choice_ids, keep_choices, delete_game_choice);
    let existing_branches_ids: Vec<String> = existing.scenes.iter().flat_map(|s| s.branches.iter().flatten()).map(|c| c.id.clone()).collect();
    prune!("branches", existing_branches_ids, keep_branches, delete_game_branches);
    prune!("scenes", existing.scenes.iter().map(|s| &s.id), keep_scenes, delete_game_scene);
    prune!("endings", existing.endings.iter().map(|e| &e.id), keep_endings, delete_game_ending);
    prune!("characters", existing.characters.keys(), keep_characters, delete_game_character);
    prune!("state_defs", existing.state_defs.iter().map(|d| &d.id), keep_state_defs, delete_game_state_def);

    tx.commit().await?;
    Ok((deleted, removed_files))
}

/// 패키지 안의 모든 엔티티 id를 새 uuid로 바꾸고, 그 id를 가리키는 참조도 같이 바꾼다.
/// 안 바꾸면 같은 패키지를 두 번 가져오거나, 내보낸 내 게임을 다시 가져올 때 id(PK)가 충돌해서
/// 이미 있는 게임의 행을 덮어쓴다. (id는 전역 PK인데 game_id만 새로 발급하고 있었다.)
/// 방법: 패키지를 JSON 문자열로 만들어 "옛 id"를 "새 id"로 치환한다. id를 가리키는 곳이 필드(next_scene_id 등)뿐 아니라
/// 문자열로 저장된 JSON(조건/효과)과 "from>to" 형태의 키 안에도 있어서, 따옴표로 둘러싸인 형태와
/// '>' 옆에 붙은 형태, 이스케이프된 형태를 모두 치환한다. 월드인포 북 id는 import가 따로 새로 만들어 이어주므로 건드리지 않는다.
fn remap_ids(data: PackageData) -> Result<PackageData, AppError> {
    let mut old_ids: HashSet<String> = HashSet::new();
    macro_rules! collect {
        ($items:expr) => {
            for i in &$items {
                if let Some(id) = &i.id {
                    old_ids.insert(id.clone());
                }
            }
        };
    }
    collect!(data.state_defs);
    collect!(data.maps);
    collect!(data.zones);
    collect!(data.locations);
    collect!(data.interactables);
    collect!(data.time_periods);
    collect!(data.weather_types);
    collect!(data.events);
    collect!(data.encounter_tables);
    collect!(data.encounter_entries);
    collect!(data.endings);
    collect!(data.characters);
    collect!(data.items);
    collect!(data.relationship_defs);
    collect!(data.spawn_locations);
    for a in &data.assets {
        old_ids.extend(a.id.clone());
    }
    for a in &data.audio_assets {
        old_ids.extend(a.base.id.clone());
    }
    for a in &data.character_assets {
        old_ids.extend(a.base.id.clone());
    }
    for a in &data.cg_assets {
        old_ids.extend(a.base.id.clone());
    }
    for a in &data.background_assets {
        old_ids.extend(a.base.id.clone());
    }
    for s in &data.scenes {
        old_ids.insert(s.id.clone());
        for c in s.choices.iter().flatten() {
            old_ids.insert(c.id.clone());
        }
        for b in s.branches.iter().flatten() {
            old_ids.insert(b.id.clone());
        }
    }
    old_ids.retain(|id| !id.is_empty() && id != "player"); // 'player'는 id가 아니라 예약어

    let mut text = serde_json::to_string(&data)?;
    for old in old_ids {
        let new = uuid::Uuid::new_v4().to_string();
        for (from, to) in [
            (format!("\"{old}\""), format!("\"{new}\"")),                  // "id"
            (format!("\\\"{old}\\\""), format!("\\\"{new}\\\"")),  // 문자열 안의 JSON: \"id\"
            (format!("\"{old}>"), format!("\"{new}>")),                    // "id>other" 관계 키
            (format!(">{old}\""), format!(">{new}\"")),                    // "other>id"
            (format!("\\\"{old}>"), format!("\\\"{new}>")),
            (format!(">{old}\\\""), format!(">{new}\\\"")),
        ] {
            text = text.replace(&from, &to);
        }
    }
    Ok(serde_json::from_str(&text)?)
}

/// 남이 만든 .tgame을 games_root/<새 game_id>/ 에 풀고, data/definition.json을
/// 파싱해서 db::game의 upsert_* 함수들로 밀어넣는다. 전부 한 트랜잭션으로 묶어서
/// 중간에 실패하면 아무것도 안 남게 한다.
/// 템플릿으로 새 게임을 만들 때 가져온 프로젝트에 덮어쓸 값.
pub struct ImportOverrides {
    pub title: String,
    pub template_id: String,
}

pub async fn import(pool: &SqlitePool, games_root: &Path, zip_path: &Path) -> Result<String, AppError> {
    let file = std::fs::File::open(zip_path)?;
    if file.metadata()?.len() > MAX_PACKAGE_BYTES {
        return Err(AppError::Validation("패키지 크기가 너무 큽니다".into()));
    }
    let archive = zip::ZipArchive::new(file).map_err(|e| AppError::Archive(e.to_string()))?;
    import_archive(pool, games_root, archive, None).await
}

/// 메모리에 있는 패키지(번들된 템플릿 패키지)를 가져온다. 파일 import와 같은 경로를 탄다.
pub async fn import_bytes(pool: &SqlitePool, games_root: &Path, bytes: &[u8], overrides: Option<ImportOverrides>) -> Result<String, AppError> {
    if bytes.len() as u64 > MAX_PACKAGE_BYTES {
        return Err(AppError::Validation("패키지 크기가 너무 큽니다".into()));
    }
    let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| AppError::Archive(e.to_string()))?;
    import_archive(pool, games_root, archive, overrides).await
}

/// 새 game_id로 폴더를 만들고 import한다. 중간에 실패하면 만들다 만 폴더를 지운다 (DB는 트랜잭션이라 안 남는다).
async fn import_archive<R: std::io::Read + std::io::Seek>(pool: &SqlitePool, games_root: &Path, mut archive: zip::ZipArchive<R>, overrides: Option<ImportOverrides>) -> Result<String, AppError> {
    let new_game_id = uuid::Uuid::new_v4().to_string();
    let dest = games_root.join(&new_game_id);
    std::fs::create_dir_all(&dest)?;
    match import_into(pool, &mut archive, &dest, new_game_id, overrides).await {
        Ok(id) => Ok(id),
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dest);
            Err(e)
        }
    }
}

async fn import_into<R: std::io::Read + std::io::Seek>(pool: &SqlitePool, archive: &mut zip::ZipArchive<R>, dest: &Path, new_game_id: String, overrides: Option<ImportOverrides>) -> Result<String, AppError> {

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| AppError::Archive(e.to_string()))?;
        let Some(rel_path) = entry.enclosed_name() else {
            return Err(AppError::Archive("잘못된 경로가 포함된 패키지입니다".into()));
        };
        let out_path = dest.join(rel_path);
        if !out_path.starts_with(&dest) {
            return Err(AppError::Archive("경로 탈출이 감지됐습니다".into()));
        }
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }

    let manifest_raw = std::fs::read_to_string(dest.join("manifest.json")).map_err(|_| AppError::Validation("manifest.json이 없습니다".into()))?;
    let _manifest: Manifest = serde_json::from_str(&manifest_raw)?; // TODO: min_engine_version 체크

    let data_raw = std::fs::read_to_string(dest.join("data/definition.json")).map_err(|_| AppError::Validation("data/definition.json이 없습니다".into()))?;
    let mut data: PackageData = serde_json::from_str(&data_raw)?;
    data = remap_ids(data)?;
    if let Some(o) = &overrides {
        data.project.title = o.title.clone();
        data.project.template_id = o.template_id.clone();
    }

    // game_id를 전부 새로 발급받은 걸로 덮어쓴다. 내부 엔티티 id(맵/씬/노드 등)는 원본 그대로 둔다 —
    // 그래야 next_node_id 같은 상호 참조가 깨지지 않는다.
    data.project.id = Some(new_game_id.clone());
    for m in &mut data.maps {
        m.game_id = new_game_id.clone();
    }
    for p in &mut data.time_periods {
        p.game_id = new_game_id.clone();
    }
    for w in &mut data.weather_types {
        w.game_id = new_game_id.clone();
    }
    for e in &mut data.events {
        e.game_id = new_game_id.clone();
    }
    for e in &mut data.endings {
        e.game_id = new_game_id.clone();
    }
    for c in &mut data.characters {
        c.game_id = new_game_id.clone();
    }
    // zones(map_id로 스코프), weather_transitions/encounter_tables/encounter_entries(map_id
    // 내장), event_conditions/event_character_pool(event_id 내장)은 game_id를 직접 안 들고
    // 있어서 손 안 댐 — 위에서 game_id가 바뀐 부모 엔티티들의 (불변인) id를 그대로 참조한다.

    let mut tx = pool.begin().await?;

    // 월드인포 북은 전역 테이블이라 id가 충돌할 수 있다 — 북마다 새로 만들고 {원본 id → 새 id}로
    // project/characters의 world_info_ids를 다시 이어준다. 패키지에 없는 id(dangling)는 버린다.
    let mut book_id_map: HashMap<String, String> = HashMap::new();
    for pb in std::mem::take(&mut data.world_info_books) {
        let created = create_worldinfo_book(&mut *tx, pb.book).await?;
        for mut entry in pb.entries {
            entry.id = None;
            entry.book_id = created.id.clone();
            upsert_worldinfo_entry(&mut *tx, entry).await?;
        }
        book_id_map.insert(pb.id, created.id);
    }
    let remap = |ids: &[String]| -> Vec<String> { ids.iter().filter_map(|id| book_id_map.get(id).cloned()).collect() };
    data.project.world_info_ids = remap(&data.project.world_info_ids);
    for c in &mut data.characters {
        c.world_info_ids = remap(&c.world_info_ids);
    }

    write_definition(&mut tx, &new_game_id, data, false).await?;

    tx.commit().await?;
    Ok(new_game_id)
}

/// GameDefinition(=정의 데이터 전체)과 games_root/<game_id>/의 scripts·assets를 묶어서
/// output_dir에 .tgame으로 저장하고 경로를 반환한다. 세이브/세션/로그/대화/API 키는
/// 절대 포함 안 함 — GameDefinition 자체가 정적 데이터만 들고 있어서 자연히 보장된다.
pub async fn export(pool: &SqlitePool, games_root: &Path, game_id: &str, definition: &GameDefinition, output_dir: &Path) -> Result<PathBuf, AppError> {
    let src = games_root.join(game_id);
    if !src.exists() {
        return Err(AppError::NotFound("게임 파일을 찾을 수 없습니다".into()));
    }
    std::fs::create_dir_all(output_dir)?;
    let out_path = unique_path(output_dir, &sanitize_filename(&definition.project.title), "tgame");

    let file = std::fs::File::create(&out_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    let manifest = Manifest {
        format_version: "1".into(),
        title: definition.project.title.clone(),
        author: None, // TODO: 작성자 정보를 어디서 가져올지 (앱 프로필? 설정?)
        min_engine_version: env!("CARGO_PKG_VERSION").into(),
    };
    zip.start_file("manifest.json", options).map_err(|e| AppError::Archive(e.to_string()))?;
    std::io::Write::write_all(&mut zip, serde_json::to_string_pretty(&manifest)?.as_bytes())?;

    let world_info_books = collect_world_info_books(pool, definition).await?;
    let package_data = to_package_data(definition, world_info_books);
    zip.start_file("data/definition.json", options).map_err(|e| AppError::Archive(e.to_string()))?;
    std::io::Write::write_all(&mut zip, serde_json::to_string_pretty(&package_data)?.as_bytes())?;

    add_dir_to_zip(&mut zip, &src, &src, options)?;

    zip.finish().map_err(|e| AppError::Archive(e.to_string()))?;
    Ok(out_path)
}

/// base 기준 상대경로를 유지하면서 dir 아래 모든 파일을 재귀적으로 zip에 담는다.
/// manifest.json / data/는 games/<id>/ 디렉터리 안에 있을 리 없으니 이름 충돌 걱정은 안 해도 된다
/// (그 둘은 export()가 별도로 썼고, games/<id>/는 index.html, scripts/, assets/만 있는 디렉터리).
fn add_dir_to_zip<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    base: &Path,
    dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<(), AppError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).expect("base 하위 경로여야 함").to_string_lossy().replace('\\', "/");
        // 중복 추가 방지용
        if rel == "manifest.json" || rel == "data/definition.json" {
            continue;
        }
        if path.is_dir() {
            zip.add_directory(format!("{rel}/"), options).map_err(|e| AppError::Archive(e.to_string()))?;
            add_dir_to_zip(zip, base, &path, options)?;
        } else {
            zip.start_file(rel, options).map_err(|e| AppError::Archive(e.to_string()))?;
            let bytes = std::fs::read(&path)?;
            std::io::Write::write_all(zip, &bytes)?;
        }
    }
    Ok(())
}

/// 같은 이름이 이미 있으면 "이름 (1).tgame"처럼 번호를 붙인다 (다운로드 폴더의 기존 파일을 덮어쓰지 않게).
fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{ext}"));
    if !first.exists() {
        return first;
    }
    (1..10_000).map(|n| dir.join(format!("{stem} ({n}).{ext}"))).find(|p| !p.exists()).unwrap_or(first)
}

fn sanitize_filename(name: &str) -> String {
    name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}
