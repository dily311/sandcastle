use crate::models::ai_service::*;
use sqlx::{Error, Executor, Sqlite};
use uuid::Uuid;

// =======================================
// API Profile
// =======================================
pub async fn get_all_api_profile<'e, E>(executor: E) -> Result<Vec<ApiProfiles>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, ApiProfiles>(
        r#"
        SELECT * FROM api_profiles
    "#,
    )
    .fetch_all(executor)
    .await?;
    Ok(rows)
}
pub async fn get_api_profile<'e, E>(executor: E, id: &str) -> Result<ApiProfiles, Error> where E: Executor<'e, Database = Sqlite> {
    let row = sqlx::query_as::<_, ApiProfiles>(
        r#"
        SELECT * FROM api_profiles WHERE id = ?
    "#,
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(row)
}

pub async fn create_api_profile<'e, E>(executor: E, input: ApiProfilesInput,) -> Result<ApiProfiles, Error> where E: Executor<'e, Database = Sqlite> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();

    sqlx::query(r#"
        INSERT INTO api_profiles (id, name, provider,format_type, base_url, model, created_at, updated_at) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    "#)
    .bind(&id)
    .bind(&input.name)
    .bind(&input.provider)
    .bind(&input.format_type)
    .bind(&input.base_url)
    .bind(&input.model)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(ApiProfiles {
        id,
        name: input.name,
        provider: input.provider,
        format_type: input.format_type,
        base_url: input.base_url,
        model: input.model,
        created_at: now,
        updated_at: now,
    })
}

pub async fn update_api_profile<'e, E>(executor: E, id: &str, input: ApiProfilesInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(r#"
        UPDATE api_profiles SET (name, provider,format_type, base_url, model,  updated_at) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        WHERE id=?

    "#)
    .bind(&input.name)
    .bind(&input.provider)
    .bind(&input.format_type)
    .bind(&input.base_url)
    .bind(&input.model)
    .bind(now)
    .bind(id)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn delete_api_profile<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM api_profiles WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}
// =======================================
// 프롬프트
// =======================================
pub async fn get_all_prompt_presets<'e, E>(executor: E) -> Result<Vec<PromptPresets>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, PromptPresetsRows>(
        r#"
        SELECT 
            id, name, description, preset_type, source_format, config_json, settings_json, compatibility_json, created_at, updated_at
        FROM prompt_presets
    "#,
    )
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}
pub async fn get_prompt_preset<'e, E>(executor: E, id: &str) -> Result<PromptPresets, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, PromptPresetsRows>(
        r#"
        SELECT
            id, name, description, preset_type, source_format, config_json, settings_json, compatibility_json, created_at, updated_at
        FROM prompt_presets 
        where id = ?
    "#,
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(rows.into())
}

/// 프롬프트 프리셋 생성/수정. 저장된 id를 돌려준다.
pub async fn upsert_prompt_preset<'e, E>(executor: E, input: PromptPresetsInput) -> Result<String, Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    let config_json = input.config_json.to_string();
    // 값이 없으면 빈 문자열이 아니라 NULL로 저장
    let settings_json = input.settings_json.as_ref().map(|v| v.to_string());
    let compatibility_json = input.compatibility_json.as_ref().map(|v| v.to_string());
    sqlx::query(r#"
        INSERT INTO prompt_presets (id, name, description, preset_type, source_format, config_json, settings_json, compatibility_json, created_at, updated_at) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name=excluded.name, 
            description=excluded.description, 
            preset_type=excluded.preset_type, 
            source_format=excluded.source_format, 
            config_json=excluded.config_json, 
            settings_json=excluded.settings_json, 
            compatibility_json=excluded.compatibility_json, 
            updated_at=excluded.updated_at

    "#)
    .bind(&id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.preset_type)
    .bind(&input.source_format)
    .bind(config_json)
    .bind(settings_json)
    .bind(compatibility_json)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(id)
}

pub async fn delete_prompt_preset<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM prompt_presets WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// prompt_blocks
pub async fn get_all_prompt_blocks<'e, E>(executor: E) -> Result<Vec<PromptBlocks>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, PromptBlocksRows>(
        r#"
        SELECT 
            id, preset_id, identifier, block_type, name, role, content, enabled, position, depth, sort_order, triggers_json, extra_json, created_at, updated_at
        FROM prompt_blocks
    "#,
    )
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}
/// 프리셋에 속한 블록 (정렬 순서대로)
pub async fn get_prompt_blocks_by_preset<'e, E>(executor: E, preset_id: &str) -> Result<Vec<PromptBlocks>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, PromptBlocksRows>(
        r#"
        SELECT
            id, preset_id, identifier, block_type, name, role, content, enabled, position, COALESCE(depth, 0) AS depth, sort_order, triggers_json, extra_json, created_at, updated_at
        FROM prompt_blocks
        WHERE preset_id = ?
        ORDER BY sort_order ASC
    "#,
    )
    .bind(preset_id)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_prompt_block<'e, E>(executor: E, id: &str) -> Result<PromptBlocks, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, PromptBlocksRows>(
        r#"
        SELECT
            id, preset_id, identifier, block_type, name, role, content, enabled, position, depth, sort_order, triggers_json, extra_json, created_at, updated_at
        FROM prompt_blocks 
        where id = ?
    "#,
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(rows.into())
}

pub async fn upsert_prompt_block<'e, E>(executor: E, preset_id: &str, input: PromptBlocksInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    // 이전에는 문자열을 한 번 더 JSON 인코딩해서 triggers가 무시되었다
    let triggers_json = if input.triggers_json.is_array() { input.triggers_json.to_string() } else { "[]".to_string() };
    let extra_json = input.extra_json.as_ref().map(|v| v.to_string());
    sqlx::query(r#"
        INSERT INTO prompt_blocks (id, preset_id, identifier, block_type, name, role, content, enabled, position, depth, sort_order, triggers_json, extra_json, created_at, updated_at) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            preset_id=excluded.preset_id,
            identifier=excluded.identifier,
            block_type=excluded.block_type,
            name=excluded.name,
            role=excluded.role,
            content=excluded.content,
            enabled=excluded.enabled,
            position=excluded.position,
            depth=excluded.depth,
            sort_order=excluded.sort_order,
            triggers_json=excluded.triggers_json,
            extra_json=excluded.extra_json,
            updated_at=excluded.updated_at

    "#)
    .bind(id)
    .bind(preset_id)
    .bind(&input.identifier)
    .bind(&input.block_type)
    .bind(&input.name)
    .bind(&input.role)
    .bind(&input.content)
    .bind(&input.enabled)
    .bind(&input.position)
    .bind(&input.depth)
    .bind(&input.sort_order)
    .bind(triggers_json)
    .bind(extra_json)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn delete_prompt_block<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM prompt_blocks WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}
/// 프리셋의 블록을 통째로 바꾼다 (목록에 없는 기존 블록은 삭제). 트랜잭션 안에서 부를 것.
pub async fn replace_prompt_blocks(
    conn: &mut sqlx::SqliteConnection,
    preset_id: &str,
    blocks: Vec<PromptBlocksInput>,
) -> Result<(), Error> {
    let keep: Vec<String> = blocks.iter().filter_map(|b| b.id.clone()).collect();
    let existing: Vec<String> = sqlx::query_scalar("SELECT id FROM prompt_blocks WHERE preset_id = ?")
        .bind(preset_id)
        .fetch_all(&mut *conn)
        .await?;
    for id in existing.iter().filter(|id| !keep.contains(id)) {
        sqlx::query("DELETE FROM prompt_blocks WHERE id = ?").bind(id).execute(&mut *conn).await?;
    }
    for (i, mut b) in blocks.into_iter().enumerate() {
        b.preset_id = preset_id.to_string();
        b.sort_order = i as i64; // 화면에서 보낸 순서가 곧 실행 순서
        upsert_prompt_block(&mut *conn, preset_id, b).await?;
    }
    Ok(())
}
