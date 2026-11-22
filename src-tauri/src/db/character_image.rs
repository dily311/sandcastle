use sqlx::{Error, Executor, Sqlite};

use crate::models::character_image::CharacterImageRow;

const COLUMNS: &str = "id, character_id, key, file_path, thumb_path, enabled, sort_order, created_at, updated_at, folder";

pub async fn get_character_images<'e, E>(executor: E, character_id: &str, enabled_only: bool) -> Result<Vec<CharacterImageRow>, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let sql = format!(
        "SELECT {COLUMNS} FROM character_images WHERE character_id = ? {} ORDER BY sort_order, created_at",
        if enabled_only { "AND enabled = 1" } else { "" }
    );
    sqlx::query_as::<_, CharacterImageRow>(sqlx::AssertSqlSafe(sql))
        .bind(character_id)
        .fetch_all(executor)
        .await
}

pub async fn get_character_image<'e, E>(executor: E, id: &str) -> Result<CharacterImageRow, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let sql = format!("SELECT {COLUMNS} FROM character_images WHERE id = ?");
    sqlx::query_as::<_, CharacterImageRow>(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_one(executor)
        .await
}

/// 같은 캐릭터에 같은 키(대소문자 무시)가 있는지. `except_id`는 수정 중인 자기 자신 제외용
pub async fn image_key_exists<'e, E>(executor: E, character_id: &str, folder: &str, key: &str, except_id: Option<&str>) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM character_images WHERE character_id = ? AND folder = ? AND key = ? COLLATE NOCASE AND id != ?")
        .bind(character_id)
        .bind(folder)
        .bind(key)
        .bind(except_id.unwrap_or_default())
        .fetch_one(executor)
        .await?;
    Ok(count > 0)
}

pub async fn insert_character_image<'e, E>(executor: E, row: &CharacterImageRow) -> Result<(), Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        r#"
        INSERT INTO character_images
            (id, character_id, key, file_path, thumb_path, enabled, sort_order, created_at, updated_at, folder)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&row.id)
    .bind(&row.character_id)
    .bind(&row.key)
    .bind(&row.file_path)
    .bind(&row.thumb_path)
    .bind(row.enabled)
    .bind(row.sort_order)
    .bind(row.created_at)
    .bind(row.updated_at)
    .bind(&row.folder)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update_character_image<'e, E>(executor: E, id: &str, key: &str, enabled: bool, sort_order: i64, folder: &str) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let res = sqlx::query("UPDATE character_images SET key = ?, enabled = ?, sort_order = ?, folder = ?, updated_at = ? WHERE id = ?")
        .bind(key)
        .bind(enabled)
        .bind(sort_order)
        .bind(folder)
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(id)
        .execute(executor)
        .await?;
    Ok(res.rows_affected() != 0)
}

pub async fn delete_character_image<'e, E>(executor: E, id: &str) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let res = sqlx::query("DELETE FROM character_images WHERE id = ?").bind(id).execute(executor).await?;
    Ok(res.rows_affected() != 0)
}

pub async fn next_image_sort_order<'e, E>(executor: E, character_id: &str) -> Result<i64, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM character_images WHERE character_id = ?")
        .bind(character_id)
        .fetch_one(executor)
        .await
}

/// 캐릭터가 쓸 판 폴더 (None = 전체, 5-E-21)
pub async fn get_asset_version<'e, E>(executor: E, character_id: &str) -> Result<Option<String>, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    Ok(sqlx::query_scalar::<_, Option<String>>("SELECT asset_version FROM characters WHERE id = ?")
        .bind(character_id)
        .fetch_optional(executor)
        .await?
        .flatten()
        .filter(|v| !v.is_empty()))
}

pub async fn set_asset_version<'e, E>(executor: E, character_id: &str, version: Option<&str>) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let r = sqlx::query("UPDATE characters SET asset_version = ? WHERE id = ?")
        .bind(version.filter(|v| !v.is_empty()))
        .bind(character_id)
        .execute(executor)
        .await?;
    Ok(r.rows_affected() != 0)
}
