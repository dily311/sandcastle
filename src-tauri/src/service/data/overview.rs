//! 데이터 관리 화면용 조회: 캐릭터 → 채팅 트리, 전체 개수, 저장 공간
use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{db::conn::DbPool, error::Result};

use super::archive::dir_size;

#[derive(Debug, Clone, Serialize, FromRow, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: String,
    pub title: Option<String>,
    pub mode: Option<String>,
    pub character_id: Option<String>,
    pub message_count: i64,
    /// meta.hidden 메시지 수
    pub hidden_count: i64,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub last_message_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CharacterData {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
    pub conversation_count: usize,
    pub message_count: i64,
    pub image_count: i64,
    pub world_info_book_count: i64,
    /// 가장 최근 메시지 시각 (없으면 대화 갱신 시각)
    pub last_activity: Option<i64>,
    /// 최근 활동 순
    pub conversations: Vec<ConversationSummary>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DataTotals {
    pub characters: i64,
    pub personas: i64,
    pub conversations: i64,
    pub messages: i64,
    pub world_info_books: i64,
    pub prompt_presets: i64,
    pub regex_scripts: i64,
    pub character_images: i64,
    pub games: i64,
}

#[derive(Debug, Clone, Default, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsage {
    pub database_bytes: u64,
    pub characters_bytes: u64,
    pub games_bytes: u64,
    pub themes_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DataOverview {
    /// 최근 활동 순
    pub characters: Vec<CharacterData>,
    /// 캐릭터가 지워졌거나 없는 대화
    pub orphan_conversations: Vec<ConversationSummary>,
    pub totals: DataTotals,
    pub storage: StorageUsage,
}

pub async fn conversation_summaries(db: &DbPool) -> Result<Vec<ConversationSummary>> {
    Ok(sqlx::query_as::<_, ConversationSummary>(
        r#"
        SELECT c.id, c.title, c.mode, c.character_id, c.created_at, c.updated_at,
               COUNT(m.id) AS message_count,
               COALESCE(SUM(CASE WHEN json_valid(m.meta) AND json_extract(m.meta, '$.hidden') = 1 THEN 1 ELSE 0 END), 0) AS hidden_count,
               MAX(m.created_at) AS last_message_at
        FROM conversations c
        LEFT JOIN messages m ON m.conversation_id = c.id
        GROUP BY c.id
        ORDER BY COALESCE(MAX(m.created_at), c.updated_at, 0) DESC
        "#,
    )
    .fetch_all(db)
    .await?)
}

async fn count(db: &DbPool, table: &str) -> Result<i64> {
    // table은 아래 고정 목록에서만 온다
    let sql = format!("SELECT COUNT(*) FROM {table}");
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql)).fetch_one(db).await?)
}

async fn grouped_counts(db: &DbPool, sql: &'static str) -> Result<HashMap<String, i64>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(sql).fetch_all(db).await?;
    Ok(rows.into_iter().collect())
}

pub async fn data_overview(db: &DbPool, app_data_dir: &Path) -> Result<DataOverview> {
    let characters: Vec<(String, String, Option<String>)> = sqlx::query_as("SELECT id, name, avatar FROM characters").fetch_all(db).await?;
    let images = grouped_counts(db, "SELECT character_id, COUNT(*) FROM character_images GROUP BY character_id").await?;
    let books = grouped_counts(db, "SELECT character_id, COUNT(*) FROM character_worldinfo GROUP BY character_id").await?;

    let mut by_char: HashMap<String, Vec<ConversationSummary>> = HashMap::new();
    let mut orphans = vec![];
    let known: std::collections::HashSet<&str> = characters.iter().map(|(id, _, _)| id.as_str()).collect();
    for c in conversation_summaries(db).await? {
        match c.character_id.as_deref().filter(|id| known.contains(id)) {
            Some(id) => by_char.entry(id.to_string()).or_default().push(c),
            None => orphans.push(c),
        }
    }

    let mut list: Vec<CharacterData> = characters
        .into_iter()
        .map(|(id, name, avatar)| {
            let conversations = by_char.remove(&id).unwrap_or_default();
            let last_activity = conversations.iter().filter_map(|c| c.last_message_at.or(c.updated_at)).max();
            CharacterData {
                conversation_count: conversations.len(),
                message_count: conversations.iter().map(|c| c.message_count).sum(),
                image_count: images.get(&id).copied().unwrap_or(0),
                world_info_book_count: books.get(&id).copied().unwrap_or(0),
                last_activity,
                conversations,
                id,
                name,
                avatar,
            }
        })
        .collect();
    list.sort_by(|a, b| b.last_activity.cmp(&a.last_activity).then_with(|| a.name.cmp(&b.name)));

    let totals = totals(db).await?;

    let base = app_data_dir.to_path_buf();
    let storage = tokio::task::spawn_blocking(move || {
        let file = |name: &str| std::fs::metadata(base.join(name)).map(|m| m.len()).unwrap_or(0);
        let mut s = StorageUsage {
            database_bytes: file("app.db") + file("app.db-wal"),
            characters_bytes: dir_size(&base.join("characters")),
            games_bytes: dir_size(&base.join("games")),
            themes_bytes: dir_size(&base.join("themes")),
            total_bytes: 0,
        };
        s.total_bytes = s.database_bytes + s.characters_bytes + s.games_bytes + s.themes_bytes;
        s
    })
    .await
    .unwrap_or_default();

    Ok(DataOverview {
        characters: list,
        orphan_conversations: orphans,
        totals,
        storage,
    })
}

pub async fn totals(db: &DbPool) -> Result<DataTotals> {
    Ok(DataTotals {
        characters: count(db, "characters").await?,
        personas: count(db, "personas").await?,
        conversations: count(db, "conversations").await?,
        messages: count(db, "messages").await?,
        world_info_books: count(db, "worldinfo_books").await?,
        prompt_presets: count(db, "prompt_presets").await?,
        regex_scripts: count(db, "regex_scripts").await?,
        character_images: count(db, "character_images").await?,
        games: count(db, "game_projects").await?,
    })
}
