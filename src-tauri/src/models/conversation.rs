use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ---------- AppSettings ----------
#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConvSettings {
    pub default_persona_id: Option<String>,
}

// ---------- 대화 변수 정의 (5-F-4-3, ADR-029) ----------
/// 변수 종류 (게임 변수의 number·boolean·string과 같음)
#[derive(Serialize, Deserialize, specta::Type, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VarKind {
    Number,
    Boolean,
    #[default]
    String,
}

/// 캐릭터에 두는 변수 정의. 값은 대화 meta.variables (글자). 정의 없는 변수는 지금처럼
#[derive(Serialize, Deserialize, specta::Type, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VarDef {
    pub key: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub kind: VarKind,
    /// 대화에 값이 없을 때 보이는 처음 값
    #[serde(default)]
    pub default_value: Option<String>,
    /// 수일 때만: 최소·최대·AI가 한 번에 바꿀 수 있는 폭
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub max_step: Option<f64>,
    /// AI가 지금 값을 읽을 수 있나 / 바꿀 수 있나 (5-F-4-4에서 씀)
    #[serde(default)]
    pub ai_read: bool,
    #[serde(default)]
    pub ai_write: bool,
}

// ---------- 프로필 그림 맞추기 (5-E-17-3) ----------
/// 동그란 프로필 칸 안에서 그림의 어느 곳을 보일지: x·y는 그림 안 위치(%), zoom은 확대 배율
#[derive(Serialize, Deserialize, specta::Type, Debug, Clone, Copy, PartialEq)]
pub struct AvatarFit {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl AvatarFit {
    /// 범위 밖 값은 자르고, 기본값(가운데·1배)이면 None — 저장하지 않는다
    pub fn normalized(self) -> Option<Self> {
        let num = |v: f64, lo: f64, hi: f64, d: f64| if v.is_finite() { v.clamp(lo, hi) } else { d };
        let f = Self {
            x: num(self.x, 0.0, 100.0, 50.0),
            y: num(self.y, 0.0, 100.0, 50.0),
            zoom: num(self.zoom, 1.0, 4.0, 1.0),
        };
        (f != Self { x: 50.0, y: 50.0, zoom: 1.0 }).then_some(f)
    }

    /// DB 글(JSON) → 값. 읽지 못하면 None
    pub fn parse(s: Option<&str>) -> Option<Self> {
        s.and_then(|s| serde_json::from_str::<Self>(s).ok()).and_then(Self::normalized)
    }
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
    #[sqlx(default)]
    pub avatar_fit: Option<String>,
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
    /// 프로필 그림 확대·위치 (없으면 가운데·확대 없음)
    pub avatar_fit: Option<AvatarFit>,
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
            avatar_fit: AvatarFit::parse(r.avatar_fit.as_deref()),
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
    #[sqlx(default)]
    pub avatar_fit: Option<String>,
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
    /// 프로필 그림 확대·위치 (없으면 가운데·확대 없음)
    pub avatar_fit: Option<AvatarFit>,
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
            avatar_fit: AvatarFit::parse(r.avatar_fit.as_deref()),
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
    /// 분류 (5-F-1)
    #[sqlx(default)]
    pub category: String,
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
    /// 분류: 캐릭터·장소·아이템·이벤트·세계 또는 사용자가 만든 것. 빈 글 = 분류 없음 (5-F-1)
    pub category: String,
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
            category: r.category,
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
    /// 분류 (없으면 빈 값, 5-F-1)
    #[serde(default)]
    pub category: String,
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
            category: v.category,
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
    pub model: Option<String>,
    #[sqlx(default)]
    pub read_mode: i64,
    /// 뷰에서만 옴 (메시지의 가장 늦은 작성 시각)
    #[sqlx(default)]
    pub last_message_at: Option<i64>,
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
    /// 이 대화에서 고른 모델. 없으면 API 프로필의 기본 모델 (ADR-020)
    pub model: Option<String>,
    /// 마지막 메시지의 작성 시각 (메시지가 없으면 None). 목록의 시각·정렬은 이것을 먼저 본다 —
    /// updated_at은 이 기기에서 바뀐 시각이라 가져온 대화는 가져온 때가 된다
    pub last_message_at: Option<i64>,
    /// 읽기 모드: 보내는 줄을 숨김 (5-E-6)
    pub read_mode: bool,
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
            model: r.model,
            last_message_at: r.last_message_at,
            read_mode: r.read_mode != 0,
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
    /// 이 대화의 모델 (None = API 프로필의 기본 모델)
    #[serde(default)]
    pub model: Option<String>,
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
