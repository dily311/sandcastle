use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ---------- AppSettings ----------
#[derive(Serialize, specta::Type)]
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

#[derive(Serialize, specta::Type)]
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
            world_info_ids: r.world_info_ids.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            tags: r.tags.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            color: r.color,
            is_system_default: r.is_system_default != 0,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize, specta::Type)]
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

#[derive(Serialize, specta::Type)]
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
    /// 커버 이미지 (cover 열을 읽지 않는 쿼리도 있어 없으면 None)
    #[sqlx(default)]
    pub cover: Option<String>,
    #[sqlx(default)]
    pub cover_y: Option<f64>,
    pub description: String,
    pub alternate_descriptions: Option<String>,
    pub first_message: Option<String>,
    pub alternate_first_message: Option<String>,
    pub example_dialogue: Option<String>,
    pub system_prompt_override: Option<String>,
    pub creator_notes: Option<String>,
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

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    /// 캐릭터 페이지 위쪽 커버 이미지
    #[serde(default)]
    pub cover: Option<String>,
    /// 커버에서 보일 세로 위치 % (0 위 ~ 100 아래)
    #[serde(default = "half")]
    pub cover_y: f64,
    pub description: String,
    pub alternate_descriptions: Vec<String>,
    pub first_message: Option<String>,
    pub alternate_first_message: Vec<String>,
    pub example_dialogue: Option<String>,
    pub system_prompt_override: Option<String>,
    /// 제작자 메모 (소개 화면용, 프롬프트에 안 들어감)
    pub creator_notes: Option<String>,
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
            cover: r.cover,
            cover_y: r.cover_y.unwrap_or(50.0),
            description: r.description,
            alternate_descriptions: r.alternate_descriptions.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            first_message: r.first_message,
            alternate_first_message: r.alternate_first_message.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            example_dialogue: r.example_dialogue,
            system_prompt_override: r.system_prompt_override,
            creator_notes: r.creator_notes,
            world_info_ids: r.world_info_ids.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
            color: r.color,
            tags: r.tags.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default(),
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

#[derive(Deserialize, specta::Type)]
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
    /// 예전 묶음/입력에는 없으므로 기본값 허용
    #[serde(default)]
    pub creator_notes: Option<String>,
    pub world_info_ids: Vec<String>,
    pub tags: Vec<String>,
    pub color: Option<String>,
    pub category: Option<String>,
    pub default_persona_id: Option<String>,
    pub default_prompt_preset_id: Option<String>,
    pub favorite: bool,
}

#[derive(FromRow, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CharacterOption {
    pub id: String,
    pub name: String,
}

// ---------- WorldInfo ----------
#[derive(FromRow, Serialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfoBook {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, specta::Type)]
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
    /// 제목 (예전 쿼리처럼 열을 고르지 않은 곳에서는 빈 값)
    #[sqlx(default)]
    pub comment: Option<String>,
}

#[derive(Serialize, Clone, specta::Type)]
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
    /// 제목 (SillyTavern의 comment). 프롬프트에는 들어가지 않음
    pub comment: String,
}

impl From<WorldInfoEntryRow> for WorldInfoEntry {
    fn from(r: WorldInfoEntryRow) -> Self {
        Self {
            id: r.id,
            book_id: r.book_id,
            keys: r.keys.and_then(|k| serde_json::from_str(&k).ok()).unwrap_or_default(),
            secondary_keys: r.secondary_keys.and_then(|k| serde_json::from_str(&k).ok()).unwrap_or_default(),
            content: r.content,
            enabled: r.enabled != 0,
            priority: r.priority,
            constant: r.constant != 0,
            comment: r.comment.unwrap_or_default(),
        }
    }
}

#[derive(Serialize, Deserialize, specta::Type)]
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
    /// 제목 (없으면 빈 값)
    #[serde(default)]
    pub comment: String,
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
            comment: v.comment,
        }
    }
}

// ---------- Conversation ----------
#[derive(FromRow)]
pub struct ConversationRow {
    pub id: String,
    // 아래 nullable 컬럼들은 캐릭터/페르소나 삭제 시 NULL이 될 수 있다 (ON DELETE SET NULL)
    pub title: Option<String>,
    pub character_id: Option<String>,
    pub persona_id: Option<String>,
    pub world_info_ids: Option<String>,
    pub mode: Option<String>,
    pub meta: Option<String>,
    pub pinned: Option<i64>,
    pub images_enabled: i64,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub message_count: i64,
    pub folder_id: Option<String>,
    pub prompt_preset_id: Option<String>,
    pub api_profile_id: Option<String>,
}

#[derive(Serialize, specta::Type)]
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
    /// 이 대화에서 캐릭터 이미지(<cg>키</cg>) 사용 여부
    pub images_enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: i64,
    /// 대화 목록 폴더 (없으면 폴더 밖)
    pub folder_id: Option<String>,
    /// 이 대화의 프리셋/API. 없으면 앱 설정의 선택값
    pub prompt_preset_id: Option<String>,
    pub api_profile_id: Option<String>,
}

impl From<ConversationRow> for Conversation {
    fn from(r: ConversationRow) -> Self {
        Self {
            id: r.id,
            title: r.title.unwrap_or_default(),
            character_id: r.character_id.unwrap_or_default(),
            persona_id: r.persona_id.unwrap_or_default(),
            world_info_ids: r.world_info_ids.and_then(|w| serde_json::from_str(&w).ok()).unwrap_or_default(),
            mode: r.mode.unwrap_or_default(),
            meta: r.meta.and_then(|m| serde_json::from_str(&m).ok()).unwrap_or_default(),
            pinned: r.pinned.unwrap_or(0) != 0,
            images_enabled: r.images_enabled != 0,
            created_at: r.created_at.unwrap_or_default(),
            updated_at: r.updated_at.unwrap_or_default(),
            message_count: r.message_count,
            folder_id: r.folder_id,
            prompt_preset_id: r.prompt_preset_id,
            api_profile_id: r.api_profile_id,
        }
    }
}

/// 대화 모드. 둘 다 같은 대화 기록·AI 흐름을 쓰고, 기본 시스템 프롬프트만 다르다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, strum_macros::EnumString, strum_macros::Display, specta::Type)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ConversationMode {
    /// 일반 대화
    #[default]
    Chat,
    /// 폰 메신저 대화: AppSetting.phone_mode_prompt가 붙는다
    Phone,
}

impl ConversationMode {
    /// DB 값 해석. 비었거나 모르는 값은 chat으로 본다.
    pub fn parse_or_default(value: &str) -> Self {
        value.trim().to_ascii_lowercase().parse().unwrap_or_default()
    }

    /// 이 모드에서 앱 고정 규칙 뒤에 붙일 시스템 프롬프트
    pub fn system_prompt(&self, phone_mode_prompt: &str) -> Option<String> {
        match self {
            Self::Chat => None,
            Self::Phone => Some(phone_mode_prompt.trim().to_string()).filter(|s| !s.is_empty()),
        }
    }
}

#[derive(Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConversationInput {
    pub title: String,
    pub character_id: String,
    pub persona_id: String,
    pub world_info_ids: Vec<String>,
    pub mode: String,
    pub meta: Option<serde_json::Value>,
    /// 생략하면 켜짐
    #[serde(default)]
    pub images_enabled: Option<bool>,
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

#[derive(Clone, Serialize, specta::Type)]
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
            meta: r.meta.and_then(|m| serde_json::from_str(&m).ok()).unwrap_or_default(),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Deserialize, specta::Type)]
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

#[cfg(test)]
mod mode_tests {
    use super::ConversationMode;

    #[test]
    fn parse_and_prompt() {
        assert_eq!(ConversationMode::parse_or_default("Phone"), ConversationMode::Phone);
        assert_eq!(ConversationMode::parse_or_default(""), ConversationMode::Chat);
        assert_eq!(ConversationMode::parse_or_default("game"), ConversationMode::Chat);
        assert_eq!(ConversationMode::Phone.to_string(), "phone");

        assert_eq!(ConversationMode::Chat.system_prompt("p"), None);
        assert_eq!(ConversationMode::Phone.system_prompt("  p  "), Some("p".into()));
        assert_eq!(ConversationMode::Phone.system_prompt("   "), None, "비우면 붙이지 않음");
    }
}

// ---------- 대화 폴더 ----------
#[derive(Debug, Clone, Serialize, FromRow, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConversationFolder {
    pub id: String,
    pub character_id: String,
    pub name: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 대화별 설정 (오른쪽 패널). None이면 앱 설정의 선택값을 쓴다.
#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConversationOptions {
    pub persona_id: String,
    pub prompt_preset_id: Option<String>,
    pub api_profile_id: Option<String>,
}

/// 한 번의 대화 차례 결과
#[derive(Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatTurn {
    /// 이번에 저장된 사용자 메시지 (이어쓰기·다시 생성이면 없음)
    pub user: Option<Message>,
    pub assistant: Message,
}

/// 폰 연락처. 주인(폰 주인)과 연락처가 각각 페르소나 또는 캐릭터
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PhoneContact {
    /// "persona" | "character"
    pub owner_kind: String,
    pub owner_id: String,
    /// "persona" | "character"
    pub contact_kind: String,
    pub contact_id: String,
    pub status: String,
    pub sort_order: i64,
    pub created_at: i64,
}

fn half() -> f64 {
    50.0
}
