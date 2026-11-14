//! 채팅용 캐릭터 이미지 (`<cg>키</cg>`)
use std::path::Path;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(FromRow)]
pub struct CharacterImageRow {
    pub id: String,
    pub character_id: String,
    pub key: String,
    pub file_path: String,
    pub thumb_path: Option<String>,
    pub enabled: bool,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 프론트 전달용. 경로는 절대 경로 (프론트에서 convertFileSrc로 표시)
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CharacterImage {
    pub id: String,
    pub character_id: String,
    pub key: String,
    pub path: String,
    pub thumbnail_path: Option<String>,
    pub enabled: bool,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl CharacterImage {
    /// DB의 상대 경로를 앱 데이터 폴더 기준 절대 경로로 바꿔서 만든다
    pub fn from_row(r: CharacterImageRow, app_data_dir: &Path) -> Self {
        let abs = |p: &str| app_data_dir.join(p).to_string_lossy().to_string();
        Self {
            path: abs(&r.file_path),
            thumbnail_path: r.thumb_path.as_deref().map(abs),
            id: r.id,
            character_id: r.character_id,
            key: r.key,
            enabled: r.enabled,
            sort_order: r.sort_order,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CharacterImageUpdate {
    pub key: String,
    pub enabled: bool,
    pub sort_order: i64,
}

/// AI 응답에서 실제로 쓰인 이미지 (응답 본문의 `<cg>키</cg>` 순서대로)
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ChatImageRef {
    pub key: String,
    pub image_id: String,
    pub path: String,
}
