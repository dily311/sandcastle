//! 대화 화면용 쿼리: 폴더, 대화별 설정, 이름/고정/이동, 분기, 보낸 프롬프트 기록
use sqlx::{Error, Executor, Sqlite, SqlitePool};
use uuid::Uuid;

use crate::models::conversation::{ConversationFolder, ConversationOptions, Message, MessageInput, MessageRow};

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

// ---------- 폴더 ----------
pub async fn list_folders<'e, E>(executor: E, character_id: &str) -> Result<Vec<ConversationFolder>, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, ConversationFolder>("SELECT * FROM conversation_folders WHERE character_id = ? ORDER BY sort_order, created_at")
        .bind(character_id)
        .fetch_all(executor)
        .await
}

pub async fn create_folder(pool: &SqlitePool, character_id: &str, name: &str) -> Result<ConversationFolder, Error> {
    let next: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM conversation_folders WHERE character_id = ?")
        .bind(character_id)
        .fetch_one(pool)
        .await?;
    let folder = ConversationFolder {
        id: Uuid::new_v4().to_string(),
        character_id: character_id.to_string(),
        name: name.to_string(),
        sort_order: next,
        created_at: now(),
        updated_at: now(),
    };
    sqlx::query("INSERT INTO conversation_folders (id,character_id,name,sort_order,created_at,updated_at) VALUES (?,?,?,?,?,?)")
        .bind(&folder.id)
        .bind(&folder.character_id)
        .bind(&folder.name)
        .bind(folder.sort_order)
        .bind(folder.created_at)
        .bind(folder.updated_at)
        .execute(pool)
        .await?;
    Ok(folder)
}

pub async fn rename_folder<'e, E>(executor: E, id: &str, name: &str) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let r = sqlx::query("UPDATE conversation_folders SET name = ?, updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(now())
        .bind(id)
        .execute(executor)
        .await?;
    Ok(r.rows_affected() != 0)
}

/// 폴더만 지운다. 안의 대화는 폴더 밖으로 (FK ON DELETE SET NULL)
pub async fn delete_folder<'e, E>(executor: E, id: &str) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let r = sqlx::query("DELETE FROM conversation_folders WHERE id = ?").bind(id).execute(executor).await?;
    Ok(r.rows_affected() != 0)
}

// ---------- 대화 ----------
pub async fn rename_conversation<'e, E>(executor: E, id: &str, title: &str) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let r = sqlx::query("UPDATE conversations SET title = ? WHERE id = ?")
        .bind(title)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(r.rows_affected() != 0)
}

pub async fn set_pinned<'e, E>(executor: E, id: &str, pinned: bool) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let r = sqlx::query("UPDATE conversations SET pinned = ? WHERE id = ?")
        .bind(pinned)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(r.rows_affected() != 0)
}

pub async fn move_to_folder<'e, E>(executor: E, id: &str, folder_id: Option<&str>) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let r = sqlx::query("UPDATE conversations SET folder_id = ? WHERE id = ?")
        .bind(folder_id)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(r.rows_affected() != 0)
}

pub async fn set_options<'e, E>(executor: E, id: &str, o: &ConversationOptions) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    // 모델은 API 프로필에 딸린 이름이라, 대화의 API가 (정해진 것에서) 다른 것으로 바뀌면 NULL로 돌아간다 (다른 공급자의 모델 이름이 남아 요청이 실패하는 일을 막음).
    // 앱 기본 API를 따라가던 대화(NULL)가 API와 모델을 한꺼번에 정하는 경우는 모델을 둔다
    let r = sqlx::query(
        "UPDATE conversations SET persona_id = ?, prompt_preset_id = ?,
             model = CASE WHEN api_profile_id IS ? OR api_profile_id IS NULL THEN ? ELSE NULL END,
             api_profile_id = ?
         WHERE id = ?",
    )
    .bind(&o.persona_id)
    .bind(&o.prompt_preset_id)
    .bind(&o.api_profile_id)
    .bind(o.model.as_deref().map(str::trim).filter(|m| !m.is_empty()))
    .bind(&o.api_profile_id)
    .bind(id)
    .execute(executor)
    .await?;
    Ok(r.rows_affected() != 0)
}

/// 대화를 새 대화로 복사. `upto`가 있으면 그 메시지(포함)까지만 — 분기 나누기.
/// 메시지 시각은 그대로 둔다 (기록 정렬이 created_at 기준).
pub async fn copy_conversation(pool: &SqlitePool, id: &str, title: &str, upto: Option<&str>) -> Result<String, Error> {
    let mut tx = pool.begin().await?;
    let new_id = Uuid::new_v4().to_string();
    let t = now();
    sqlx::query(
        "INSERT INTO conversations (id,title,character_id,persona_id,prompt_preset_id,world_info_ids,mode,meta,pinned,images_enabled,folder_id,api_profile_id,model,created_at,updated_at)
         SELECT ?,?,character_id,persona_id,prompt_preset_id,world_info_ids,mode,meta,0,images_enabled,folder_id,api_profile_id,model,?,? FROM conversations WHERE id = ?",
    )
    .bind(&new_id)
    .bind(title)
    .bind(t)
    .bind(t)
    .bind(id)
    .execute(&mut *tx)
    .await?;

    let rows = sqlx::query_as::<_, MessageRow>("SELECT * FROM messages WHERE conversation_id = ? ORDER BY created_at, rowid")
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
    if let Some(stop) = upto {
        if !rows.iter().any(|r| r.id == stop) {
            return Err(Error::RowNotFound);
        }
    }
    for r in rows {
        let last = upto == Some(r.id.as_str());
        sqlx::query(
            "INSERT INTO messages (id,conversation_id,seq,role,name,content,character_id,meta,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?,?,?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&new_id)
        .bind(r.seq)
        .bind(&r.role)
        .bind(&r.name)
        .bind(&r.content)
        .bind(&r.character_id)
        .bind(&r.meta)
        .bind(r.created_at)
        .bind(r.updated_at)
        .execute(&mut *tx)
        .await?;
        if last {
            break;
        }
    }
    tx.commit().await?;
    Ok(new_id)
}

// ---------- 메시지 ----------
pub async fn get_message<'e, E>(executor: E, id: &str) -> Result<Message, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let row = sqlx::query_as::<_, MessageRow>("SELECT * FROM messages WHERE id = ?")
        .bind(id)
        .fetch_one(executor)
        .await?;
    Ok(row.into())
}

pub async fn last_message<'e, E>(executor: E, conversation_id: &str) -> Result<Option<Message>, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let row = sqlx::query_as::<_, MessageRow>("SELECT * FROM messages WHERE conversation_id = ? ORDER BY created_at DESC, rowid DESC LIMIT 1")
        .bind(conversation_id)
        .fetch_optional(executor)
        .await?;
    Ok(row.map(Into::into))
}

/// 다음 메시지 번호 (대화 안에서 seq는 겹치면 안 됨)
pub async fn next_seq<'e, E>(executor: E, conversation_id: &str) -> Result<i64, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_scalar("SELECT COALESCE(MAX(seq), 0) + 1 FROM messages WHERE conversation_id = ?")
        .bind(conversation_id)
        .fetch_one(executor)
        .await
}

/// 새 메시지 저장 (seq는 알아서). 저장된 메시지를 돌려준다.
pub async fn push_message(pool: &SqlitePool, mut input: MessageInput) -> Result<Message, Error> {
    input.seq = next_seq(pool, &input.conversation_id).await?;
    let msg = super::conversation::append_message(pool, input).await?;
    super::conversation::touch_conversation(pool, &msg.conversation_id).await?;
    Ok(msg)
}

pub async fn save_prompt<'e, E>(executor: E, message_id: &str, prompt: &serde_json::Value) -> Result<(), Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query("INSERT INTO message_prompts (message_id, prompt, created_at) VALUES (?,?,?) ON CONFLICT(message_id) DO UPDATE SET prompt = excluded.prompt, created_at = excluded.created_at")
        .bind(message_id)
        .bind(prompt.to_string())
        .bind(now())
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn get_prompt<'e, E>(executor: E, message_id: &str) -> Result<Option<serde_json::Value>, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let s: Option<String> = sqlx::query_scalar("SELECT prompt FROM message_prompts WHERE message_id = ?")
        .bind(message_id)
        .fetch_optional(executor)
        .await?;
    Ok(s.and_then(|s| serde_json::from_str(&s).ok()))
}

// ---------- 대화 meta ----------
/// 대화 meta에 키를 합친다 (JSON merge patch: 값이 null이면 그 키를 지움)
pub async fn patch_conversation_meta(pool: &SqlitePool, id: &str, patch: &serde_json::Value) -> Result<(), Error> {
    sqlx::query("UPDATE conversations SET meta = json_patch(COALESCE(meta, '{}'), ?), updated_at = ? WHERE id = ?")
        .bind(patch.to_string())
        .bind(now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
