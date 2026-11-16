use crate::models::conversation::*;
use sqlx::{Error, Executor, Sqlite};
use uuid::Uuid;

// =======================================
// Conversations
// =======================================
pub async fn get_all_conversations<'e, E>(executor: E) -> Result<Vec<Conversation>, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows = sqlx::query_as::<_, ConversationRow>(
        "SELECT * FROM v_conversations_with_count ORDER BY updated_at DESC",
    )
    .fetch_all(executor)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}
pub async fn get_conversation<'e, E>(executor: E, id: &str) -> Result<Conversation, Error> where E: Executor<'e, Database = Sqlite>,{
    let row = sqlx::query_as::<_, ConversationRow>("SELECT * FROM v_conversations_with_count WHERE id=?")
        .bind(id)
        .fetch_one(executor)
        .await?;
    Ok(row.into())
}
pub async fn create_conversation<'e, E>(executor: E, input: ConversationInput) -> Result<Conversation, Error> where E: Executor<'e, Database = Sqlite>,{
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let wi_json = serde_json::to_string(&input.world_info_ids).unwrap_or_default();
    let meta = serde_json::to_string(&input.meta).unwrap_or_default();

    sqlx::query(
        "INSERT INTO conversations (id,title,character_id,persona_id,world_info_ids,mode,meta,images_enabled,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,?,?,?)"
    )
    .bind(&id).bind(&input.title).bind(&input.character_id).bind(&input.persona_id).bind(&wi_json)
    .bind(&input.mode).bind(meta).bind(input.images_enabled.unwrap_or(true)).bind(now).bind(now)
    .execute(executor)
    .await?;

    Ok(Conversation {
        id,
        title: input.title,
        character_id: input.character_id,
        persona_id: input.persona_id,
        world_info_ids: input.world_info_ids,
        mode: input.mode,
        meta: input.meta,
        pinned: false,
        images_enabled: input.images_enabled.unwrap_or(true),
        created_at: now,
        updated_at: now,
        message_count: 0,
        folder_id: None,
        prompt_preset_id: None,
        api_profile_id: None,
    })
}

/** conversation 업데이트일 업데이트 */
/// 대화별 이미지 기능 스위치
pub async fn set_conversation_images_enabled<'e, E>(executor: E, id: &str, enabled: bool) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let result = sqlx::query("UPDATE conversations SET images_enabled = ? WHERE id = ?")
        .bind(enabled)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn touch_conversation<'e, E>(executor: E, id: &str) -> Result<i64, Error> where E: Executor<'e, Database = Sqlite>,{
    let now = chrono::Utc::now().timestamp_millis();
    let result = sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&id)
        .execute(executor)
        .await?;
    if result.rows_affected() == 0 {
        return Err(Error::RowNotFound);
    }
    Ok(now)
}

pub async fn delete_conversation<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let result = sqlx::query("DELETE FROM conversations WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?;

    Ok(result.rows_affected() != 0)
}

// =======================================
// Persona
// =======================================
/** 페르소나가 db에 존재하는지 확인
 * return true: 있음
 *        false: 없음
 */
pub async fn persona_exists<'e, E>(executor: E, persona_id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let char_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM personas WHERE id = ?")
        .bind(persona_id)
        .fetch_optional(executor)
        .await?;
    Ok(!char_exists.is_none())
}

pub async fn get_all_personas<'e, E>(executor: E) -> Result<Vec<Persona>, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows = sqlx::query_as::<_, PersonaRow>(
        "SELECT 
            p.id,
            p.name,
            p.avatar,
            p.description,
            (
                SELECT json_group_array(book_id) 
                FROM persona_worldinfo pw 
                WHERE pw.persona_id = p.id
            ) AS world_info_ids,
            p.tags,
            p.color,
            p.is_system_default,
            p.created_at,
            p.updated_at
        FROM personas p
        ORDER BY p.updated_at DESC"
    )
        .fetch_all(executor)
        .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}
pub async fn get_persona<'e, E>(executor: E, id: &str) -> Result<Persona, Error> where E: Executor<'e, Database = Sqlite>,{
    let row= sqlx::query_as::<_, PersonaRow>(
        "SELECT 
            p.id,
            p.name,
            p.avatar,
            p.description,
            (
                SELECT json_group_array(book_id) 
                FROM persona_worldinfo pw 
                WHERE pw.persona_id = p.id
            ) AS world_info_ids,
            p.tags,
            p.color,
            p.is_system_default,
            p.created_at,
            p.updated_at
        FROM personas p
        WHERE p.id = ?"
    )
        .bind(id)
        .fetch_one(executor)
        .await?;
    Ok(row.into())
}

pub async fn get_persona_usage<'e, E>(executor: E, id: &str) -> Result<PersonaUsage, Error> where E: Executor<'e, Database = Sqlite>,{
    let (character_count, setting_count): (i64, i64) = sqlx::query_as(
        "SELECT 
            (SELECT COUNT(*) FROM characters WHERE default_persona_id = ?1),
            (SELECT COUNT(*) FROM conv_settings WHERE default_persona_id = ?1)",
    )
    .bind(id) // ?1 자리에 매핑됩니다.
    .fetch_one(executor)
    .await?;

    let is_system_default = setting_count > 0;

    Ok(PersonaUsage {
        character_count,
        is_system_default,
    })
}

pub async fn create_persona<'e, E>(executor: E, input: PersonaInput) -> Result<Persona, Error> where E: Executor<'e, Database = Sqlite>,{
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let tags_json = serde_json::to_string(&input.tags).unwrap_or_default();

    sqlx::query(
        "INSERT INTO personas (id,name,avatar,description,tags,color,is_system_default,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,?,?)"
    )
    .bind(&id).bind(&input.name).bind(&input.avatar).bind(&input.description)
    .bind(&tags_json).bind(&input.color).bind(&input.is_system_default).bind(now).bind(now)
    .execute(executor)
    .await?;

    Ok(Persona {
        id: id,
        name: input.name,
        avatar: input.avatar,
        description: input.description,
        world_info_ids: input.world_info_ids,
        tags: input.tags,
        color: input.color,
        is_system_default: input.is_system_default,
        created_at: now,
        updated_at: now,
    })
}

pub async fn update_persona<'e, E>(executor: E, id: &str, input: PersonaInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    let now = chrono::Utc::now().timestamp_millis();
    let tags_json = serde_json::to_string(&input.tags).unwrap_or_default();

    sqlx::query(
        "
        UPDATE personas SET 
            name=?, avatar=?, description=?, tags=?, color=?, updated_at=? 
        WHERE id=?",
    )
    .bind(&input.name)
    .bind(&input.avatar)
    .bind(&input.description)
    .bind(&tags_json)
    .bind(&input.color)
    .bind(now)
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn delete_persona<'e, E>(executor: E, persona_id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let result = sqlx::query("DELETE FROM personas WHERE id = ?")
        .bind(persona_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

/// 페르소나에 연결된 월드인포 책을 통째로 바꾼다
pub async fn replace_persona_worldinfo(conn: &mut sqlx::SqliteConnection, persona_id: &str, book_ids: &[String]) -> Result<(), Error> {
    sqlx::query("DELETE FROM persona_worldinfo WHERE persona_id = ?")
        .bind(persona_id)
        .execute(&mut *conn)
        .await?;
    for book_id in book_ids {
        sqlx::query("INSERT OR IGNORE INTO persona_worldinfo (persona_id, book_id) VALUES (?, ?)")
            .bind(persona_id)
            .bind(book_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub async fn set_persona_avatar<'e, E>(executor: E, id: &str, avatar: Option<&str>) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query("UPDATE personas SET avatar = ?, updated_at = ? WHERE id = ?")
        .bind(avatar)
        .bind(now)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}

/// 캐릭터에 연결된 월드인포 책을 통째로 바꾼다
pub async fn replace_character_worldinfo(conn: &mut sqlx::SqliteConnection, character_id: &str, book_ids: &[String]) -> Result<(), Error> {
    sqlx::query("DELETE FROM character_worldinfo WHERE character_id = ?")
        .bind(character_id)
        .execute(&mut *conn)
        .await?;
    for book_id in book_ids {
        sqlx::query("INSERT OR IGNORE INTO character_worldinfo (character_id, book_id) VALUES (?, ?)")
            .bind(character_id)
            .bind(book_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub async fn set_character_avatar<'e, E>(executor: E, id: &str, avatar: Option<&str>) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query("UPDATE characters SET avatar = ?, updated_at = ? WHERE id = ?")
        .bind(avatar)
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn set_character_cover<'e, E>(executor: E, id: &str, cover: Option<&str>) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query("UPDATE characters SET cover = ?, updated_at = ? WHERE id = ?")
        .bind(cover)
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn set_character_cover_y<'e, E>(executor: E, id: &str, y: f64) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query("UPDATE characters SET cover_y = ? WHERE id = ?").bind(y.clamp(0.0, 100.0)).bind(id).execute(executor).await?;
    Ok(())
}

pub async fn get_system_default_persona<'e, E>(executor: E) -> Result<String, Error> where E: Executor<'e, Database = Sqlite>,{
    let row =
        sqlx::query_as::<_, (String,)>("SELECT default_persona_id FROM conv_settings WHERE id = 1")
            .fetch_one(executor)
            .await?;
    Ok(row.0)
}

pub async fn set_system_default_persona<'e, E>(executor: E, persona_id: &str) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query("UPDATE conv_settings SET default_persona_id = ?, updated_at = ? WHERE id = 1")
        .bind(persona_id)
        .bind(now)
        .execute(executor)
        .await?;
    Ok(())
}

// =======================================
// Character
// =======================================
/** 캐릭터가 db에 존재하는지 확인
 * return true: 있음
 *        false: 없음
 */
pub async fn char_exists<'e, E>(executor: E, character_id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let char_exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM characters WHERE id = ?")
        .bind(character_id)
        .fetch_optional(executor)
        .await?;
    Ok(!char_exists.is_none())
}

pub async fn get_character_conversation_count<'e, E>(executor: E,character_id: &str) -> Result<i64, Error> where E: Executor<'e, Database = Sqlite>,{
    let row =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM conversations WHERE character_id = ?")
            .bind(&character_id)
            .fetch_one(executor)
            .await?;

    Ok(row)
}

pub async fn get_all_characters<'e, E>(executor: E) -> Result<Vec<Character>, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows =
        sqlx::query_as::<_, CharacterRow>(
            "
            SELECT 
                c.id,
                c.name,
                c.avatar,
                c.cover,
                c.cover_y,
                c.description,
                c.alternate_descriptions,
                c.first_message,
                c.alternate_first_message,
                c.example_dialogue,
                c.system_prompt_override,
                c.creator_notes,
                (
                    SELECT json_group_array(book_id) 
                    FROM character_worldinfo cw 
                    WHERE cw.character_id = c.id
                ) AS world_info_ids,
                c.color,
                c.tags, 
                c.category, 
                c.default_persona_id, 
                c.default_prompt_preset_id, 
                c.favorite, 
                (
                    SELECT COUNT(*) FROM conversations conv WHERE conv.character_id = c.id
                ) AS message_count,
                c.created_at,
                c.updated_at 
            FROM characters c
            ORDER BY updated_at DESC
            ")
            .fetch_all(executor)
            .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_character<'e, E>(executor: E, id: &str) -> Result<Character, Error> where E: Executor<'e, Database = Sqlite>,{
    let row =
        sqlx::query_as::<_, CharacterRow>(
            "
            SELECT 
                c.id,
                c.name,
                c.avatar,
                c.cover,
                c.cover_y,
                c.description,
                c.alternate_descriptions,
                c.first_message,
                c.alternate_first_message,
                c.example_dialogue,
                c.system_prompt_override,
                c.creator_notes,
                (
                    SELECT json_group_array(book_id) 
                    FROM character_worldinfo cw 
                    WHERE cw.character_id = c.id
                ) AS world_info_ids,
                c.color,
                c.tags, 
                c.category, 
                c.default_persona_id, 
                c.default_prompt_preset_id, 
                c.favorite, 
                (
                    SELECT COUNT(*) FROM conversations conv WHERE conv.character_id = c.id
                ) AS message_count,
                c.created_at,
                c.updated_at 
            FROM characters c
            WHERE c.id=?"
        )
            .bind(id)
            .fetch_one(executor)
            .await?;

    Ok(row.into())
}

pub async fn get_list_characters<'e, E>(executor: E) -> Result<Vec<CharacterOption>, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows = sqlx::query_as::<_, CharacterOption>(
        "SELECT id, name FROM characters ORDER BY updated_at DESC",
    )
    .fetch_all(executor)
    .await?;

    Ok(rows)
}

pub async fn create_character<'e, E>(executor: E, input: CharacterInput) -> Result<Character, Error> where E: Executor<'e, Database = Sqlite>,{
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let tags_json = serde_json::to_string(&input.tags).unwrap_or_default();
    let alternate_descriptions = serde_json::to_string(&input.alternate_descriptions).unwrap_or_default();
    let alternate_first_message = serde_json::to_string(&input.alternate_first_message).unwrap_or_default();

    sqlx::query(
        "INSERT INTO characters (id,name,avatar,description,alternate_descriptions,first_message,alternate_first_message,example_dialogue,system_prompt_override,creator_notes,tags,category,color,default_persona_id,default_prompt_preset_id,favorite,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.avatar)
    .bind(&input.description)
    .bind(alternate_descriptions)
    .bind(&input.first_message)
    .bind(alternate_first_message)
    .bind(&input.example_dialogue)
    .bind(&input.system_prompt_override)
    .bind(&input.creator_notes)
    .bind(&tags_json)
    .bind(&input.category)
    .bind(&input.color)
    .bind(&input.default_persona_id)
    .bind(&input.default_prompt_preset_id)
    .bind(input.favorite)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(Character {
        id,
        name: input.name,
        avatar: input.avatar,
        cover: None,
        cover_y: 50.0,
        description: input.description,
        alternate_descriptions: input.alternate_descriptions,
        first_message: input.first_message,
        alternate_first_message: input.alternate_first_message,
        example_dialogue: input.example_dialogue,
        system_prompt_override: input.system_prompt_override,
        creator_notes: input.creator_notes,
        world_info_ids: input.world_info_ids,
        color: input.color,
        tags: input.tags,
        category: input.category,
        default_persona_id: input.default_persona_id,
        default_prompt_preset_id: input.default_prompt_preset_id,
        favorite: input.favorite,
        message_count: 0,
        created_at: now,
        updated_at: now,
    })
}

pub async fn update_character<'e, E>(executor: E,id: &str,input: &CharacterInput,) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    let now = chrono::Utc::now().timestamp_millis();
    let tags_json = serde_json::to_string(&input.tags).unwrap_or_default();
    let alternate_descriptions = serde_json::to_string(&input.alternate_descriptions).unwrap_or_default();
    let alternate_first_message = serde_json::to_string(&input.alternate_first_message).unwrap_or_default();

    sqlx::query(
        "
        UPDATE characters SET 
            name=?, 
            avatar=?, 
            description=?, 
            alternate_descriptions=?, 
            first_message=?, 
            alternate_first_message =?, 
            example_dialogue=?, 
            system_prompt_override=?, 
            creator_notes=?,
            tags=?, 
            category=?, 
            color=?, 
            default_persona_id=?,
            default_prompt_preset_id=?,
            favorite=?,
            updated_at=? 
         WHERE id=?",
    )
    .bind(&input.name)
    .bind(&input.avatar)
    .bind(&input.description)
    .bind(alternate_descriptions)
    .bind(&input.first_message)
    .bind(alternate_first_message)
    .bind(&input.example_dialogue)
    .bind(&input.system_prompt_override)
    .bind(&input.creator_notes)
    .bind(&tags_json)
    .bind(&input.category)
    .bind(&input.color)
    .bind(&input.default_persona_id)
    .bind(&input.default_prompt_preset_id)
    .bind(&input.favorite)
    .bind(now)
    .bind(&id)
    .execute(executor)
    .await?;

    Ok(())
}
pub async fn update_character_is_favorite<'e, E>(executor: E, id: &str, is_favorite: bool) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query(
        "
        UPDATE characters SET favorite=? WHERE id=?",
    )
    .bind(is_favorite)
    .bind(&id)
    .execute(executor)
    .await?;

    Ok(())
}
pub async fn delete_character<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let result = sqlx::query("DELETE FROM characters WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?;

    Ok(result.rows_affected() != 0)
}

// =======================================
// WorldInfo
// =======================================
pub async fn get_all_worldinfo_books<'e, E>(executor: E) -> Result<Vec<WorldInfoBook>, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows = sqlx::query_as::<_, WorldInfoBook>(
        "SELECT * FROM worldinfo_books ORDER BY updated_at DESC",
    )
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

pub async fn get_worldinfo_book<'e, E>(executor: E, id: &str) -> Result<WorldInfoBook, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows = sqlx::query_as::<_, WorldInfoBook>(
        "SELECT * FROM worldinfo_books WHERE id = ? ORDER BY updated_at DESC",
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(rows)
}

pub async fn create_worldinfo_book<'e, E>(executor: E, input: WorldInfoBookInput,) -> Result<WorldInfoBook, Error> where E: Executor<'e, Database = Sqlite>,{
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query("INSERT INTO worldinfo_books (id,name,description,created_at,updated_at) VALUES (?,?,?,?,?)")
        .bind(&id).bind(&input.name).bind(&input.description).bind(now).bind(now)
        .execute(executor)
        .await?;
    Ok(WorldInfoBook {
        id,
        name: input.name,
        description: input.description,
        created_at: now,
        updated_at: now,
    })
}

pub async fn delete_worldinfo_book<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let result = sqlx::query("DELETE FROM worldinfo_books WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------
// WorldInfo Entry
// ---------------------------------------
pub async fn get_all_conv_worldinfo_book_with_entries<'e, E>(executor: E, conversation_id: &str, character_id: &str, persona_id: &str) -> Result<Vec<WorldInfoEntry>, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows = sqlx::query_as::<_, WorldInfoEntryRow>(
    r#"
        SELECT id, book_id, keys, secondary_keys, content, enabled, priority, constant
        FROM worldinfo_entries
        WHERE enabled = 1
        AND book_id IN (
            -- 대화에 직접 붙인 로어북은 conversations.world_info_ids(JSON 배열)에 있다
            -- (예전에는 존재하지 않는 conversation_worldinfo 테이블을 읽어서 매번 실패했음)
            SELECT j.value
            FROM conversations c,
                 json_each(CASE WHEN json_valid(c.world_info_ids) THEN c.world_info_ids ELSE '[]' END) j
            WHERE c.id = ?
            UNION
            SELECT book_id FROM character_worldinfo WHERE character_id = ?
            UNION
            SELECT book_id FROM persona_worldinfo WHERE persona_id = ?
        )
        "#,
    )
    .bind(conversation_id)
    .bind(character_id)
    .bind(persona_id)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_all_worldinfo_entries<'e, E>(executor: E, book_id: &str,) -> Result<Vec<WorldInfoEntry>, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows = sqlx::query_as::<_, WorldInfoEntryRow>(
        "SELECT * FROM worldinfo_entries WHERE book_id = ? ORDER BY priority DESC",
    )
    .bind(&book_id)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn upsert_worldinfo_entry<'e, E>(executor: E, input: WorldInfoEntryInput,) -> Result<WorldInfoEntry, Error> where E: Executor<'e, Database = Sqlite>,{
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let keys_json = serde_json::to_string(&input.keys).unwrap_or_default();
    let skeys_json = serde_json::to_string(&input.secondary_keys).unwrap_or_default();

    sqlx::query(
        "INSERT INTO worldinfo_entries (id, book_id, keys, secondary_keys, content, enabled, priority, constant, comment)
         VALUES (?,?,?,?,?,?,?,?,?)
         ON CONFLICT(id) DO UPDATE SET
           keys=excluded.keys, 
           secondary_keys=excluded.secondary_keys, 
           content=excluded.content,
           enabled=excluded.enabled, priority=excluded.priority, 
           constant=excluded.constant, comment=excluded.comment"
    )
    .bind(&id)
    .bind(&input.book_id)
    .bind(&keys_json)
    .bind(&skeys_json)
    .bind(&input.content)
    .bind(input.enabled as i64)
    .bind(input.priority)
    .bind(input.constant as i64)
    .bind(&input.comment)
    .execute(executor)
    .await?;

    Ok(WorldInfoEntry {
        comment: input.comment,
        id,
        book_id: input.book_id,
        keys: input.keys,
        secondary_keys: input.secondary_keys,
        content: input.content,
        enabled: input.enabled,
        priority: input.priority,
        constant: input.constant,
    })
}

/// 책 이름·설명 바꾸기
pub async fn update_worldinfo_book<'e, E>(executor: E, id: &str, input: &WorldInfoBookInput) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let r = sqlx::query("UPDATE worldinfo_books SET name = ?, description = ?, updated_at = ? WHERE id = ?")
        .bind(&input.name)
        .bind(&input.description)
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(id)
        .execute(executor)
        .await?;
    Ok(r.rows_affected() != 0)
}

/// 책마다 (전체 항목 수, 켜진 항목 수)
pub async fn worldinfo_entry_counts<'e, E>(executor: E) -> Result<Vec<(String, i64, i64)>, Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query_as::<_, (String, i64, i64)>("SELECT book_id, COUNT(*), COALESCE(SUM(enabled), 0) FROM worldinfo_entries GROUP BY book_id")
        .fetch_all(executor)
        .await
}

/// 대화에 직접 붙인 로어북 바꾸기
pub async fn set_conversation_world_info<'e, E>(executor: E, id: &str, book_ids: &[String]) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let r = sqlx::query("UPDATE conversations SET world_info_ids = ?, updated_at = ? WHERE id = ?")
        .bind(serde_json::to_string(book_ids).unwrap_or_else(|_| "[]".into()))
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(id)
        .execute(executor)
        .await?;
    Ok(r.rows_affected() != 0)
}

pub async fn delete_worldinfo_entry<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let result = sqlx::query("DELETE FROM worldinfo_entries WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// =======================================
// character_worldinfo
// =======================================
pub async fn get_character_world_info_ids<'e, E>(executor: E, character_id: &str) -> Result<Vec<String>, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT book_id FROM character_worldinfo WHERE character_id = ?",
    )
    .bind(character_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

pub async fn create_character_worldinfo_ids<'e, E>(executor: E, character_id: &str, book_id: &str,) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query("INSERT INTO character_worldinfo (character_id, book_id) VALUES (?,?)")
        .bind(character_id)
        .bind(book_id)
        .execute(executor)
        .await?;
    Ok(())
}
pub async fn delete_character_worldinfo_ids<'e, E>(executor: E, character_id: &str,) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query("DELETE FROM character_worldinfo WHERE character_id = ?")
        .bind(character_id)
        .execute(executor)
        .await?;
    Ok(())
}

// =======================================
// persona_worldinfo
// =======================================

pub async fn get_persona_worldinfo_ids<'e, E>(executor: E, persona_id: &str,) -> Result<Vec<String>, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT book_id FROM persona_worldinfo WHERE persona_id = ?",
    )
    .bind(persona_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

pub async fn create_persona_worldinfo_ids<'e, E>(executor: E,persona_id: &str,book_id: &str,) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query("INSERT INTO persona_worldinfo (persona_id, book_id) VALUES (?,?)")
        .bind(persona_id)
        .bind(book_id)
        .execute(executor)
        .await?;
    Ok(())
}
// =======================================
// Message
// =======================================
/** prams
 * - limit: 불러오는 메시지 개수 제한
 * - before: 불러올 메시지 전 메시지
 */
pub async fn get_messages<'e, E>(executor: E, conversation_id: &str, limit: i64, before: Option<i64>,) -> Result<Vec<Message>, Error> where E: Executor<'e, Database = Sqlite>,{
    let rows = match before {
            Some(b) => sqlx::query_as::<_, MessageRow>(
                "SELECT * FROM messages WHERE conversation_id = ? AND created_at < ? ORDER BY created_at DESC, rowid DESC LIMIT ?",
            )
            .bind(&conversation_id)
            .bind(b)
            .bind(limit),
        None => sqlx::query_as::<_, MessageRow>(
                "SELECT * FROM messages WHERE conversation_id = ? ORDER BY created_at DESC, rowid DESC LIMIT ?",
            )
            .bind(&conversation_id)
            .bind(limit),
    }
    .fetch_all(executor)
    .await?;

    let mut msgs: Vec<Message> = rows.into_iter().map(Into::into).collect();
    msgs.reverse();
    Ok(msgs)
}

pub async fn get_message_meta<'e, E>(executor: E, id: &str,) -> Result<Option<serde_json::Value>, Error> where E: Executor<'e, Database = Sqlite>,{
    let row = sqlx::query_scalar::<_, String>("SELECT meta FROM messages WHERE id = ?")
        .bind(id)
        .fetch_optional(executor)
        .await?;
    Ok(row
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default())
}
pub async fn append_message<'e, E>(executor: E, input: MessageInput) -> Result<Message, Error> where E: Executor<'e, Database = Sqlite>,{
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    let meta = serde_json::to_string(&input.meta).unwrap_or_default();

    sqlx::query(
        "INSERT INTO messages (id,conversation_id,seq,role,name,content,character_id,meta,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&input.conversation_id)
    .bind(&input.seq)
    .bind(&input.role)
    .bind(&input.name)
    .bind(&input.content)
    .bind(&input.character_id)
    .bind(meta)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(Message {
        id,
        conversation_id: input.conversation_id,
        seq: input.seq,
        role: input.role,
        name: input.name,
        content: input.content,
        character_id: input.character_id,
        meta: input.meta,
        created_at: now,
        updated_at: now,
    })
}

/// 가져오기용: 작성 시각을 지정해서 메시지 저장 (대화 기록 정렬이 created_at 기준이라 원래 시각이 필요)
pub async fn insert_message_at<'e, E>(executor: E, input: MessageInput, created_at: i64) -> Result<(), Error> where E: Executor<'e, Database = Sqlite>,{
    let id = Uuid::new_v4().to_string();
    let meta = input.meta.as_ref().map(|m| m.to_string());
    sqlx::query(
        "INSERT INTO messages (id,conversation_id,seq,role,name,content,character_id,meta,created_at,updated_at)
         VALUES (?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&input.conversation_id)
    .bind(input.seq)
    .bind(&input.role)
    .bind(&input.name)
    .bind(&input.content)
    .bind(&input.character_id)
    .bind(meta)
    .bind(created_at)
    .bind(created_at)
    .execute(executor)
    .await?;
    Ok(())
}

/// 이름으로 캐릭터 찾기 (대소문자 무시). 동명이인이 있을 수 있어 목록으로 돌려준다.
pub async fn find_character_ids_by_name<'e, E>(executor: E, name: &str) -> Result<Vec<String>, Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query_scalar("SELECT id FROM characters WHERE name = ? COLLATE NOCASE")
        .bind(name.trim())
        .fetch_all(executor)
        .await
}

/// 이름으로 페르소나 찾기 (대소문자 무시)
pub async fn find_persona_ids_by_name<'e, E>(executor: E, name: &str) -> Result<Vec<String>, Error> where E: Executor<'e, Database = Sqlite>,{
    sqlx::query_scalar("SELECT id FROM personas WHERE name = ? COLLATE NOCASE")
        .bind(name.trim())
        .fetch_all(executor)
        .await
}

pub async fn update_message<'e, E>(executor: E, id: &str, content: &str, meta: &Option<serde_json::Value>) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let now = chrono::Utc::now().timestamp_millis();
    let meta_str = serde_json::to_string(&meta).unwrap_or_default();
    let result = sqlx::query("UPDATE messages SET content = ?, meta = ?, updated_at=? WHERE id = ?")
        .bind(content)
        .bind(meta_str)
        .bind(now)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn update_message_content<'e, E>(executor: E, id: &str, content: &str,) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let now = chrono::Utc::now().timestamp_millis();
    let result = sqlx::query("UPDATE messages SET content = ?, updated_at = ? WHERE id = ?")
        .bind(content)
        .bind(now)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn update_message_meta<'e, E>(executor: E, id: &str, meta: serde_json::Value) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let now = chrono::Utc::now().timestamp_millis();
    let meta_string = serde_json::to_string(&meta).unwrap_or_default();
    let result = sqlx::query("UPDATE messages SET meta = ?, updated_at = ? WHERE id = ?")
        .bind(meta_string)
        .bind(now)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn delete_message<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let result = sqlx::query("DELETE FROM messages WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}
