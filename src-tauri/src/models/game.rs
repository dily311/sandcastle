use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::game::runtime::GameState;
// =========================================================
// DB에서 가져오는 데이터 정의
// =========================================================
// ---------------------------------------------------------
// 게임 프로젝트
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameProjectRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub template_id: String,
    pub version: String,
    pub api_version: String,
    pub cover_image: Option<String>,
    pub start_scene_key: Option<String>,
    pub start_map_id: Option<String>,
    pub start_location_id: Option<String>,
    pub world_info_ids: Option<String>,
    pub custom_css: Option<String>,
    pub settings_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameProject {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub template_id: String,
    pub version: String,
    pub api_version: String,
    pub cover_image: Option<String>,
    pub start_scene_key: Option<String>,
    pub start_map_id: Option<String>,
    pub start_location_id: Option<String>,
    pub world_info_ids: Vec<String>,
    pub custom_css: Option<String>,
    pub settings_json: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<GameProjectRow> for GameProject {
    fn from(r: GameProjectRow) -> Self {
        Self {
            id: r.id,
            title: r.title,
            description: r.description,
            template_id: r.template_id,
            version: r.version,
            api_version: r.api_version,
            cover_image: r.cover_image,
            start_scene_key: r.start_scene_key,
            start_map_id: r.start_map_id,
            start_location_id: r.start_location_id,
            world_info_ids: r
                .world_info_ids
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            custom_css: r.custom_css,
            settings_json: r.settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameProjectInput {
    pub id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub template_id: String,
    pub version: String,
    pub api_version: String,
    pub cover_image: Option<String>,
    pub start_scene_key: Option<String>,
    pub start_map_id: Option<String>,
    pub start_location_id: Option<String>,
    pub world_info_ids: Vec<String>,
    pub custom_css: Option<String>,
    pub settings_json: Option<String>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

impl From<GameProject> for GameProjectInput {
    fn from(v: GameProject) -> Self {
        Self {
            id: Some(v.id),
            title: v.title,
            description: v.description,
            template_id: v.template_id,
            version: v.version,
            api_version: v.api_version,
            cover_image: v.cover_image,
            start_scene_key: v.start_scene_key,
            start_map_id: v.start_map_id,
            start_location_id: v.start_location_id,
            world_info_ids: v.world_info_ids,
            custom_css: v.custom_css,
            settings_json: v.settings_json.and_then(|s| serde_json::to_string(&s).ok()),
            created_at: Some(v.created_at),
            updated_at: Some(v.updated_at),
        }
    }
}

impl GameProjectInput {
    pub fn default(id: Option<String>, title: String, template_id: String) -> Self {
        Self {
            id,
            title,
            description: None,
            template_id,
            version: "1.0".into(),
            api_version: "1.0".into(),
            cover_image: None,
            start_scene_key: None,
            start_map_id: None,
            start_location_id: None,
            world_info_ids: vec![],
            custom_css: None,
            settings_json: None,
            created_at: None,
            updated_at: None,
        }
    }
}

// ---------------------------------------------------------
// 게임 변수
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameStateDefRow {
    pub id: String,
    pub game_id: String,
    pub description: Option<String>,
    pub key: String,
    #[sqlx(rename = "type")]
    pub def_type: String,
    pub default_value: String,
    pub config_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameStateDef {
    pub id: String,
    pub game_id: String,
    pub description: Option<String>,
    pub key: String,
    #[serde(rename = "type")]
    pub def_type: String,
    pub default_value: String,
    // 형식: { "ai_readable": true, "ai_writable": true, "min": 0, "max": 100, "max_step": 3 }
    pub config_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<GameStateDefRow> for GameStateDef {
    fn from(r: GameStateDefRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            description: r.description,
            key: r.key,
            def_type: r.def_type,
            default_value: r.default_value,
            config_json: r.config_json,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameStateDefInput {
    pub id: Option<String>,
    pub description: Option<String>,
    pub key: String,
    #[serde(rename = "type")]
    pub def_type: String,
    pub default_value: String,
    pub config_json: String,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

impl From<GameStateDef> for GameStateDefInput {
    fn from(v: GameStateDef) -> Self {
        Self {
            id: Some(v.id),
            description: v.description,
            key: v.key,
            def_type: v.def_type,
            default_value: v.default_value,
            config_json: v.config_json,
            created_at: Some(v.created_at),
            updated_at: Some(v.updated_at),
        }
    }
}

// ---------------------------------------------------------
// 게임 캐릭터
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameCharactersRow {
    pub id: String,
    pub game_id: String,
    pub character_id: Option<String>,
    pub name: String,
    pub avatar: Option<String>,
    pub description: Option<String>,
    pub first_message: Option<String>,
    pub example_dialogue: Option<String>,
    pub system_prompt_override: Option<String>,
    pub behavior_type: Option<String>,
    pub world_info_ids: Option<String>,
    pub tags: Option<String>,
    pub settings_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameCharacters {
    pub id: String,
    pub game_id: String,
    pub character_id: Option<String>,
    pub name: String,
    pub avatar: Option<String>,
    pub description: Option<String>,
    pub first_message: Option<String>,
    pub example_dialogue: Option<String>,
    pub system_prompt_override: Option<String>,
    pub behavior_type: Option<String>,
    pub world_info_ids: Vec<String>,
    pub tags: Option<Vec<String>>,
    pub settings_json: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<GameCharactersRow> for GameCharacters {
    fn from(r: GameCharactersRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            character_id: r.character_id,
            name: r.name,
            avatar: r.avatar,
            description: r.description,
            first_message: r.first_message,
            example_dialogue: r.example_dialogue,
            system_prompt_override: r.system_prompt_override,
            behavior_type: r.behavior_type,
            world_info_ids: r
                .world_info_ids
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            tags: r.tags.and_then(|s| serde_json::from_str(&s).ok()),
            settings_json: r
                .settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameCharactersInput {
    pub id: Option<String>,
    pub game_id: String,
    pub character_id: Option<String>,
    pub name: String,
    pub avatar: Option<String>,
    pub description: Option<String>,
    pub first_message: Option<String>,
    pub example_dialogue: Option<String>,
    pub system_prompt_override: Option<String>,
    pub behavior_type: Option<String>,
    pub world_info_ids: Vec<String>,
    pub tags: Option<String>,
    pub settings_json: serde_json::Value,
}
impl From<GameCharacters> for GameCharactersInput {
    fn from(v: GameCharacters) -> Self {
        Self {
            id: Some(v.id),
            game_id: v.game_id,
            character_id: v.character_id,
            name: v.name,
            avatar: v.avatar,
            description: v.description,
            first_message: v.first_message,
            example_dialogue: v.example_dialogue,
            system_prompt_override: v.system_prompt_override,
            world_info_ids: v.world_info_ids,
            behavior_type: v.behavior_type,
            tags: v.tags.and_then(|s| serde_json::to_string(&s).ok()),
            settings_json: v.settings_json,
        }
    }
}
// ---------- game_character_spawn_locations ----------
#[derive(FromRow)]
pub struct GameCharacterSpawnLocationsRow {
    pub id: String,
    pub game_character_id: String,
    pub location_id: String,
    pub spawn_weight: i64,
    pub condition_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameCharacterSpawnLocations {
    pub id: String,
    pub game_character_id: String,
    pub location_id: String,
    pub spawn_weight: i64,
    pub condition_json: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}
impl From<GameCharacterSpawnLocationsRow> for GameCharacterSpawnLocations {
    fn from(r: GameCharacterSpawnLocationsRow) -> Self {
        Self {
            id: r.id,
            game_character_id: r.game_character_id,
            location_id: r.location_id,
            spawn_weight: r.spawn_weight,
            condition_json: r
                .condition_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameCharacterSpawnLocationsInput {
    pub id: Option<String>,
    pub game_character_id: String,
    pub location_id: String,
    pub spawn_weight: i64,
    pub condition_json: serde_json::Value,
}

impl From<GameCharacterSpawnLocations> for GameCharacterSpawnLocationsInput {
    fn from(v: GameCharacterSpawnLocations) -> Self {
        Self {
            id: Some(v.id),
            game_character_id: v.game_character_id,
            location_id: v.location_id,
            spawn_weight: v.spawn_weight,
            condition_json: v.condition_json,
        }
    }
}

// ---------------------------------------------------------
// 게임 에셋
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameAssetsRow {
    pub id: String,
    pub game_id: String,
    pub key: String,
    pub name: String,
    pub category: String,
    pub kind: String,
    pub file_path: String,
    pub settings_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameAssets {
    pub id: String,
    pub game_id: String,
    pub key: String,
    pub name: String,
    pub category: String,
    pub kind: String,
    pub file_path: String,
    pub settings_json: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}
impl From<GameAssetsRow> for GameAssets {
    fn from(r: GameAssetsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            key: r.key,
            name: r.name,
            category: r.category,
            kind: r.kind,
            file_path: r.file_path,
            settings_json: r
                .settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
impl From<AudioAssetsRow> for GameAssets {
    fn from(r: AudioAssetsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            key: r.key,
            name: r.name,
            category: r.category,
            kind: r.kind,
            file_path: r.file_path,
            settings_json: r
                .settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
impl From<CharacterAssetsRow> for GameAssets {
    fn from(r: CharacterAssetsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            key: r.key,
            name: r.name,
            category: r.category,
            kind: r.kind,
            file_path: r.file_path,
            settings_json: r
                .settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
impl From<CgAssetsRow> for GameAssets {
    fn from(r: CgAssetsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            key: r.key,
            name: r.name,
            category: r.category,
            kind: r.kind,
            file_path: r.file_path,
            settings_json: r
                .settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
impl From<BackgroundAssetsRow> for GameAssets {
    fn from(r: BackgroundAssetsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            key: r.key,
            name: r.name,
            category: r.category,
            kind: r.kind,
            file_path: r.file_path,
            settings_json: r
                .settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
#[derive(Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameAssetsInput {
    pub id: Option<String>,
    pub game_id: String,
    pub key: String,
    pub name: String,
    pub category: String,
    pub kind: String,
    pub file_path: String,
    pub settings_json: Option<serde_json::Value>,
}

impl From<GameAssets> for GameAssetsInput {
    fn from(r: GameAssets) -> Self {
        Self {
            id: Some(r.id),
            game_id: r.game_id,
            key: r.key,
            name: r.name,
            category: r.category,
            kind: r.kind,
            file_path: r.file_path,
            settings_json: r.settings_json,
        }
    }
}

#[derive(FromRow, Clone)]
pub struct AudioAssetsRow {
    pub id: String,
    pub game_id: String,
    pub key: String,
    pub name: String,
    pub category: String,
    pub kind: String,
    pub file_path: String,
    pub settings_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub tag: Option<String>,
    pub game_character_id: Option<String>,
    pub transcript: Option<String>,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AudioAssets {
    #[serde(flatten)]
    pub base: GameAssets,
    pub tag: Option<String>,
    pub game_character_id: Option<String>,
    pub transcript: Option<String>,
}

impl From<AudioAssetsRow> for AudioAssets {
    fn from(r: AudioAssetsRow) -> Self {
        Self {
            base: r.clone().into(),
            tag: r.tag,
            game_character_id: r.game_character_id,
            transcript: r.transcript,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AudioAssetsInput {
    #[serde(flatten)]
    pub base: GameAssetsInput,
    pub tag: Option<String>,
    pub game_character_id: Option<String>,
    pub transcript: Option<String>,
}

impl From<AudioAssets> for AudioAssetsInput {
    fn from(r: AudioAssets) -> Self {
        Self {
            base:r.base.into(),
            tag: r.tag,
            game_character_id: r.game_character_id,
            transcript: r.transcript,
        }
    }
}

#[derive(FromRow, Clone)]
pub struct CharacterAssetsRow {
    pub id: String,
    pub game_id: String,
    pub key: String,
    pub name: String,
    pub category: String,
    pub kind: String,
    pub file_path: String,
    pub settings_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub game_character_id: Option<String>,
    pub usage: String,
    pub emotion: Option<String>,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CharacterAssets {
    #[serde(flatten)]
    pub base: GameAssets,

    pub game_character_id: Option<String>,
    pub usage: String,
    pub emotion: Option<String>,
}

impl From<CharacterAssetsRow> for CharacterAssets {
    fn from(r: CharacterAssetsRow) -> Self {
        Self {
            base: r.clone().into(),
            game_character_id: r.game_character_id,
            usage: r.usage,
            emotion: r.emotion,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CharacterAssetsInput {
    #[serde(flatten)]
    pub base: GameAssetsInput,

    pub game_character_id: Option<String>,
    pub usage: String,
    pub emotion: Option<String>,
}

impl From<CharacterAssets> for CharacterAssetsInput {
    fn from(r: CharacterAssets) -> Self {
        Self {
            base: r.base.into(),
            game_character_id: r.game_character_id,
            usage: r.usage,
            emotion: r.emotion,
        }
    }
}

#[derive(FromRow, Clone)]
pub struct CgAssetsRow {
    pub id: String,
    pub game_id: String,
    pub key: String,
    pub name: String,
    pub category: String,
    pub kind: String,
    pub file_path: String,
    pub settings_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub game_character_id: Option<String>,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub seed: Option<i64>,
    pub generation_params: Option<String>,
    pub ai_usable: bool,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CgAssets {
    #[serde(flatten)]
    pub base: GameAssets,

    pub game_character_id: Option<String>,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub seed: Option<i64>,
    pub generation_params: Option<String>,
    pub ai_usable: bool,
}

impl From<CgAssetsRow> for CgAssets {
    fn from(r: CgAssetsRow) -> Self {
        Self {
            base: r.clone().into(),
            game_character_id: r.game_character_id,
            description: r.description,
            prompt: r.prompt,
            negative_prompt: r.negative_prompt,
            seed: r.seed,
            generation_params: r.generation_params,
            ai_usable: r.ai_usable,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CgAssetsInput {
    #[serde(flatten)]
    pub base: GameAssetsInput,

    pub game_character_id: Option<String>,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub seed: Option<i64>,
    pub generation_params: Option<String>,
    pub ai_usable: bool,
}

impl From<CgAssets> for CgAssetsInput {
    fn from(r: CgAssets) -> Self {
        Self {
            base: r.base.into(),
            game_character_id: r.game_character_id,
            description: r.description,
            prompt: r.prompt,
            negative_prompt: r.negative_prompt,
            seed: r.seed,
            generation_params: r.generation_params,
            ai_usable: r.ai_usable,
        }
    }
}
#[derive(FromRow, Clone)]
pub struct BackgroundAssetsRow {
    pub id: String,
    pub game_id: String,
    pub key: String,
    pub name: String,
    pub category: String,
    pub kind: String,
    pub file_path: String,
    pub settings_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub location_id: Option<String>,
    pub time_id: Option<String>,
    pub weather_id: Option<String>,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundAssets {
    #[serde(flatten)]
    pub base: GameAssets,

    pub location_id: Option<String>,
    pub time_id: Option<String>,
    pub weather_id: Option<String>,
}

impl From<BackgroundAssetsRow> for BackgroundAssets {
    fn from(r: BackgroundAssetsRow) -> Self {
        Self {
            base: r.clone().into(),
            location_id: r.location_id,
            time_id: r.time_id,
            weather_id: r.weather_id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundAssetsInput {
    #[serde(flatten)]
    pub base: GameAssetsInput,

    pub location_id: Option<String>,
    pub time_id: Option<String>,
    pub weather_id: Option<String>,
}

impl From<BackgroundAssets> for BackgroundAssetsInput {
    fn from(r: BackgroundAssets) -> Self {
        Self {
            base: r.base.into(),
            location_id: r.location_id,
            time_id: r.time_id,
            weather_id: r.weather_id,
        }
    }
}
// ---------------------------------------------------------
// 게임 맵
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameMapsRow {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub danger_level: i64,
    pub default_weather_pool_id: Option<String>,
    pub settings_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameMaps {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub danger_level: i64,
    pub default_weather_pool_id: Option<String>,
    pub settings_json: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<GameMapsRow> for GameMaps {
    fn from(r: GameMapsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            name: r.name,
            danger_level: r.danger_level,
            default_weather_pool_id: r.default_weather_pool_id,
            settings_json: r
                .settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameMapsInput {
    pub id: Option<String>,
    pub game_id: String,
    pub name: String,
    pub danger_level: i64,
    pub default_weather_pool_id: Option<String>,
    pub settings_json: serde_json::Value,
}

impl From<GameMaps> for GameMapsInput {
    fn from(v: GameMaps) -> Self {
        Self {
            id: Some(v.id),
            game_id: v.game_id,
            name: v.name,
            danger_level: v.danger_level,
            default_weather_pool_id: v.default_weather_pool_id,
            settings_json: v.settings_json,
        }
    }
}

// ----- GameMapsZoneRow ---------
#[derive(FromRow)]
pub struct GameMapsZoneRow {
    pub id: String,
    pub map_id: String,
    pub name: String,
    pub tags: Option<String>,
    pub settings_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameMapsZone {
    pub id: String,
    pub map_id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub settings_json: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<GameMapsZoneRow> for GameMapsZone {
    fn from(r: GameMapsZoneRow) -> Self {
        Self {
            id: r.id,
            map_id: r.map_id,
            name: r.name,
            tags: r
                .tags
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default(),
            settings_json: r
                .settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameMapsZoneInput {
    pub id: Option<String>,
    pub map_id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub settings_json: serde_json::Value,
}

impl From<GameMapsZone> for GameMapsZoneInput {
    fn from(v: GameMapsZone) -> Self {
        Self {
            id: Some(v.id),
            map_id: v.map_id,
            name: v.name,
            tags: v.tags,
            settings_json: v.settings_json,
        }
    }
}

// --- GameLocations ---
#[derive(FromRow)]
pub struct GameLocationsRow {
    pub id: String,
    pub zone_id: String,
    pub name: String,
    pub description: Option<String>,
    pub settings_json: Option<String>,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameLocations {
    pub id: String,
    pub zone_id: String,
    pub name: String,
    pub description: Option<String>,
    pub settings_json: serde_json::Value,
}

impl From<GameLocationsRow> for GameLocations {
    fn from(r: GameLocationsRow) -> Self {
        Self {
            id: r.id,
            zone_id: r.zone_id,
            name: r.name,
            description: r.description,
            settings_json: r
                .settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameLocationsInput {
    pub id: Option<String>,
    pub zone_id: String,
    pub name: String,
    pub description: Option<String>,
    pub settings_json: serde_json::Value,
}

impl From<GameLocations> for GameLocationsInput {
    fn from(r: GameLocations) -> Self {
        Self {
            id: Some(r.id),
            zone_id: r.zone_id,
            name: r.name,
            description: r.description,
            settings_json: r.settings_json,
        }
    }
}

// --- GameLocationConnections ---
#[derive(FromRow)]
pub struct GameLocationConnectionsRow {
    pub from_id: String,
    pub to_id: String,
    pub travel_seconds: u32,
    pub condition: Option<String>,
    pub settings_json: Option<String>,
}
#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameLocationConnections {
    pub from_id: String,
    pub to_id: String,
    pub travel_seconds: u32,
    pub condition: Option<String>,
    pub settings_json: serde_json::Value,
}

impl From<GameLocationConnectionsRow> for GameLocationConnections {
    fn from(r: GameLocationConnectionsRow) -> Self {
        Self {
            from_id: r.from_id,
            to_id: r.to_id,
            travel_seconds: r.travel_seconds,
            condition: r.condition,
            settings_json: r
                .settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameLocationConnectionsInput {
    pub from_id: String,
    pub to_id: String,
    pub travel_seconds: u32,
    pub condition: Option<String>,
    pub settings_json: serde_json::Value,
}

impl From<GameLocationConnections> for GameLocationConnectionsInput {
    fn from(r: GameLocationConnections) -> Self {
        Self {
            from_id: r.from_id,
            to_id: r.to_id,
            travel_seconds: r.travel_seconds,
            condition: r.condition,
            settings_json: r.settings_json,
        }
    }
}
// ---------------------------------------------------------
// 게임 시간
// ---------------------------------------------------------
#[derive(FromRow, Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameTimePeriods {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub sort_order: i64,
    pub start_minute: u32,
    pub end_minute: u32,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameTimePeriodsInput {
    pub id: Option<String>,
    pub game_id: String,
    pub name: String,
    pub sort_order: i64,
    pub start_minute: u32,
    pub end_minute: u32,
}

impl From<GameTimePeriods> for GameTimePeriodsInput {
    fn from(v: GameTimePeriods) -> Self {
        Self {
            id: Some(v.id),
            game_id: v.game_id,
            name: v.name,
            sort_order: v.sort_order,
            start_minute: v.start_minute,
            end_minute: v.end_minute,
        }
    }
}
// ---------------------------------------------------------
// 게임 날씨
// ---------------------------------------------------------

#[derive(FromRow)]
pub struct GameWeatherTypesRow {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub encounter_rate_modifier: f64,
    pub visibility_modifier: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameWeatherTypes {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub encounter_rate_modifier: f64,
    pub visibility_modifier: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<GameWeatherTypesRow> for GameWeatherTypes {
    fn from(r: GameWeatherTypesRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            name: r.name,
            encounter_rate_modifier: r.encounter_rate_modifier,
            visibility_modifier: r.visibility_modifier,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameWeatherTypesInput {
    pub id: Option<String>,
    pub game_id: String,
    pub name: String,
    pub encounter_rate_modifier: f64,
    pub visibility_modifier: f64,
}
impl From<GameWeatherTypes> for GameWeatherTypesInput {
    fn from(v: GameWeatherTypes) -> Self {
        Self {
            id: Some(v.id),
            game_id: v.game_id,
            name: v.name,
            encounter_rate_modifier: v.encounter_rate_modifier,
            visibility_modifier: v.visibility_modifier,
        }
    }
}
// --- GameWeatherPools ---
#[derive(FromRow)]
pub struct GameWeatherPoolsRow {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameWeatherPools {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<GameWeatherPoolsRow> for GameWeatherPools {
    fn from(r: GameWeatherPoolsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            name: r.name,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameWeatherPoolsInput {
    pub id: Option<String>,
    pub game_id: String,
    pub name: String,
}

impl From<GameWeatherPools> for GameWeatherPoolsInput {
    fn from(r: GameWeatherPools) -> Self {
        Self {
            id: Some(r.id),
            game_id: r.game_id,
            name: r.name,
        }
    }
}

// --- GameWeatherPoolEntries ---
#[derive(FromRow)]
pub struct GameWeatherPoolEntriesRow {
    pub pool_id: String,
    pub weather_id: String,
    pub weight: i64,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameWeatherPoolEntries {
    pub pool_id: String,
    pub weather_id: String,
    pub weight: i64,
}
impl From<GameWeatherPoolEntriesRow> for GameWeatherPoolEntries {
    fn from(r: GameWeatherPoolEntriesRow) -> Self {
        Self {
            pool_id: r.pool_id,
            weather_id: r.weather_id,
            weight: r.weight,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameWeatherPoolEntriesInput {
    pub pool_id: String,
    pub weather_id: String,
    pub weight: i64,
}

impl From<GameWeatherPoolEntries> for GameWeatherPoolEntriesInput {
    fn from(r: GameWeatherPoolEntries) -> Self {
        Self {
            pool_id: r.pool_id,
            weather_id: r.weather_id,
            weight: r.weight,
        }
    }
}
// --- GameWeatherTransitions ---
#[derive(FromRow)]
pub struct GameWeatherTransitionsRow {
    pub from_weather_id: String,
    pub to_weather_id: String,
    pub map_id: String,
    pub probability: f64,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameWeatherTransitions {
    pub from_weather_id: String,
    pub to_weather_id: String,
    pub map_id: String,
    pub probability: f64,
}

impl From<GameWeatherTransitionsRow> for GameWeatherTransitions {
    fn from(r: GameWeatherTransitionsRow) -> Self {
        Self {
            from_weather_id: r.from_weather_id,
            to_weather_id: r.to_weather_id,
            map_id: r.map_id,
            probability: r.probability,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameWeatherTransitionsInput {
    pub from_weather_id: String,
    pub to_weather_id: String,
    pub map_id: String,
    pub probability: f64,
}
impl From<GameWeatherTransitions> for GameWeatherTransitionsInput {
    fn from(v: GameWeatherTransitions) -> Self {
        Self {
            from_weather_id: v.from_weather_id,
            to_weather_id: v.to_weather_id,
            map_id: v.map_id,
            probability: v.probability,
        }
    }
}
// ---------------------------------------------------------
// 게임 이벤트
// ---------------------------------------------------------
// --- GameEvents ---
#[derive(FromRow)]
pub struct GameEventsRow {
    pub id: String,
    pub game_id: String,
    pub event_type: String,
    pub trigger_type: String,
    pub repeat_policy: String,
    pub name: String,
    pub description: String,
    pub requires_npc: bool,
    pub config_json: Option<String>,
    pub effect_json: Option<String>,
    pub cooldown_seconds: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEvents {
    pub id: String,
    pub game_id: String,
    pub event_type: String,
    pub trigger_type: String,
    pub repeat_policy: String,
    pub name: String,
    pub description: String,
    pub requires_npc: bool,
    pub config_json: serde_json::Value,
    pub effect_json: serde_json::Value,
    pub cooldown_seconds: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<GameEventsRow> for GameEvents {
    fn from(r: GameEventsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            event_type: r.event_type,
            trigger_type: r.trigger_type,
            repeat_policy: r.repeat_policy,
            name: r.name,
            description: r.description,
            requires_npc: r.requires_npc,
            config_json: r
                .config_json
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default(),
            effect_json: r
                .effect_json
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default(),
            cooldown_seconds: r.cooldown_seconds,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEventsInput {
    pub id: Option<String>,
    pub game_id: String,
    pub event_type: String,
    pub trigger_type: String,
    pub repeat_policy: String,
    pub name: String,
    pub description: String,
    pub requires_npc: bool,
    pub config_json: serde_json::Value,
    pub effect_json: serde_json::Value,
    pub cooldown_seconds: i64,
}

impl From<GameEvents> for GameEventsInput {
    fn from(v: GameEvents) -> Self {
        Self {
            id: Some(v.id),
            game_id: v.game_id,
            event_type: v.event_type,
            name: v.name,
            description: v.description,
            requires_npc: v.requires_npc,
            config_json: v.config_json,
            effect_json: v.effect_json,
            cooldown_seconds: v.cooldown_seconds,
            trigger_type: v.trigger_type,
            repeat_policy: v.repeat_policy,
        }
    }
}

// --- GameEventConditions ---
#[derive(FromRow)]
pub struct GameEventConditionsRow {
    pub id: String,
    pub event_id: String,
    pub condition_type: String,
    pub condition_value: Option<String>,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEventConditions {
    pub id: String,
    pub event_id: String,
    pub condition_type: String,
    pub condition_value: Option<String>,
}
impl From<GameEventConditionsRow> for GameEventConditions {
    fn from(r: GameEventConditionsRow) -> Self {
        Self {
            id: r.id,
            event_id: r.event_id,
            condition_type: r.condition_type,
            condition_value: r.condition_value,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEventConditionsInput {
    pub id: Option<String>,
    pub event_id: String,
    pub condition_type: String,
    pub condition_value: Option<String>,
}

impl GameEventConditionsInput {
    pub fn from_condition<T: serde::Serialize>(event_id: &str, c: &T) -> Option<Self> {
        let value = serde_json::to_value(c).ok()?;
        let condition_type = value.get("type")?.as_str()?.to_string();
        let condition_value = serde_json::to_string(&value).ok();

        Some(Self {
            id: None,
            event_id: event_id.to_string(),
            condition_type,
            condition_value,
        })
    }
}
// --- game_interactables ---
#[derive(FromRow)]
pub struct GameInteractablesRow {
    pub id: String,
    pub location_id: String,
    pub name: String,
    pub event_id: Option<String>,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameInteractables {
    pub id: String,
    pub location_id: String,
    pub name: String,
    pub event_id: Option<String>,
}

impl From<GameInteractablesRow> for GameInteractables {
    fn from(r: GameInteractablesRow) -> Self {
        Self {
            id: r.id,
            location_id: r.location_id,
            name: r.name,
            event_id: r.event_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameInteractablesInput {
    pub id: Option<String>,
    pub location_id: String,
    pub name: String,
    pub event_id: Option<String>,
}
impl GameInteractablesInput {
    pub fn from_condition(location_id: &str, t: &GameInteractables) -> Self {
        Self {
            id: Some(t.id.clone()),
            location_id: location_id.into(),
            name: t.name.clone(),
            event_id: t.event_id.clone(),
        }
    }
}

// --- GameEventCharacterPool ---
#[derive(FromRow)]
pub struct GameEventCharacterPoolRow {
    pub event_id: String,
    pub game_character_id: String,
    pub spawn_weight: i64,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEventCharacterPool {
    pub event_id: String,
    pub game_character_id: String,
    pub spawn_weight: i64,
}

impl From<GameEventCharacterPoolRow> for GameEventCharacterPool {
    fn from(r: GameEventCharacterPoolRow) -> Self {
        Self {
            event_id: r.event_id,
            game_character_id: r.game_character_id,
            spawn_weight: r.spawn_weight,
        }
    }
}
#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEventCharacterPoolInput {
    pub id: Option<String>,
    pub event_id: String,
    pub game_character_id: String,
    pub spawn_weight: i64,
}

impl GameEventCharacterPoolInput {
    pub fn from_condition(event_id: &str, c: &GameEventCharacterPool) -> Self {
        Self {
            id: None,
            event_id: event_id.into(),
            game_character_id: c.game_character_id.clone(),
            spawn_weight: c.spawn_weight,
        }
    }
}
// ---------------------------------------------------------
// 게임 인카운트 테이블
// ---------------------------------------------------------
#[derive(FromRow, Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEncounterTables {
    pub id: String,
    pub map_id: String,
    pub zone_id: Option<String>,
    pub location_id: Option<String>,
    pub base_trigger_chance: f64,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEncounterTablesInput {
    pub id: Option<String>,
    pub map_id: String,
    pub zone_id: Option<String>,
    pub location_id: Option<String>,
    pub base_trigger_chance: f64,
}

impl From<GameEncounterTables> for GameEncounterTablesInput {
    fn from(v: GameEncounterTables) -> Self {
        Self {
            id: Some(v.id),
            map_id: v.map_id,
            zone_id: v.zone_id,
            location_id: v.location_id,
            base_trigger_chance: v.base_trigger_chance,
        }
    }
}

// --- GameEncounterEntries ---
#[derive(FromRow)]
pub struct GameEncounterEntriesRow {
    pub id: String,
    pub encounter_table_id: String,
    pub event_id: String,
    pub weight: i64,
    pub time_period_id: Option<String>,
    pub weather_id: Option<String>,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEncounterEntries {
    pub id: String,
    pub encounter_table_id: String,
    pub event_id: String,
    pub weight: i64,
    pub time_period_id: Option<String>,
    pub weather_id: Option<String>,
}

impl From<GameEncounterEntriesRow> for GameEncounterEntries {
    fn from(r: GameEncounterEntriesRow) -> Self {
        Self {
            id: r.id,
            encounter_table_id: r.encounter_table_id,
            event_id: r.event_id,
            weight: r.weight,
            time_period_id: r.time_period_id,
            weather_id: r.weather_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEncounterEntriesInput {
    pub id: Option<String>,
    pub encounter_table_id: String,
    pub event_id: String,
    pub weight: i64,
    pub time_period_id: Option<String>,
    pub weather_id: Option<String>,
}

impl From<GameEncounterEntries> for GameEncounterEntriesInput {
    fn from(v: GameEncounterEntries) -> Self {
        Self {
            id: Some(v.id),
            encounter_table_id: v.encounter_table_id,
            event_id: v.event_id,
            weight: v.weight,
            time_period_id: v.time_period_id,
            weather_id: v.weather_id,
        }
    }
}

// ---------------------------------------------------------
// 게임 씬
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameSceneRow {
    pub id: String,
    pub game_id: String,
    pub scene_name: String,
    pub description: Option<String>,
    pub scene_key: String,
    pub settings_json: Option<String>,
    pub script: Option<String>,
    pub is_ending: Option<bool>,
    pub ending_id: Option<String>,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameScene {
    pub id: String,
    pub game_id: String,
    pub scene_name: String,
    pub description: Option<String>,
    pub scene_key: String,
    pub is_ending: Option<bool>,
    pub ending_id: Option<String>,
    pub settings_json: Option<serde_json::Value>,
    pub script: Option<String>,
    pub choices: Option<Vec<GameChoiceRow>>,
    pub branches: Option<Vec<GameBranchRow>>,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl GameScene {
    pub fn from_row(
        r: GameSceneRow,
        choices: Option<Vec<GameChoiceRow>>,
        branches: Option<Vec<GameBranchRow>>,
    ) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            scene_name: r.scene_name,
            description: r.description,
            scene_key: r.scene_key,
            ending_id: r.ending_id,
            settings_json: r.settings_json.and_then(|s| serde_json::from_str(&s).ok()),
            script: r.script,
            choices,
            branches,
            is_ending: r.is_ending,
            sort_order: r.sort_order,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameSceneInput {
    pub id: String,
    pub scene_name: String,
    pub description: Option<String>,
    pub scene_key: String,
    pub is_ending: Option<bool>,
    pub ending_id: Option<String>,
    pub settings_json: Option<String>,
    pub script: Option<String>,
    pub choices: Option<Vec<GameChoiceInput>>,
    pub branches: Option<Vec<GameBranchInput>>,
}

impl From<GameScene> for GameSceneInput {
    fn from(v: GameScene) -> Self {
        Self {
            id: v.id,
            scene_name: v.scene_name,
            description: v.description,
            scene_key: v.scene_key,
            is_ending: v.is_ending,
            ending_id: v.ending_id,
            settings_json: v.settings_json.and_then(|s| serde_json::to_string(&s).ok()),
            script: v.script,
            choices: v.choices.map(|c| c.into_iter().map(Into::into).collect()),
            branches: v.branches.map(|b| b.into_iter().map(Into::into).collect()),
        }
    }
}

// ---------- Branch ----------
#[derive(FromRow, Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameBranchRow {
    pub id: String,
    pub scene_id: String,
    pub condition: Option<String>,
    pub next_scene_id: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameBranchInput {
    pub id: String,
    pub scene_id: String,
    pub condition: Option<String>,
    pub next_scene_id: Option<String>,
    pub sort_order: i64,
}

impl From<GameBranchRow> for GameBranchInput {
    fn from(v: GameBranchRow) -> Self {
        Self {
            id: v.id,
            scene_id: v.scene_id,
            condition: v.condition,
            next_scene_id: v.next_scene_id,
            sort_order: v.sort_order,
        }
    }
}

// ---------- Choice ----------
#[derive(FromRow, Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameChoiceRow {
    pub id: String,
    pub scene_id: String,
    pub text: String,
    pub next_scene_id: Option<String>,
    pub condition: Option<String>,
    pub effect: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameChoiceInput {
    pub id: String,
    pub scene_id: String,
    pub text: String,
    pub next_scene_id: Option<String>,
    pub condition: Option<String>,
    pub effect: Option<String>,
    pub sort_order: i64,
}

impl From<GameChoiceRow> for GameChoiceInput {
    fn from(v: GameChoiceRow) -> Self {
        Self {
            id: v.id,
            scene_id: v.scene_id,
            text: v.text,
            next_scene_id: v.next_scene_id,
            condition: v.condition,
            effect: v.effect,
            sort_order: v.sort_order,
        }
    }
}

// ---------------------------------------------------------
// 게임 엔딩
// ---------------------------------------------------------
#[derive(FromRow, Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEndings {
    pub id: String,
    pub game_id: String,
    pub ending_key: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEndingsInput {
    pub id: Option<String>,
    pub game_id: String,
    pub ending_key: String,
    pub title: String,
    pub description: String,
}

impl From<GameEndings> for GameEndingsInput {
    fn from(v: GameEndings) -> Self {
        Self {
            id: Some(v.id),
            game_id: v.game_id,
            ending_key: v.ending_key,
            title: v.title,
            description: v.description,
        }
    }
}
// ---------------------------------------------------------
// 아이템
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameItemsRow {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub kind: Option<String>,
    pub max_stack: u32,
    pub is_unique: bool,
    pub is_key: bool,
    pub consumable: bool,
    pub use_effect: Option<String>,
    pub use_condition: Option<String>,
    pub on_acquire_event_id: Option<String>,
    pub settings_json: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameItems {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub kind: Option<String>,
    pub max_stack: u32,
    pub is_unique: bool,
    pub is_key: bool,
    pub consumable: bool,
    pub use_effect: serde_json::Value,
    pub use_condition: serde_json::Value,
    pub on_acquire_event_id: Option<String>,
    pub settings_json: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<GameItemsRow> for GameItems {
    fn from(r: GameItemsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            name: r.name,
            description: r.description,
            icon: r.icon,
            kind: r.kind,
            max_stack: r.max_stack,
            is_unique: r.is_unique,
            is_key: r.is_key,
            consumable: r.consumable,
            use_effect: r
                .use_effect
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            use_condition: r
                .use_condition
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            on_acquire_event_id: r.on_acquire_event_id,
            settings_json: r
                .settings_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameItemsInput {
    pub id: Option<String>,
    pub game_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub kind: Option<String>,
    pub max_stack: u32,
    pub is_unique: bool,
    pub is_key: bool,
    pub consumable: bool,
    pub use_effect: serde_json::Value,
    pub use_condition: serde_json::Value,
    pub on_acquire_event_id: Option<String>,
    pub settings_json: serde_json::Value,
}

impl GameItemsInput {
    pub fn from(r: GameItems) -> Self {
        Self {
            id: Some(r.id),
            game_id: r.game_id,
            name: r.name,
            description: r.description,
            icon: r.icon,
            kind: r.kind,
            max_stack: r.max_stack,
            is_unique: r.is_unique,
            is_key: r.is_key,
            consumable: r.consumable,
            use_effect: r.use_effect,
            use_condition: r.use_condition,
            on_acquire_event_id: r.on_acquire_event_id,
            settings_json: r.settings_json,
        }
    }
}

// ---------------------------------------------------------
// 관계도
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameRelationshipDefsRow {
    pub id: String,
    pub game_id: String,
    pub from_id: String,
    pub to_id: String,
    pub values_json: Option<String>,
    pub tags: Option<String>,
}
#[derive(Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameRelationshipDefs {
    pub id: String,
    pub game_id: String,
    pub from_id: String,
    pub to_id: String,
    pub values_json: serde_json::Value,
    pub tags: serde_json::Value,
}
impl From<GameRelationshipDefsRow> for GameRelationshipDefs {
    fn from(r: GameRelationshipDefsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            from_id: r.from_id,
            to_id: r.to_id,
            values_json: r
                .values_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            tags: r
                .tags
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
        }
    }
}
#[derive(Debug, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameRelationshipDefsInput {
    pub id: Option<String>,
    pub game_id: String,
    pub from_id: String,
    pub to_id: String,
    pub values_json: serde_json::Value,
    pub tags: serde_json::Value,
}

impl From<GameRelationshipDefs> for GameRelationshipDefsInput {
    fn from(v: GameRelationshipDefs) -> Self {
        Self {
            id: Some(v.id),
            game_id: v.game_id,
            from_id: v.from_id,
            to_id: v.to_id,
            values_json: v.values_json,
            tags: v.tags,
        }
    }
}

// =========================================================
// 게임 런타임
// =========================================================

// ---------------------------------------------------------
// 게임 플레이 세션
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GamePlayerSessionsRow {
    pub id: String,
    pub game_id: String,
    pub player_id: Option<String>,
    pub current_scene_id: Option<String>,
    pub current_map_id: Option<String>,
    pub current_zone_id: Option<String>,
    pub current_location_id: Option<String>,
    pub current_weather_id: Option<String>,
    pub game_time: i64,
    pub flags_json: Option<String>,
    pub pending_scenes_json: Option<String>,
    pub inventory_json: Option<String>,
    pub relationships_json: Option<String>,
    pub stage_json: Option<String>,
    pub zone_weather_json: Option<String>,
    pub parent_session_id: Option<String>,
    pub forked_from_seq: Option<u32>,
    pub head_seq: u32,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GamePlayerSessions {
    pub id: String,
    pub game_id: String,
    pub player_id: String,
    pub current_scene_id: Option<String>,
    pub current_map_id: Option<String>,
    pub current_zone_id: Option<String>,
    pub current_location_id: Option<String>,
    pub current_weather_id: Option<String>,
    pub game_time: i64,
    pub flags_json: serde_json::Value,
    pub parent_session_id: Option<String>,
    pub pending_scenes_json: serde_json::Value,
    pub inventory_json: serde_json::Value,
    pub relationships_json: serde_json::Value,
    pub stage_json: serde_json::Value,
    /// zone별 날씨 (GameState.zone_weather)
    pub zone_weather_json: serde_json::Value,
    pub forked_from_seq: Option<u32>,
    pub head_seq: u32,
    pub updated_at: i64,
}

impl From<GamePlayerSessionsRow> for GamePlayerSessions {
    fn from(r: GamePlayerSessionsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            player_id: r.player_id.unwrap_or_default(),
            current_scene_id: r.current_scene_id,
            current_map_id: r.current_map_id,
            current_zone_id: r.current_zone_id,
            current_location_id: r.current_location_id,
            current_weather_id: r.current_weather_id,
            game_time: r.game_time,
            flags_json: r
                .flags_json
                .and_then(|f| serde_json::from_str(&f).ok())
                .unwrap_or_default(),
            parent_session_id: r.parent_session_id,
            pending_scenes_json: r
                .pending_scenes_json
                .and_then(|f| serde_json::from_str(&f).ok())
                .unwrap_or_default(),
            inventory_json: r
                .inventory_json
                .and_then(|f| serde_json::from_str(&f).ok())
                .unwrap_or_default(),
            relationships_json: r
                .relationships_json
                .and_then(|f| serde_json::from_str(&f).ok())
                .unwrap_or_default(),
            stage_json: r
                .stage_json
                .and_then(|f| serde_json::from_str(&f).ok())
                .unwrap_or_default(),
            zone_weather_json: r
                .zone_weather_json
                .and_then(|f| serde_json::from_str(&f).ok())
                .unwrap_or_default(),
            forked_from_seq: r.forked_from_seq,
            head_seq: r.head_seq,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GamePlayerSessionsInput {
    pub id: Option<String>,
    pub game_id: String,
    pub player_id: Option<String>,
    pub current_scene_id: Option<String>,
    pub current_map_id: Option<String>,
    pub current_zone_id: Option<String>,
    pub current_location_id: Option<String>,
    pub current_weather_id: Option<String>,
    pub game_time: i64,
    pub flags_json: serde_json::Value,
    pub parent_session_id: Option<String>,
    pub pending_scenes_json: serde_json::Value,
    pub inventory_json: serde_json::Value,
    pub relationships_json: serde_json::Value,
    pub stage_json: serde_json::Value,
    pub zone_weather_json: serde_json::Value,
    pub forked_from_seq: Option<u32>,
    pub head_seq: u32,
}

impl GamePlayerSessionsInput {
    pub fn from_state(v: GameState, game_id: String) -> Self {
        Self {
            id: Some(v.session_id),
            game_id,
            player_id: Some(v.player_id),
            current_scene_id: v.current_scene_id,
            current_map_id: Some(v.map_id),
            current_zone_id: v.zone_id,
            current_location_id: v.location_id,
            current_weather_id: Some(v.weather_id),
            game_time: v.game_time,
            flags_json: v.flags,
            pending_scenes_json: v.pending_scenes_json,
            inventory_json: v.inventory_json,
            relationships_json: v.relationships_json,
            stage_json: v.stage_json,
            zone_weather_json: v.zone_weather,
            parent_session_id: v.parent_session_id,
            forked_from_seq: v.forked_from_seq,
            head_seq: v.head_seq,
        }
    }
}
// ---------------------------------------------------------
// 게임 세이브
// ---------------------------------------------------------
#[derive(FromRow, Serialize, Deserialize, specta::Type)]
pub struct GameSaveRow {
    pub name: Option<String>,
    pub seq: u32,
    pub session_id: String,
    pub slot_number: i64,
    pub current_scene_id: Option<String>,
    pub current_map_id: Option<String>,
    pub current_zone_id: Option<String>,
    pub current_location_id: Option<String>,
    pub current_weather_id: Option<String>,
    pub game_time: i64,
    pub state_snapshot: Option<String>,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameSave {
    pub name: Option<String>,
    pub seq: u32,
    pub session_id: String,
    pub slot_number: i64,
    pub current_scene_id: Option<String>,
    pub current_map_id: Option<String>,
    pub current_zone_id: Option<String>,
    pub current_location_id: Option<String>,
    pub current_weather_id: Option<String>,
    pub game_time: i64,
    pub state_snapshot: serde_json::Value,
    pub updated_at: i64,
}

impl From<GameSaveRow> for GameSave {
    fn from(r: GameSaveRow) -> Self {
        Self {
            name: r.name,
            seq: r.seq,
            session_id: r.session_id,
            slot_number: r.slot_number,
            current_scene_id: r.current_scene_id,
            current_map_id: r.current_map_id,
            current_zone_id: r.current_zone_id,
            current_location_id: r.current_location_id,
            current_weather_id: r.current_weather_id,
            game_time: r.game_time,
            state_snapshot: r
                .state_snapshot
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameSaveInput {
    pub name: Option<String>,
    pub seq: u32,
    pub session_id: String,
    pub slot_number: i64,
    pub current_scene_id: Option<String>,
    pub current_map_id: Option<String>,
    pub current_zone_id: Option<String>,
    pub current_location_id: Option<String>,
    pub current_weather_id: Option<String>,
    pub game_time: i64,
    pub state_snapshot: serde_json::Value,
}

// ---------------------------------------------------------
// 게임 세션 기록
// ---------------------------------------------------------
#[derive(FromRow, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameJournalRow {
    pub session_id: String,
    pub seq: u32,
    pub kind: String,
    pub payload_json: Option<String>,
    pub game_time: i64,
    pub created_at: i64,
}

#[derive(Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameJournal {
    pub session_id: String,
    pub seq: u32,
    pub kind: String,
    pub payload_json: serde_json::Value,
    pub game_time: i64,
    pub created_at: i64,
}

impl From<GameJournalRow> for GameJournal {
    fn from(r: GameJournalRow) -> Self {
        Self {
            session_id: r.session_id,
            seq: r.seq,
            kind: r.kind,
            payload_json: r
                .payload_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            game_time: r.game_time,
            created_at: r.created_at,
        }
    }
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameJournalInput {
    pub session_id: String,
    pub seq: u32,
    pub kind: String,
    pub payload_json: serde_json::Value,
    pub game_time: i64,
}

impl From<GameJournal> for GameJournalInput {
    fn from(v: GameJournal) -> Self {
        Self {
            session_id: v.session_id,
            seq: v.seq,
            kind: v.kind,
            payload_json: v.payload_json,
            game_time: v.game_time,
        }
    }
}

// ---------------------------------------------------------
// 게임 세션 내 스토리 계획
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameStoryPlansRow {
    pub session_id: String,
    pub seq: u32,
    pub plan_json: Option<String>,
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameStoryPlans {
    pub session_id: String,
    pub seq: u32,
    pub plan_json: serde_json::Value,
    pub reason: serde_json::Value,
}

impl From<GameStoryPlansRow> for GameStoryPlans {
    fn from(r: GameStoryPlansRow) -> Self {
        Self {
            session_id: r.session_id,
            seq: r.seq,
            plan_json: r
                .plan_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            reason: r
                .reason
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
        }
    }
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameStoryPlansInput {
    pub session_id: String,
    pub seq: u32,
    pub plan_json: serde_json::Value,
    pub reason: serde_json::Value,
}

// ---------------------------------------------------------
// 게임 세션 내 캐릭터 관계도
// ---------------------------------------------------------
// game_relationship_snapshots
#[derive(FromRow)]
pub struct GameRelationshipSnapshotsRow {
    pub session_id: String,
    pub seq: u32,
    pub relationships_json: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameRelationshipSnapshots {
    pub session_id: String,
    pub seq: u32,
    pub relationships_json: serde_json::Value,
    pub created_at: i64,
}

impl From<GameRelationshipSnapshotsRow> for GameRelationshipSnapshots {
    fn from(r: GameRelationshipSnapshotsRow) -> Self {
        Self {
            session_id: r.session_id,
            seq: r.seq,
            relationships_json: serde_json::from_str(&r.relationships_json).unwrap_or_default(),
            created_at: r.created_at,
        }
    }
}
// ---------------------------------------------------------
// 게임 세션 내 캐릭터 상태
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameCharacterStatesSnapshotsRow {
    pub session_id: String,
    pub seq: u32,
    pub game_character_id: String,
    pub current_location_id: Option<String>,
    pub status: String,
    pub state_json: Option<String>,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameCharacterStatesSnapshots {
    pub session_id: String,
    pub seq: u32,
    pub game_character_id: String,
    pub current_location_id: Option<String>,
    pub status: String,
    pub state_json: serde_json::Value,
    pub updated_at: i64,
}

impl From<GameCharacterStatesSnapshotsRow> for GameCharacterStatesSnapshots {
    fn from(r: GameCharacterStatesSnapshotsRow) -> Self {
        Self {
            session_id: r.session_id,
            seq: r.seq,
            game_character_id: r.game_character_id,
            current_location_id: r.current_location_id,
            status: r.status,
            state_json: r
                .state_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameCharacterStatesSnapshotsInput {
    pub session_id: String,
    pub seq: u32,
    pub game_character_id: String,
    pub current_location_id: Option<String>,
    pub status: String,
    pub state_json: serde_json::Value,
}

// ---------------------------------------------------------
// 게임 이벤트 로그
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameEventLogRow {
    pub seq: u32,
    pub session_id: String,
    pub event_id: String,
    pub weather_id: String,
    pub triggered_at: f32,
    pub result_json: String,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEventLog {
    pub seq: u32,
    pub session_id: String,
    pub event_id: String,
    pub weather_id: String,
    pub triggered_at: f32,
    pub result_json: serde_json::Value,
}

impl From<GameEventLogRow> for GameEventLog {
    fn from(r: GameEventLogRow) -> Self {
        Self {
            seq: r.seq,
            session_id: r.session_id,
            event_id: r.event_id,
            weather_id: r.weather_id,
            triggered_at: r.triggered_at,
            result_json: serde_json::from_str(&r.result_json).unwrap_or_default(),
        }
    }
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEventLogInput {
    pub seq: u32,
    pub session_id: String,
    pub event_id: String,
    pub weather_id: String,
    pub triggered_at: f32,
    pub result_json: serde_json::Value,
}

// ---- GameEventLogCharacters ----
#[derive(FromRow, Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEventLogCharacters {
    pub session_id: String,
    pub seq: u32,
    pub game_character_id: String,
    pub role: Option<String>,
    pub sort_order: i64,
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameEventLogCharactersInput {
    pub session_id: String,
    pub seq: u32,
    pub game_character_id: String,
    pub role: Option<String>,
    pub sort_order: i64,
}

impl From<GameEventLogCharacters> for GameEventLogCharactersInput {
    fn from(v: GameEventLogCharacters) -> Self {
        Self {
            session_id: v.session_id,
            seq: v.seq,
            game_character_id: v.game_character_id,
            role: v.role,
            sort_order: v.sort_order,
        }
    }
}

// ---------------------------------------------------------
// 게임 대화 세션
// ---------------------------------------------------------

#[derive(FromRow)]
pub struct GameConversationsRow {
    pub id: String,
    pub game_id: String,
    pub session_id: String,
    pub branch_key: String,
    pub participants_key: String,
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameConversations {
    pub id: String,
    pub game_id: String,
    pub session_id: String,
    pub branch_key: String,
    pub participants_key: String,
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<GameConversationsRow> for GameConversations {
    fn from(r: GameConversationsRow) -> Self {
        Self {
            id: r.id,
            game_id: r.game_id,
            session_id: r.session_id,
            branch_key: r.branch_key,
            participants_key: r.participants_key,
            title: r.title,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameConversationsInput {
    pub id: Option<String>,
    pub game_id: String,
    pub session_id: String,
    pub branch_key: String,
    pub participants_key: String,
    pub title: Option<String>,
}

impl From<GameConversations> for GameConversationsInput {
    fn from(r: GameConversations) -> Self {
        Self {
            id: Some(r.id),
            game_id: r.game_id,
            session_id: r.session_id,
            branch_key: r.branch_key,
            participants_key: r.participants_key,
            title: r.title,
        }
    }
}

#[derive(FromRow, Serialize, Deserialize, specta::Type)]
pub struct GameConversationsIdsMap {
    pub old_id: String,
    pub new_id: String,
}

// ---------------------------------------------------------
// GameMessages
// ---------------------------------------------------------
#[derive(FromRow)]
pub struct GameMessagesRow {
    pub id: String,
    pub seq: u32,
    pub conversation_id: String,
    pub role: String,
    pub name: Option<String>,
    pub character_id: Option<String>,
    pub content: String,
    pub meta: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameMessages {
    pub id: String,
    pub seq: u32,
    pub conversation_id: String,
    pub role: String,
    pub name: Option<String>,
    pub character_id: Option<String>,
    pub content: String,
    pub meta: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<GameMessagesRow> for GameMessages {
    fn from(r: GameMessagesRow) -> Self {
        Self {
            id: r.id,
            seq: r.seq,
            conversation_id: r.conversation_id,
            role: r.role,
            name: r.name,
            character_id: r.character_id,
            content: r.content,
            meta: r
                .meta
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameMessagesInput {
    pub id: Option<String>,
    pub seq: u32,
    pub conversation_id: String,
    pub role: String,
    pub name: Option<String>,
    pub character_id: Option<String>,
    pub content: String,
    pub meta: serde_json::Value,
}

impl From<GameMessages> for GameMessagesInput {
    fn from(r: GameMessages) -> Self {
        Self {
            id: Some(r.id),
            seq: r.seq,
            conversation_id: r.conversation_id,
            role: r.role,
            name: r.name,
            character_id: r.character_id,
            content: r.content,
            meta: r.meta,
        }
    }
}

// ---------------------------------------------------------
// GameConversationParticipants
// ---------------------------------------------------------
#[derive(FromRow, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GameConversationParticipants {
    pub conversation_id: String,
    pub game_character_id: String,
    pub sort_order: i64,
}
