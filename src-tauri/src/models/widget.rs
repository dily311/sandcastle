use std::path::PathBuf;

use lofty::tag::{Accessor, ItemKey, Tag};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::sys::music::save_album_art;

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct WidgetAudioState {
    pub muted: bool,
    pub volume: f64,
    pub loop_mode: i64,
    pub is_shuffle: bool,
}

impl Default for WidgetAudioState {
    fn default() -> Self {
        Self {
            muted: false,
            volume: 0.7,
            loop_mode: 0,
            is_shuffle: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct WidgetMusicSetting {
    pub resize: bool,
    pub is_mini_player: bool,
    pub is_img_edit: bool,
    pub is_play_list_open: bool,
    pub pos: serde_json::Value,
    pub selected_folder: Option<String>,
    pub current_playlist_id: Option<String>,
    pub current_song_index: i64,
    pub audio: WidgetAudioState,
}

impl Default for WidgetMusicSetting {
    fn default() -> Self {
        Self {
            resize: false,
            is_mini_player: false,
            is_img_edit: false,
            is_play_list_open: false,
            pos: serde_json::Value::default(),
            selected_folder: None,
            current_playlist_id: Some("default".into()),
            current_song_index: 0,
            audio: WidgetAudioState::default(),
        }
    }
}

// 음악 정보 구조체
#[derive(FromRow, Serialize, Deserialize, Clone, specta::Type)]
pub struct MusicMetaSummary {
    pub path: String,
    pub album_art: String,
    pub updated_at: i64,
}

#[derive(FromRow)]
pub struct MusicMetaRow {
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_title: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<String>,
    pub genre: Option<String>,
    pub comment: Option<String>,
    pub album_art: Option<String>,
    pub lyrics: Option<String>,
    pub img_pos: Option<String>,
    pub duration: f64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MusicMeta {
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_title: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<String>,
    pub genre: Option<String>,
    pub comment: Option<String>,
    pub album_art: Option<String>,
    pub lyrics: Option<String>,
    pub duration: f64,
    pub img_pos: Option<serde_json::Value>,
    pub updated_at: i64,
}

impl From<MusicMetaRow> for MusicMeta {
    fn from(r: MusicMetaRow) -> Self {
        Self {
            path: r.path,
            title: r.title,
            artist: r.artist,
            album: r.album,
            album_title: r.album_title,
            album_artist: r.album_artist,
            track_number: r.track_number,
            genre: r.genre,
            comment: r.comment,
            album_art: r.album_art,
            lyrics: r.lyrics,
            duration: r.duration,
            img_pos: r.img_pos.and_then(|p| serde_json::from_str(&p).ok()),
            updated_at: r.updated_at,
        }
    }
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MusicMetaInput {
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_title: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<String>,
    pub genre: Option<String>,
    pub comment: Option<String>,
    pub album_art: Option<String>,
    pub lyrics: Option<String>,
    pub duration: f64,
    pub updated_at: i64,
}

impl MusicMetaInput {
    pub fn default(path: &PathBuf, tag: Option<&Tag>, cache_dir: &PathBuf, duration: f64, updated_at: i64) -> Self {
        let audio_src = path.to_string_lossy().to_string();

        let file_name_fallback = path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown Title").to_string();

        let album_art = tag.and_then(|t| t.pictures().first().and_then(|p| save_album_art(p, &file_name_fallback, cache_dir).ok()));

        Self {
            path: audio_src,
            title: tag
                .and_then(|t| t.title().map(String::from))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or(file_name_fallback),
            artist: tag.and_then(|t| t.artist().map(String::from)),
            album: tag.and_then(|t| t.album().map(String::from)),
            album_title: tag.and_then(|t| t.get_string(ItemKey::AlbumTitle).map(String::from)),
            album_artist: tag.and_then(|t| t.get_string(ItemKey::AlbumArtist).map(String::from)),
            track_number: tag.and_then(|t| t.get_string(ItemKey::TrackNumber).map(String::from)),
            genre: tag.and_then(|t| t.genre().map(String::from)),
            comment: tag.and_then(|t| t.comment().map(String::from)),
            album_art,
            lyrics: tag.and_then(|t| t.get_string(ItemKey::Lyrics).map(String::from)),
            duration,
            updated_at,
        }
    }
}

#[derive(FromRow, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover_image: Option<String>,
    pub sort_order: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistInput {
    pub title: String,
    pub description: Option<String>,
    pub cover_image: Option<String>,
    pub sort_order: u32,
}

#[derive(FromRow)]
pub struct PlaylistItemRow {
    #[sqlx(flatten)]
    pub meta: MusicMetaRow,
    pub sort_order: u32,
}

#[derive(Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistItem {
    pub meta: MusicMeta,
    pub sort_order: u32,
}

impl From<PlaylistItemRow> for PlaylistItem {
    fn from(r: PlaylistItemRow) -> Self {
        Self {
            meta: r.meta.into(),
            sort_order: r.sort_order,
        }
    }
}

// =========================================================
// 시메지 (마스코트) 설정
// =========================================================

/// 시메지 행동별 가중치(빈도)
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct MascotBehaviorWeights {
    pub idle: u32,
    pub walk: u32,
    pub sitting: u32,
    pub sleep: u32,
    pub working: u32,
}

impl Default for MascotBehaviorWeights {
    fn default() -> Self {
        Self {
            idle: 40,
            walk: 35,
            sitting: 10,
            sleep: 10,
            working: 5,
        }
    }
}

/// 시메지 포즈/상태별 이미지 경로 목록 (여러 장이면 프레임 애니메이션)
#[derive(Debug, Serialize, Deserialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct MascotImages {
    pub idle: Vec<String>,
    pub walk: Vec<String>,
    pub sitting: Vec<String>,
    pub sleep: Vec<String>,
    pub drag: Vec<String>,
    pub happy: Vec<String>,
    pub annoyed: Vec<String>,
    pub working: Vec<String>,
}

/// 시메지 상황별 대사 (각 항목은 여러 줄/후보 중 랜덤 선택, {{user}}/{{char}}/{{notify}} 매크로 지원)
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct MascotDialogues {
    /// 기본 인사 (등장 시)
    pub greeting: Vec<String>,
    /// 오랜만에 볼 경우 (7일 이상 미접속)
    pub long_absence: Vec<String>,
    /// 선물을 준 경우
    pub gift: Vec<String>,
    /// 알림이 온 경우 ({{notify}} 치환)
    pub notify: Vec<String>,
    /// 잡아 끈 경우
    pub drag: Vec<String>,
    /// 짜증 난 경우
    pub annoyed: Vec<String>,
    /// 행복해한 경우
    pub happy: Vec<String>,
    /// 평소 혼잣말
    pub idle_chatter: Vec<String>,
    /// 다른 시메지와 대화할 때의 기본 대사 (AI 미사용/오프라인 폴백)
    pub peer_chat: Vec<String>,
}

impl Default for MascotDialogues {
    fn default() -> Self {
        Self {
            greeting: vec!["안녕, {{user}}!".into(), "오늘도 좋은 하루 보내자!".into()],
            long_absence: vec!["{{user}}! 정말 오랜만이야, 보고 싶었어!".into(), "일주일 넘게 어디 갔었어? 기다렸잖아!".into()],
            gift: vec!["와, 고마워 {{user}}! 소중히 간직할게!".into(), "선물이야? 정말 기뻐!".into()],
            notify: vec!["{{user}}, 알림이 왔어! {{notify}}".into(), "띵동~ 새 소식이 도착했어!".into()],
            drag: vec!["으악, 잠깐만! 내려놔줘!".into(), "어디로 데려가는 거야~?".into()],
            annoyed: vec!["그만 좀 괴롭혀! 어지럽단 말이야!".into(), "흥, 자꾸 그러면 토라질 거야!".into()],
            happy: vec!["헤헤, 기분 좋아!".into(), "역시 {{user}} 옆이 제일 좋아!".into()],
            idle_chatter: vec!["심심한데 뭐 하고 놀까?".into(), "오늘 날씨는 어떨까~".into()],
            peer_chat: vec!["안녕! 오늘 기분은 어때?".into(), "같이 산책할래?".into()],
        }
    }
}

/// 개별 시메지 캐릭터 설정
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct MascotCharacterConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub character_id: Option<String>,
    pub persona_id: Option<String>,
    pub api_profile_id: Option<String>,
    pub model: Option<String>,
    pub preset_id: Option<String>,
    pub use_ai_dialogue: bool,
    pub scale: f64,
    pub speed: f64,
    pub action_interval_sec: f64,
    pub behavior_weights: MascotBehaviorWeights,
    pub images: MascotImages,
    pub dialogues: MascotDialogues,
    pub last_seen_at: Option<i64>,
}

impl Default for MascotCharacterConfig {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "시메지".into(),
            enabled: true,
            character_id: None,
            persona_id: None,
            api_profile_id: None,
            model: None,
            preset_id: None,
            use_ai_dialogue: true,
            scale: 1.0,
            speed: 80.0,
            action_interval_sec: 5.0,
            behavior_weights: MascotBehaviorWeights::default(),
            images: MascotImages::default(),
            dialogues: MascotDialogues::default(),
            last_seen_at: None,
        }
    }
}

/// 위젯 마스코트(시메지) 전체 설정 (widget.json 의 "mascot" 키)
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct WidgetMascotSetting {
    pub characters: Vec<MascotCharacterConfig>,
    pub pair_chat_enabled: bool,
    pub pair_chat_interval_sec: u32,
    pub pair_chat_use_ai: bool,
    pub pair_chat_distance_px: u32,
}

impl Default for WidgetMascotSetting {
    fn default() -> Self {
        Self {
            characters: vec![],
            pair_chat_enabled: true,
            pair_chat_interval_sec: 90,
            // 사용자가 누르지 않아도 정해진 간격마다 API를 부르므로 기본 꺼짐
            pair_chat_use_ai: false,
            pair_chat_distance_px: 260,
        }
    }
}

/// 시메지 AI 응답 요청 (유저와 대화 / 선물 반응 등)
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MascotChatRequest {
    pub mascot_id: String,
    pub user_message: String,
    /// 상황 힌트 (예: "chat", "gift", "greeting", "long_absence", "notify")
    pub event_kind: Option<String>,
    /// 최근 짧은 대화 기록 (role: "user" | "assistant", content)
    #[serde(default)]
    pub history: Vec<MascotChatHistoryItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MascotChatHistoryItem {
    pub role: String,
    pub content: String,
}

/// 두 시메지 간 대화 생성 요청
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MascotPeerChatRequest {
    pub mascot_a_id: String,
    pub mascot_b_id: String,
    pub topic: Option<String>,
    /// 타이머가 시작한 대화면 true (`run_by = auto`, `pair_chat_use_ai`가 켜져 있어야 함). 사용자가 부른 것이면 false
    #[serde(default)]
    pub auto: bool,
}

/// 두 시메지 간 대화의 한 턴
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MascotPeerChatTurn {
    pub mascot_id: String,
    pub name: String,
    pub text: String,
}

/// 알림 도착 시 시메지에게 전달되는 이벤트 페이로드
#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WidgetMascotNotifyPayload {
    pub target_name: String,
    pub body: String,
}
