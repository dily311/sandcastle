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
