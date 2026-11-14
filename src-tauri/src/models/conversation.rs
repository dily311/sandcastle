use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ---------- AppSettings ----------
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvSettings {
    pub default_persona_id: Option<String>,
}

// ---------- Persona ----------
#[derive(FromRow)]
pub struct PersonaRow {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub description: String,
    pub world_info_ids: Option<String>,
    pub tags: Option<String>,
    pub color: String,
    pub is_system_default: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Persona {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub description: String,
    pub world_info_ids: Vec<String>,
    pub tags: Vec<String>,
    pub color: String,
    pub is_system_default: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<PersonaRow> for Persona {
    fn from(r: PersonaRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            avatar: r.avatar,
            description: r.description,
            world_info_ids: r
                .world_info_ids
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            tags: r
                .tags
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            color: r.color,
            is_system_default: r.is_system_default != 0,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonaInput {
    pub name: String,
    pub avatar: Option<String>,
    pub description: String,
    pub world_info_ids: Vec<String>,
    pub tags: Vec<String>,
    pub color: String,
    pub is_system_default: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonaUsage {
    pub character_count: i64,
    pub is_system_default: bool,
}

// ---------- Character ----------
#[derive(FromRow)]
pub struct CharacterRow {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub description: String,
    pub alternate_descriptions: Option<String>,
    pub first_message: Option<String>,
    pub alternate_first_message: Option<String>,
    pub example_dialogue: Option<String>,
    pub system_prompt_override: Option<String>,
    pub world_info_ids: Option<String>,
    pub color: Option<String>,
    pub tags: Option<String>,
    pub category: Option<String>,
    pub default_persona_id: Option<String>,
    pub default_prompt_preset_id: Option<String>,
    pub favorite: bool,
    pub message_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub description: String,
    pub alternate_descriptions: Vec<String>,
    pub first_message: Option<String>,
    pub alternate_first_message: Vec<String>,
    pub example_dialogue: Option<String>,
    pub system_prompt_override: Option<String>,
    pub world_info_ids: Vec<String>,
    pub color: Option<String>,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub default_persona_id: Option<String>,
    pub default_prompt_preset_id: Option<String>,
    pub favorite: bool,
    pub message_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<CharacterRow> for Character {
    fn from(r: CharacterRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            avatar: r.avatar,
            description: r.description,
            alternate_descriptions: r.alternate_descriptions.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            first_message: r.first_message,
            alternate_first_message: r.alternate_first_message.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            example_dialogue: r.example_dialogue,
            system_prompt_override: r.system_prompt_override,
            world_info_ids: r
                .world_info_ids
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            color: r.color,
            tags: r
                .tags
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            category: r.category,
            default_persona_id: r.default_persona_id,
            default_prompt_preset_id: r.default_prompt_preset_id,
            favorite: r.favorite,
            message_count: r.message_count,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterInput {
    pub name: String,
    pub avatar: Option<String>,
    pub description: String,
    pub alternate_descriptions: Vec<String>,
    pub first_message: Option<String>,
    pub alternate_first_message: Vec<String>,
    pub example_dialogue: Option<String>,
    pub system_prompt_override: Option<String>,
    pub world_info_ids: Vec<String>,
    pub tags: Vec<String>,
    pub color: Option<String>,
    pub category: Option<String>,
    pub default_persona_id: Option<String>,
    pub default_prompt_preset_id: Option<String>,
    pub favorite: bool,
}

#[derive(FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterOption {
    pub id: String,
    pub name: String,
}

// ---------- WorldInfo ----------
#[derive(FromRow, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfoBook {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfoBookInput {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
}

impl From<WorldInfoBook> for WorldInfoBookInput {
    fn from(v: WorldInfoBook) -> Self {
        Self {
            id: Some(v.id),
            name: v.name,
            description: v.description,
        }
    }
}

#[derive(FromRow)]
pub struct WorldInfoEntryRow {
    pub id: String,
    pub book_id: String,
    pub keys: Option<String>,
    pub secondary_keys: Option<String>,
    pub content: String,
    pub enabled: i64,
    pub priority: i64,
    pub constant: i64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfoEntry {
    pub id: String,
    pub book_id: String,
    pub keys: Vec<String>,
    pub secondary_keys: Vec<String>,
    pub content: String,
    pub enabled: bool,
    pub priority: i64,
    pub constant: bool, // 항상 프롬프트에 삽입
}

impl From<WorldInfoEntryRow> for WorldInfoEntry {
    fn from(r: WorldInfoEntryRow) -> Self {
        Self {
            id: r.id,
            book_id: r.book_id,
            keys: r
                .keys
                .and_then(|k| serde_json::from_str(&k).ok())
                .unwrap_or_default(),
            secondary_keys: r
                .secondary_keys
                .and_then(|k| serde_json::from_str(&k).ok())
                .unwrap_or_default(),
            content: r.content,
            enabled: r.enabled != 0,
            priority: r.priority,
            constant: r.constant != 0,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfoEntryInput {
    pub id: Option<String>, // 없으면 새로 생성, 있으면 upsert
    pub book_id: String,
    pub keys: Vec<String>,
    pub secondary_keys: Vec<String>,
    pub content: String,
    pub enabled: bool,
    pub priority: i64,
    pub constant: bool,
}

impl From<WorldInfoEntry> for WorldInfoEntryInput {
    fn from(v: WorldInfoEntry) -> Self {
        Self {
            id: Some(v.id),
            book_id: v.book_id,
            keys: v.keys,
            secondary_keys: v.secondary_keys,
            content: v.content,
            enabled: v.enabled,
            priority: v.priority,
            constant: v.constant,
        }
    }
}

// ---------- Conversation ----------
#[derive(FromRow)]
pub struct ConversationRow {
    pub id: String,
    pub title: String,
    pub character_id: String,
    pub persona_id: String,
    pub world_info_ids: Option<String>,
    pub mode: String,
    pub meta: Option<String>,
    pub pinned: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub character_id: String,
    pub persona_id: String,
    pub world_info_ids: Vec<String>,
    pub mode: String,
    pub meta: Option<serde_json::Value>,
    pub pinned: bool, // 즐겨찾기 여부
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: i64,
}

impl From<ConversationRow> for Conversation {
    fn from(r: ConversationRow) -> Self {
        Self {
            id: r.id,
            title: r.title,
            character_id: r.character_id,
            persona_id: r.persona_id,
            world_info_ids: r
                .world_info_ids
                .and_then(|w| serde_json::from_str(&w).ok())
                .unwrap_or_default(),
            mode: r.mode,
            meta: r
                .meta
                .and_then(|m| serde_json::from_str(&m).ok())
                .unwrap_or_default(),
            pinned: r.pinned != 0,
            created_at: r.created_at,
            updated_at: r.updated_at,
            message_count: r.message_count,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationInput {
    pub title: String,
    pub character_id: String,
    pub persona_id: String,
    pub world_info_ids: Vec<String>,
    pub mode: String,
    pub meta: Option<serde_json::Value>,
}

// ---------- Message ----------
#[derive(FromRow)]
pub struct MessageRow {
    pub id: String,
    pub conversation_id: String,
    pub seq: i64,
    pub role: String,
    pub name: Option<String>,
    pub content: String,
    pub character_id: Option<String>,
    pub meta: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub seq: i64,
    pub role: String,
    pub name: Option<String>,
    pub content: String,
    pub character_id: Option<String>,
    pub meta: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<MessageRow> for Message {
    fn from(r: MessageRow) -> Self {
        Self {
            id: r.id,
            conversation_id: r.conversation_id,
            seq: r.seq,
            role: r.role,
            name: r.name,
            content: r.content,
            character_id: r.character_id,
            meta: r
                .meta
                .and_then(|m| serde_json::from_str(&m).ok())
                .unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageInput {
    pub conversation_id: String,
    pub seq: i64,
    pub role: String,
    pub name: Option<String>,
    pub content: String,
    pub character_id: Option<String>,
    pub meta: Option<serde_json::Value>,
}
