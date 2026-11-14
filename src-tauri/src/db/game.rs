use uuid::Uuid;
use sqlx::{Error, Executor, Sqlite, Transaction};
use crate::{models::{conversation::{WorldInfoEntry, WorldInfoEntryRow}, game::*}};


// ---------------------------------------------------------
// 게임 프로젝트
// ---------------------------------------------------------
/** 게임 프로젝트들 모두 불러오기 */
pub async fn get_all_game_projects<'e, E>(executor: E) -> Result<Vec<GameProject>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, GameProjectRow>(
        r#"
        SELECT 
            p.id, 
            p.title, 
            p.description, 
            p.template_id, 
            p.version, 
            p.api_version, 
            p.cover_image, 
            (
                SELECT json_group_array(book_id) 
                FROM game_worldinfo gw 
                WHERE gw.game_id = p.id
            ) AS world_info_ids,
            p.custom_css, 
            p.start_scene_key,
            p.start_map_id,
            p.start_location_id,
            p.settings_json, 
            p.created_at, 
            p.updated_at
        FROM game_projects p
        ORDER BY updated_at DESC
        "#,
    )
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}
/** 게임 프로젝트 불러오기 */
pub async fn get_game_project<'e, E>(executor: E, id: &str) -> Result<GameProject, Error> where E: Executor<'e, Database = Sqlite> {
    let row = sqlx::query_as::<_, GameProjectRow>(
        r#"
        SELECT
            p.id, 
            p.title, 
            p.description, 
            p.template_id, 
            p.version, 
            p.api_version, 
            p.cover_image, 
            (
                SELECT json_group_array(book_id) 
                FROM game_worldinfo gw 
                WHERE gw.game_id = p.id
            ) AS world_info_ids,
            p.custom_css, 
            p.start_scene_key,
            p.start_map_id,
            p.start_location_id,
            p.settings_json, 
            p.created_at, 
            p.updated_at
        FROM game_projects p
        WHERE p.id = ?
        "#,
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(row.into())
}

/** 게임 프로젝트 생성/업데이트 */
pub async fn upsert_game_project<'e, E>(executor: E, input: &GameProjectInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        r#"
        INSERT INTO game_projects ( id, title, description, template_id, version, api_version, cover_image, start_scene_key, start_map_id, start_location_id, custom_css, settings_json, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,?)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            description = excluded.description,
            template_id = excluded.template_id,
            version = excluded.version,
            api_version = excluded.api_version,
            cover_image = excluded.cover_image,
            start_scene_key = excluded.start_scene_key,
            start_map_id = excluded.start_map_id,
            start_location_id = excluded.start_location_id,
            custom_css = excluded.custom_css,
            settings_json = excluded.settings_json,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(id)
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.template_id)
    .bind(&input.version)
    .bind(&input.api_version)
    .bind(&input.cover_image)
    .bind(&input.start_scene_key)
    .bind(&input.start_map_id)
    .bind(&input.start_location_id)
    .bind(&input.custom_css)
    .bind(&input.settings_json)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}

/** 게임 프로젝트 삭제
 * return true: 삭제됨
 *        false: 해당 칼럼을 찾을 수 없음
*/
pub async fn delete_game_project<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_projects WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn get_game_worldinfo<'e, E>(executor: E, game_id: &str) -> Result<Vec<WorldInfoEntry>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, WorldInfoEntryRow>(
    r#"
        SELECT id, book_id, keys, secondary_keys, content, enabled, priority, constant
        FROM worldinfo_entries
        WHERE enabled = 1
        AND book_id IN (
            SELECT book_id FROM game_worldinfo WHERE game_id = ?
        )
        "#,
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}
// ---------------------------------------------------------
// 게임 변수
// ---------------------------------------------------------
/** 특정 게임 state 모두 가져오기 */
pub async fn get_all_game_state_defs<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameStateDef>, Error> where E: Executor<'e, Database = Sqlite> {
    let state_defs = sqlx::query_as::<_, GameStateDefRow>(
        r#"
        SELECT id, game_id, description, key, type, default_value, config_json, created_at, updated_at 
        FROM game_state_defs 
        WHERE game_id = ? 
        ORDER BY key
        "#,
    )
    .bind(&game_id)
    .fetch_all(executor)
    .await?;
    Ok(state_defs.into_iter().map(Into::into).collect())
}

/** 특정 게임 state 모두 업데이트 */
pub async fn upsert_game_state_defs<'e, E>(executor: E, game_id: &str, state_def: &GameStateDefInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = state_def.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        r#"
        INSERT INTO game_state_defs ( id, game_id, key, type, default_value, config_json, description, created_at, updated_at )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(game_id, key) DO UPDATE SET
            game_id = excluded.game_id,
            key = excluded.key,
            default_value = excluded.default_value,
            config_json=excluded.config_json,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(game_id)
    .bind(&state_def.key)
    .bind(&state_def.def_type)
    .bind(&state_def.default_value)
    .bind(&state_def.config_json)
    .bind(&state_def.description)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}

/** 특정 state 제거 
 * return true: 삭제됨
 *        false: 해당 칼럼을 찾을 수 없음
*/
pub async fn delete_game_state_def<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_state_defs WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

/** 한 게임에 관련된 state들을 모두 제거 
 * return true: 삭제됨
 *        false: 해당 칼럼을 찾을 수 없음
*/
pub async fn delete_game_all_state_defs<'e, E>(executor: E, game_id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_state_defs WHERE game_id = ?")
        .bind(&game_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// 게임 캐릭터
// ---------------------------------------------------------
pub async fn get_all_game_characters<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameCharacters>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameCharactersRow>(
    "SELECT 
        c.id, 
        c.game_id, 
        c.character_id, 
        c.name, 
        c.avatar, 
        c.description, 
        c.first_message, 
        c.example_dialogue, 
        c.system_prompt_override, 
        c.behavior_type, 
        (
            SELECT json_group_array(book_id) 
            FROM game_character_worldinfo cw 
            WHERE cw.game_character_id = c.id
        ) AS world_info_ids,
        c.tags, 
        c.settings_json, 
        c.created_at, 
        c.updated_at
    FROM game_characters c
    WHERE c.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_character<'e, E>(executor: E, id: &str) -> Result<GameCharacters, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameCharactersRow>(
    "SELECT 
        c.id, 
        c.game_id, 
        c.character_id, 
        c.name, 
        c.avatar, 
        c.description, 
        c.first_message, 
        c.example_dialogue, 
        c.system_prompt_override, 
        c.behavior_type, 
        (
            SELECT json_group_array(book_id) 
            FROM game_character_worldinfo cw 
            WHERE cw.game_character_id = c.id
        ) AS world_info_ids,
        c.tags, 
        c.settings_json, 
        c.created_at, 
        c.updated_at
    FROM game_characters c
    WHERE c.id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_character<'e, E>(executor: E, game_id: &str, input: GameCharactersInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();

    sqlx::query(
        r#"
        INSERT INTO game_characters ( id, game_id, character_id, name, avatar, description, first_message, example_dialogue, system_prompt_override, behavior_type, tags, settings_json, created_at, updated_at )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            game_id = excluded.game_id,
            character_id = excluded.character_id,
            name = excluded.name,
            avatar = excluded.avatar,
            description = excluded.description,
            first_message = excluded.first_message,
            example_dialogue = excluded.example_dialogue,
            system_prompt_override = excluded.system_prompt_override,
            behavior_type = excluded.behavior_type,
            tags = excluded.tags,
            settings_json = excluded.settings_json,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(game_id)
    .bind(&input.character_id)
    .bind(&input.name)
    .bind(&input.avatar)
    .bind(&input.description)
    .bind(&input.first_message)
    .bind(&input.example_dialogue)
    .bind(&input.system_prompt_override)
    .bind(&input.behavior_type)
    .bind(&input.tags)
    .bind(&input.settings_json)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_character<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_characters WHERE id = ?")
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn get_all_game_character_worldinfo<'e, E>(executor: E, game_id: &str) -> Result<Vec<WorldInfoEntry>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, WorldInfoEntryRow>(
    r#"
        SELECT we.id, we.book_id, we.keys, we.secondary_keys, we.content, we.enabled, we.priority, we.constant
        FROM worldinfo_entries we
        WHERE we.enabled = 1
        AND we.book_id IN (
            SELECT DISTINCT cw.book_id 
            FROM game_character_worldinfo cw
            INNER JOIN game_characters c 
            ON cw.game_character_id = c.id
            WHERE c.game_id = ?
        )
        "#,
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}
pub async fn get_game_character_worldinfo<'e, E>(executor: E, game_character_id: &str) -> Result<Vec<WorldInfoEntry>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, WorldInfoEntryRow>(
    r#"
        SELECT id, book_id, keys, secondary_keys, content, enabled, priority, constant
        FROM worldinfo_entries
        WHERE enabled = 1
        AND book_id IN (
            SELECT book_id FROM game_character_worldinfo WHERE game_character_id = ?
        )
        "#,
    )
    .bind(game_character_id)
    .fetch_all(executor)
    .await?;

    Ok(rows.into_iter().map(Into::into).collect())
}

// game_character_spawn_locations
pub async fn get_all_game_character_spawn_locations<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameCharacterSpawnLocations>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameCharacterSpawnLocationsRow>(
    "SELECT 
        cs.id,
        cs.game_character_id,
        cs.location_id,
        cs.spawn_weight,
        cs.condition_json,
        cs.created_at,
        cs.updated_at
    FROM game_character_spawn_locations cs
    INNER JOIN game_characters c ON c.id = cs.game_character_id 
    WHERE c.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_character_spawn_location<'e, E>(executor: E, id: &str) -> Result<GameCharacterSpawnLocations, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameCharacterSpawnLocationsRow>(
    "SELECT 
        cs.id,
        cs.game_character_id,
        cs.location_id,
        cs.spawn_weight,
        cs.condition_json,
        cs.created_at,
        cs.updated_at
    FROM game_character_spawn_locations cs
    WHERE cs.id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_character_spawn_location<'e, E>(executor: E, input: GameCharacterSpawnLocationsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    let condition_json = serde_json::to_string(&input.condition_json).unwrap_or_default();

    sqlx::query(
        r#"
        INSERT INTO game_character_spawn_locations (id, game_character_id, location_id, spawn_weight, condition_json, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            game_character_id=excluded.game_character_id,
            location_id=excluded.location_id,
            spawn_weight=excluded.spawn_weight,
            condition_json=excluded.condition_json,
            updated_at=excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(&input.game_character_id)
    .bind(&input.location_id)
    .bind(&input.spawn_weight)
    .bind(condition_json)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_character_spawn_location<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_character_spawn_locations WHERE id = ?")
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// 게임 에셋
// ---------------------------------------------------------
pub async fn get_all_game_assets<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameAssets>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameAssetsRow>(
            r#"
            SELECT id, game_id, key, name, category, kind, file_path, settings_json, created_at, updated_at
            FROM game_assets
            WHERE game_id = ?
            "#,
        )
        .bind(game_id)
        .fetch_all(executor)
        .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_asset<'e, E>(executor: E, id: &str) -> Result<GameAssets, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameAssetsRow>(
            r#"
            SELECT id, game_id, key, name, category, kind, file_path, settings_json, created_at, updated_at
            FROM game_assets
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_one(executor)
        .await?;
    Ok(result.into())
}
pub async fn upsert_game_asset<'e, E>(executor: E, input: GameAssetsInput, game_id: &str) -> Result<String, Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    let settings_json = serde_json::to_string(&input.settings_json).unwrap_or_default();

    sqlx::query(r#"
    INSERT INTO game_assets (id, game_id, key, name, category, kind, file_path, settings_json, created_at, updated_at)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    ON CONFLICT(id) DO UPDATE SET
        game_id=excluded.game_id, 
        key=excluded.key, 
        name=excluded.name, 
        category=excluded.category,
        kind=excluded.kind,
        file_path=excluded.file_path, 
        settings_json=excluded.settings_json, 
        updated_at=excluded.updated_at
    "#)
    .bind(&id)
    .bind(game_id)
    .bind(&input.key)
    .bind(&input.name)
    .bind(&input.category)
    .bind(&input.kind)
    .bind(&input.file_path)
    .bind(settings_json)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;

    Ok(id)
}

pub async fn delete_game_asset<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_assets WHERE id = ?")
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn get_all_audio_assets<'e, E>(executor: E, game_id: &str) -> Result<Vec<AudioAssets>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, AudioAssetsRow>(
            r#"
            SELECT ga.id, ga.game_id, ga.key, ga.name, ga.category, ga.kind, ga.file_path, ga.settings_json, ga.created_at, ga.updated_at, a.tag, a.game_character_id, a.transcript
            FROM audio_assets a
            INNER JOIN game_assets ga ON ga.id = a.assets_id
            WHERE ga.game_id = ?
            "#,
        )
        .bind(game_id)
        .fetch_all(executor)
        .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_audio_asset<'e, E>(executor: E, assets_id: &str) -> Result<AudioAssets, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, AudioAssetsRow>(
            r#"
            SELECT ga.id, ga.game_id, ga.key, ga.name, ga.category, ga.kind, ga.file_path, ga.settings_json, ga.created_at, ga.updated_at, a.tag, a.game_character_id, a.transcript
            FROM audio_assets a
            INNER JOIN game_assets ga ON ga.id = a.assets_id
            WHERE ga.id = ?
            "#,
        )
        .bind(assets_id)
        .fetch_one(executor)
        .await?;
    Ok(result.into())
}
pub async fn upsert_audio_asset<'e, E>(executor: E, input: AudioAssetsInput, assets_id: &str) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(r#"
    INSERT INTO audio_assets (assets_id, tag, game_character_id, transcript)
    VALUES (?, ?, ?, ?)
    ON CONFLICT(assets_id) DO UPDATE SET
        tag=excluded.tag, 
        game_character_id=excluded.game_character_id, 
        transcript=excluded.transcript
    "#)
    .bind(assets_id)
    .bind(&input.tag)
    .bind(&input.game_character_id)
    .bind(&input.transcript)
    .execute(executor)
    .await?;

    Ok(())
}
pub async fn get_all_character_assets<'e, E>(executor: E, game_id: &str) -> Result<Vec<CharacterAssets>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, CharacterAssetsRow>(
            r#"
            SELECT ga.id, ga.game_id, ga.key, ga.name, ga.category, ga.kind, ga.file_path, ga.settings_json, ga.created_at, ga.updated_at, a.game_character_id, a.usage, a.emotion
            FROM character_assets a
            INNER JOIN game_assets ga ON ga.id = a.assets_id
            WHERE ga.game_id = ?
            "#,
        )
        .bind(game_id)
        .fetch_all(executor)
        .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_character_asset<'e, E>(executor: E, assets_id: &str) -> Result<CharacterAssets, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, CharacterAssetsRow>(
            r#"
            SELECT ga.id, ga.game_id, ga.key, ga.name, ga.category, ga.kind, ga.file_path, ga.settings_json, ga.created_at, ga.updated_at, a.game_character_id, a.usage, a.emotion
            FROM character_assets a
            INNER JOIN game_assets ga ON ga.id = a.assets_id
            WHERE ga.id = ?
            "#,
        )
        .bind(assets_id)
        .fetch_one(executor)
        .await?;
    Ok(result.into())
}
pub async fn upsert_character_asset<'e, E>(executor: E, input: CharacterAssetsInput, assets_id: &str) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(r#"
    INSERT INTO character_assets (assets_id, game_character_id, usage, emotion)
    VALUES (?, ?, ?, ?)
    ON CONFLICT(assets_id) DO UPDATE SET
        game_character_id=excluded.game_character_id, 
        usage=excluded.usage, 
        emotion=excluded.emotion
    "#)
    .bind(assets_id)
    .bind(&input.game_character_id)
    .bind(&input.usage)
    .bind(&input.emotion)
    .execute(executor)
    .await?;

    Ok(())
}
pub async fn get_all_cg_assets<'e, E>(executor: E, game_id: &str) -> Result<Vec<CgAssets>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, CgAssetsRow>(
            r#"
            SELECT ga.id, ga.game_id, ga.key, ga.name, ga.category, ga.kind, ga.file_path, ga.settings_json, ga.created_at, ga.updated_at, a.game_character_id, a.description, a.prompt, a.negative_prompt, a.seed, a.generation_params, a.ai_usable
            FROM cg_assets a
            INNER JOIN game_assets ga ON ga.id = a.assets_id
            WHERE ga.game_id = ?
            "#,
        )
        .bind(game_id)
        .fetch_all(executor)
        .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_cg_asset<'e, E>(executor: E, assets_id: &str) -> Result<CgAssets, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, CgAssetsRow>(
            r#"
            SELECT ga.id, ga.game_id, ga.key, ga.name, ga.category, ga.kind, ga.file_path, ga.settings_json, ga.created_at, ga.updated_at, a.game_character_id, a.description, a.prompt, a.negative_prompt, a.seed, a.generation_params, a.ai_usable
            FROM cg_assets a
            INNER JOIN game_assets ga ON ga.id = a.assets_id
            WHERE ga.id = ?
            "#,
        )
        .bind(assets_id)
        .fetch_one(executor)
        .await?;
    Ok(result.into())
}
pub async fn upsert_cg_asset<'e, E>(executor: E, input: CgAssetsInput, assets_id: &str) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(r#"
    INSERT INTO cg_assets (assets_id, game_character_id, description, prompt, negative_prompt, seed, generation_params, ai_usable)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    ON CONFLICT(assets_id) DO UPDATE SET
        game_character_id=excluded.game_character_id, 
        description=excluded.description, 
        prompt=excluded.prompt, 
        negative_prompt=excluded.negative_prompt, 
        seed=excluded.seed, 
        generation_params=excluded.generation_params,
        ai_usable=excluded.ai_usable
    "#)
    .bind(assets_id)
    .bind(&input.game_character_id)
    .bind(&input.description)
    .bind(&input.prompt)
    .bind(&input.negative_prompt)
    .bind(&input.seed)
    .bind(&input.generation_params)
    .bind(&input.ai_usable)
    .execute(executor)
    .await?;

    Ok(())
}
pub async fn get_all_background_assets<'e, E>(executor: E, game_id: &str) -> Result<Vec<BackgroundAssets>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, BackgroundAssetsRow>(
            r#"
            SELECT ga.id, ga.game_id, ga.key, ga.name, ga.category, ga.kind, ga.file_path, ga.settings_json, ga.created_at, ga.updated_at, a.location_id, a.time_id, a.weather_id
            FROM background_assets a
            INNER JOIN game_assets ga ON ga.id = a.assets_id
            WHERE ga.game_id = ?
            "#,
        )
        .bind(game_id)
        .fetch_all(executor)
        .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_background_asset<'e, E>(executor: E, assets_id: &str) -> Result<BackgroundAssets, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, BackgroundAssetsRow>(
            r#"
            SELECT ga.id, ga.game_id, ga.key, ga.name, ga.category, ga.kind, ga.file_path, ga.settings_json, ga.created_at, ga.updated_at, a.location_id, a.time_id, a.weather_id
            FROM background_assets a
            INNER JOIN game_assets ga ON ga.id = a.assets_id
            WHERE ga.id = ?
            "#,
        )
        .bind(assets_id)
        .fetch_one(executor)
        .await?;
    Ok(result.into())
}
pub async fn upsert_background_assets<'e, E>(executor: E, input: BackgroundAssetsInput, assets_id: &str) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(r#"
    INSERT INTO background_assets (assets_id, location_id, time_id, weather_id)
    VALUES (?, ?, ?, ?)
    ON CONFLICT(assets_id) DO UPDATE SET
        location_id=excluded.location_id, 
        time_id=excluded.time_id, 
        weather_id=excluded.weather_id
    "#)
    .bind(assets_id)
    .bind(&input.location_id)
    .bind(&input.time_id)
    .bind(&input.weather_id)
    .execute(executor)
    .await?;

    Ok(())
}


// ---------------------------------------------------------
// 게임 맵
// ---------------------------------------------------------
pub async fn get_all_game_maps<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameMaps>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameMapsRow>(
    "SELECT id, game_id, name, danger_level, default_weather_pool_id, settings_json, created_at, updated_at 
    FROM game_maps WHERE game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_map<'e, E>(executor: E, id: &str) -> Result<GameMaps, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameMapsRow>(
    "SELECT id, game_id, name, danger_level, default_weather_pool_id, settings_json, created_at, updated_at 
    FROM game_maps WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_map<'e, E>(executor: E, game_id: &str, input: GameMapsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    let settings_json = serde_json::to_string(&input.settings_json).unwrap_or_default();
    sqlx::query(
        r#"
        INSERT INTO game_maps ( id, game_id, name, danger_level, default_weather_pool_id, settings_json, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            game_id=excluded.game_id, 
            name=excluded.name, 
            danger_level=excluded.danger_level, 
            default_weather_pool_id=excluded.default_weather_pool_id, 
            settings_json=excluded.settings_json, 
            updated_at=excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(game_id)
    .bind(&input.name)
    .bind(&input.danger_level)
    .bind(&input.default_weather_pool_id)
    .bind(settings_json)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_map<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_maps WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn get_all_game_map_zones<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameMapsZone>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameMapsZoneRow>(
    "SELECT z.id, z.map_id, z.name, z.tags, z.settings_json, z.created_at, z.updated_at
    FROM game_map_zones z
    INNER JOIN game_maps m ON z.map_id = m.id 
    WHERE m.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_zone<'e, E>(executor: E, id: &str) -> Result<GameMapsZone, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameMapsZoneRow>(
    "SELECT z.id, z.map_id, z.name, z.tags, z.settings_json, z.created_at, z.updated_at 
    FROM game_map_zones z
    WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_zone<'e, E>(executor: E, map_id: &str, input: GameMapsZoneInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    let tags = serde_json::to_string(&input.tags).unwrap_or_default();
    let settings_json = serde_json::to_string(&input.settings_json).unwrap_or_default();

    sqlx::query(
        r#"
        INSERT INTO game_map_zones ( id, map_id, name, tags, settings_json, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            map_id=excluded.map_id, 
            name=excluded.name, 
            tags=excluded.tags, 
            settings_json=excluded.settings_json, 
            updated_at=excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(map_id)
    .bind(&input.name)
    .bind(tags)
    .bind(settings_json)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_zone<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_map_zones WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn get_all_game_locations<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameLocations>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameLocationsRow>(
    "SELECT l.id, l.zone_id, l.name, l.description, l.settings_json
    FROM game_locations l
    INNER JOIN game_map_zones z ON l.zone_id = z.id 
    INNER JOIN game_maps m ON z.map_id = m.id 
    WHERE m.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_locations<'e, E>(executor: E, id: &str) -> Result<GameLocations, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameLocationsRow>(
    "SELECT l.id, l.zone_id, l.name, l.description, l.settings_json
    FROM game_locations l
    WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_locations<'e, E>(executor: E, zone_id: &str, input: GameLocationsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let settings_json = serde_json::to_string(&input.settings_json).unwrap_or_default();
    sqlx::query(
        r#"
        INSERT INTO game_locations ( id, zone_id, name, description, settings_json )
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            zone_id=excluded.zone_id, 
            name=excluded.name, 
            description=excluded.description, 
            settings_json=excluded.settings_json
        "#,
    )
    .bind(&id)
    .bind(zone_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(settings_json)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_locations<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_locations WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn get_all_game_location_connections<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameLocationConnections>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameLocationConnectionsRow>(
    "SELECT c.from_id, c.to_id, c.travel_seconds, c.condition, c.settings_json
    FROM game_location_connections c
    JOIN game_locations l ON c.from_id = l.id
    JOIN game_map_zones z ON l.zone_id = z.id
    JOIN game_maps m ON z.map_id = m.id
    WHERE m.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_location_connection<'e, E>(executor: E, from_id: &str, to_id: &str) -> Result<GameLocationConnections, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameLocationConnectionsRow>(
    "SELECT from_id, to_id, travel_seconds, condition, settings_json
    FROM game_location_connections
    WHERE from_id = ? AND to_id = ?"
    )
    .bind(from_id)
    .bind(to_id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}

pub async fn upsert_game_location_connections<'e, E>(executor: E, input:GameLocationConnectionsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let settings_json = serde_json::to_string(&input.settings_json).unwrap_or_default();
    sqlx::query(
        r#"
        INSERT INTO game_location_connections ( from_id, to_id, travel_seconds, condition, settings_json )
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(from_id, to_id) DO UPDATE SET
            travel_seconds=excluded.travel_seconds,
            condition=excluded.condition,
            settings_json=excluded.settings_json
        "#,
    )
    .bind(&input.from_id)
    .bind(&input.to_id)
    .bind(&input.travel_seconds)
    .bind(&input.condition)
    .bind(settings_json)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn delete_game_location_connection<'e, E>(executor: E, from_id: &str, to_id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_location_connections WHERE from_id = ? AND to_id = ?")
        .bind(from_id)
        .bind(to_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// 게임 시간
// ---------------------------------------------------------
pub async fn get_all_game_time_periods<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameTimePeriods>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameTimePeriods>(
    "SELECT id, game_id, name, sort_order, start_minute, end_minute FROM game_time_periods WHERE game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_time_period<'e, E>(executor: E, id: &str) -> Result<GameTimePeriods, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameTimePeriods>(
    "SELECT id, game_id, name, sort_order, start_minute, end_minute FROM game_time_periods WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}

pub async fn upsert_game_time_period<'e, E>(executor: E, game_id: &str, input: GameTimePeriodsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

    sqlx::query(
        r#"
        INSERT INTO game_time_periods ( id, game_id, name, sort_order, start_minute, end_minute)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            game_id=excluded.game_id, 
            name=excluded.name, 
            sort_order=excluded.sort_order,
            start_minute=excluded.start_minute,
            end_minute=excluded.end_minute
        "#,
    )
    .bind(&id)
    .bind(game_id)
    .bind(&input.name)
    .bind(&input.sort_order)
    .bind(&input.start_minute)
    .bind(&input.end_minute)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_time_period<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_time_periods WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// 게임 날씨
// ---------------------------------------------------------
pub async fn get_all_game_weather_types<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameWeatherTypes>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameWeatherTypesRow>(
    "SELECT id, game_id, name, encounter_rate_modifier, visibility_modifier, created_at, updated_at 
    FROM game_weather_types 
    WHERE game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_weather_type<'e, E>(executor: E, id: &str) -> Result<GameWeatherTypes, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameWeatherTypesRow>(
    "SELECT id, game_id, name, encounter_rate_modifier, visibility_modifier, created_at, updated_at 
    FROM game_weather_types 
    WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_weather_type<'e, E>(executor: E, game_id: &str, input: GameWeatherTypesInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();

    sqlx::query(
        r#"
        INSERT INTO game_weather_types ( id, game_id, name, encounter_rate_modifier, visibility_modifier, created_at, updated_at )
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(game_id, id) DO UPDATE SET
            game_id=excluded.game_id, 
            name=excluded.name, 
            encounter_rate_modifier=excluded.encounter_rate_modifier, 
            visibility_modifier=excluded.visibility_modifier, 
            updated_at=excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(game_id)
    .bind(&input.name)
    .bind(&input.encounter_rate_modifier)
    .bind(&input.visibility_modifier)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_weather_type<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_weather_types WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// --- GameWeatherPools ---
pub async fn get_all_game_weather_pools<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameWeatherPools>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameWeatherPoolsRow>(
    "SELECT id, game_id, name, created_at, updated_at
    FROM game_weather_pools 
    WHERE game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_weather_pool<'e, E>(executor: E, id: &str) -> Result<GameWeatherPools, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameWeatherPoolsRow>(
    "SELECT id, game_id, name, created_at, updated_at 
    FROM game_weather_pools 
    WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_weather_pool<'e, E>(executor: E, game_id: &str, input: GameWeatherPoolsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();

    sqlx::query(
        r#"
        INSERT INTO game_weather_pools ( id, game_id, name, created_at, updated_at )
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(game_id, id) DO UPDATE SET
            game_id=excluded.game_id, 
            name=excluded.name, 
            updated_at=excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(game_id)
    .bind(&input.name)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_weather_pool<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_weather_pools WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// --- GameWeatherPoolEntries ---
pub async fn get_all_game_weather_pool_entries<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameWeatherPoolEntries>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameWeatherPoolEntriesRow>(
    "SELECT e.pool_id, e.weather_id, e.weight
    FROM game_weather_pool_entries e 
    INNER JOIN game_weather_pools p ON e.pool_id = p.id 
    WHERE p.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn upsert_game_weather_pool_entry<'e, E>(executor: E, input: GameWeatherPoolEntriesInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(
        r#"
        INSERT INTO game_weather_pool_entries ( pool_id, weather_id, weight )
        VALUES (?, ?, ?)
        ON CONFLICT(pool_id, weather_id) DO UPDATE SET
            weight=excluded.weight
        "#,
    )
    .bind(&input.pool_id)
    .bind(&input.weather_id)
    .bind(&input.weight)
    .execute(executor)
    .await?;
    Ok(())
}

// --- GameWeatherTransitions ---
pub async fn get_all_game_weather_transitions<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameWeatherTransitions>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameWeatherTransitionsRow>(
    "SELECT t.from_weather_id, t.to_weather_id, t.map_id, t.probability
    FROM game_weather_transitions t
    INNER JOIN game_maps m ON t.map_id = m.id 
    WHERE m.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn upsert_game_weather_transition<'e, E>(executor: E, input: GameWeatherTransitionsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(
        r#"
        INSERT INTO game_weather_transitions ( from_weather_id, to_weather_id, map_id, probability )
        VALUES (?, ?, ?, ?)
        ON CONFLICT(from_weather_id, to_weather_id, map_id) DO UPDATE SET
            probability=excluded.probability
        "#,
    )
    .bind(&input.from_weather_id)
    .bind(&input.to_weather_id)
    .bind(&input.map_id)
    .bind(&input.probability)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_weather_transition<'e, E>(executor: E, from_weather_id: &str, to_weather_id: &str, map_id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_weather_transitions WHERE from_weather_id = ? AND to_weather_id = ? AND map_id = ?")
        .bind(from_weather_id)
        .bind(to_weather_id)
        .bind(map_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// 게임 이벤트
// ---------------------------------------------------------
pub async fn get_all_game_events<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameEvents>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEventsRow>(
    "SELECT id, game_id, event_type, trigger_type, repeat_policy, name, description, requires_npc, config_json, effect_json, cooldown_seconds, created_at, updated_at 
    FROM game_events 
    WHERE game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_event<'e, E>(executor: E, id: &str) -> Result<GameEvents, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEventsRow>(
    "SELECT id, game_id, event_type, trigger_type, repeat_policy, name, description, requires_npc, config_json, effect_json, cooldown_seconds, created_at, updated_at  FROM game_events WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_event<'e, E>(executor: E, game_id: &str, input: GameEventsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    let config_json = serde_json::to_string(&input.config_json).unwrap_or_default();
    let effect_json = serde_json::to_string(&input.effect_json).unwrap_or_default();

    sqlx::query(
        r#"
        INSERT INTO game_events ( id, game_id, event_type, trigger_type, repeat_policy, name, description, requires_npc, config_json, effect_json, cooldown_seconds, created_at, updated_at )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            game_id=excluded.game_id, 
            event_type=excluded.event_type, 
            trigger_type=excluded.trigger_type, 
            repeat_policy=excluded.repeat_policy, 
            name=excluded.name, 
            description=excluded.description, 
            requires_npc=excluded.requires_npc, 
            config_json=excluded.config_json, 
            effect_json=excluded.effect_json, 
            cooldown_seconds=excluded.cooldown_seconds, 
            updated_at=excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(game_id)
    .bind(&input.event_type)
    .bind(&input.trigger_type)
    .bind(&input.repeat_policy)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.requires_npc)
    .bind(&config_json)
    .bind(&effect_json)
    .bind(&input.cooldown_seconds)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_event<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_events WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// --- GameEventConditions ---
pub async fn get_all_event_conditions<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameEventConditions>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEventConditionsRow>(
    "SELECT c.id, c.event_id, c.condition_type, c.condition_value 
    FROM game_event_conditions c
    INNER JOIN game_events e ON c.event_id = e.id 
    WHERE e.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_event_condition<'e, E>(executor: E, id: &str) -> Result<GameEventConditions, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEventConditionsRow>(
    "SELECT id, event_id, condition_type, condition_value  FROM game_event_conditions WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_event_condition<'e, E>(executor: E, event_id: &str, input: GameEventConditionsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

    sqlx::query(
        r#"
        INSERT INTO game_event_conditions ( id, event_id, condition_type, condition_value ) 
        VALUES (?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            event_id=excluded.event_id, 
            condition_type=excluded.condition_type, 
            condition_value=excluded.condition_value
        "#,
    )
    .bind(&id)
    .bind(event_id)
    .bind(&input.condition_type)
    .bind(&input.condition_value)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_event_conditions<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_event_conditions WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// --- game_interactables ---
pub async fn get_all_game_interactables<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameInteractables>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameInteractablesRow>(
    "SELECT i.id, i.location_id, i.name, i.event_id 
    FROM game_interactables i
    INNER JOIN game_locations l ON i.location_id = l.id 
    INNER JOIN game_map_zones z ON l.zone_id = z.id 
    INNER JOIN game_maps m ON z.map_id = m.id 
    WHERE m.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_location_game_interactables<'e, E>(executor: E, location_id: &str) -> Result<Vec<GameInteractables>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameInteractablesRow>(
    "SELECT id, location_id, name, event_id FROM game_interactables WHERE location_id = ?"
    )
    .bind(location_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_interactables<'e, E>(executor: E, id: &str) -> Result<GameInteractables, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameInteractablesRow>(
    "SELECT id, location_id, name, event_id FROM game_interactables WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_interactables<'e, E>(executor: E, input: GameInteractablesInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

    sqlx::query(
        r#"
        INSERT INTO game_interactables ( id, location_id, name, event_id ) 
        VALUES (?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            location_id=excluded.location_id, 
            name=excluded.name, 
            event_id=excluded.event_id
        "#,
    )
    .bind(&id)
    .bind(&input.location_id)
    .bind(&input.name)
    .bind(&input.event_id)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_interactables<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_interactables WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// --- GameEventCharacterPool ---
pub async fn get_all_event_characters<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameEventCharacterPool>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEventCharacterPoolRow>(
    "SELECT c.event_id, c.game_character_id, c.spawn_weight 
    FROM game_event_character_pool c
    INNER JOIN game_events e ON c.event_id = e.id
    WHERE e.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn upsert_game_event_character<'e, E>(executor: E, event_id: &str, input: GameEventCharacterPoolInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(
        r#"
        INSERT INTO game_event_character_pool ( event_id, game_character_id, spawn_weight ) 
        VALUES (?, ?, ?)
        ON CONFLICT (event_id, game_character_id) DO UPDATE SET
            spawn_weight=excluded.spawn_weight
        "#,
    )
    .bind(event_id)
    .bind(&input.game_character_id)
    .bind(&input.spawn_weight)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn delete_game_event_character<'e, E>(executor: E, event_id: &str, game_character_id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("
        DELETE FROM game_event_character_pool WHERE event_id = ? AND game_character_id = ?
    ")
    .bind(event_id)
    .bind(game_character_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// 게임 인카운트 테이블
// ---------------------------------------------------------
pub async fn get_all_game_encounter_tables<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameEncounterTables>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEncounterTables>(
    "SELECT t.id, t.map_id, t.zone_id, location_id, t.base_trigger_chance 
    FROM game_encounter_tables t
    INNER JOIN game_maps m ON t.map_id = m.id
    WHERE m.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result)
}
pub async fn get_game_encounter_tables<'e, E>(executor: E, id: &str) -> Result<GameEncounterTables, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEncounterTables>(
    "SELECT id, map_id, zone_id, location_id, base_trigger_chance  FROM game_encounter_tables WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result)
}
pub async fn upsert_game_encounter_table<'e, E>(executor: E, input: GameEncounterTablesInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

    sqlx::query(
        r#"
        INSERT INTO game_encounter_tables ( id, map_id, zone_id, location_id, base_trigger_chance ) 
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            map_id=excluded.map_id, 
            zone_id=excluded.zone_id, 
            location_id=excluded.location_id, 
            base_trigger_chance=excluded.base_trigger_chance
        "#,
    )
    .bind(&id)
    .bind(&input.map_id)
    .bind(&input.zone_id)
    .bind(&input.location_id)
    .bind(&input.base_trigger_chance)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_encounter_table<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_encounter_tables WHERE id = ?")
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn get_all_game_encounter_table_entries<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameEncounterEntries>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEncounterEntriesRow>(
    "SELECT c.id, c.encounter_table_id, c.event_id, c.weight, c.time_period_id, c.weather_id  
    FROM game_encounter_entries c
    INNER JOIN game_events e ON c.event_id = e.id
    WHERE e.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_encounter_table_entry<'e, E>(executor: E, id: &str) -> Result<GameEncounterEntries, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEncounterEntriesRow>(
    "
    SELECT id, encounter_table_id, event_id, weight, time_period_id, weather_id  
    FROM game_encounter_entries 
    WHERE id = ?
    ")
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_encounter_table_entry<'e, E>(executor: E, input: GameEncounterEntriesInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    sqlx::query(
        r#"
        INSERT INTO game_encounter_entries ( id, encounter_table_id, event_id, weight, time_period_id, weather_id ) 
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            encounter_table_id=excluded.encounter_table_id, 
            event_id=excluded.event_id, 
            weight=excluded.weight,
            time_period_id=excluded.time_period_id,
            weather_id=excluded.weather_id
        "#,
    )
    .bind(&id)
    .bind(&input.encounter_table_id)
    .bind(&input.event_id)
    .bind(&input.weight)
    .bind(&input.time_period_id)
    .bind(&input.weather_id)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_encounter_table_entry<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_encounter_entries WHERE id = ?")
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// 게임 씬
// ---------------------------------------------------------
/** 특정 게임의 씬 모두 불러오기 */
pub async fn get_all_game_scenes<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameSceneRow>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, GameSceneRow>(
    "SELECT id, game_id, scene_name, description, scene_key, is_ending, ending_id, settings_json, script, sort_order, created_at, updated_at
    FROM game_scenes WHERE game_id = ? 
    ORDER BY sort_order, id
    ",
    )
        .bind(&game_id)
        .fetch_all(executor)
        .await?;
    Ok(rows)
}

/** 게임 씬 생성/업데이트 */
pub async fn upsert_game_scene<'e, E>(executor: E, game_id: &str,scene: &GameSceneInput, scene_index: i64) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        r#"
        INSERT INTO game_scenes ( id, game_id, scene_name, description, scene_key, is_ending, ending_id, settings_json, script, sort_order, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
                game_id = excluded.game_id,
                scene_name = excluded.scene_name,
                description = excluded.description,
                scene_key = excluded.scene_key,
                is_ending = excluded.is_ending,
                ending_id = excluded.ending_id,
                settings_json = excluded.settings_json,
                script = excluded.script,
                sort_order = excluded.sort_order,
                updated_at = excluded.updated_at
        "#,
    )
    .bind(&scene.id)
    .bind(game_id)
    .bind(&scene.scene_name)
    .bind(&scene.description)
    .bind(&scene.scene_key)
    .bind(scene.is_ending)
    .bind(&scene.ending_id)
    .bind(&scene.settings_json)
    .bind(&scene.script)
    .bind(scene_index)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}

/** 특정 게임의 씬 모두 삭제
 * return true: 삭제됨
 *        false: 해당 칼럼을 찾을 수 없음
*/
pub async fn delete_game_all_scene<'e, E>(executor: E, game_id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_scenes WHERE game_id = ?")
        .bind(game_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

/** 특정 씬 삭제 
 * return true: 삭제됨
 *        false: 해당 칼럼을 찾을 수 없음
*/
pub async fn delete_game_scene<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_scenes WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// Game choices
// ---------------------------------------------------------
/** 특정 씬의 선택지 모두 불러오기 */
pub async fn get_all_game_choices<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameChoiceRow>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, GameChoiceRow>("
        SELECT ch.id, ch.scene_id, ch.text, ch.next_scene_id, ch.condition, ch.effect, ch.sort_order
        FROM game_choices ch
            INNER JOIN game_scenes s ON s.id = ch.scene_id
            WHERE s.game_id = ?
    ")
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/** 게임 선택지 생성/업데이트 */
pub async fn upsert_game_choice<'e, E>(executor: E, input: &GameChoiceInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(
        r#"
        INSERT INTO game_choices ( id, scene_id, text, next_scene_id, condition, effect, sort_order )
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            scene_id = excluded.scene_id,
            text = excluded.text,
            next_scene_id = excluded.next_scene_id,
            condition = excluded.condition,
            effect = excluded.effect,
            sort_order = excluded.sort_order
        "#,
    )
    .bind(&input.id)
    .bind(&input.scene_id)
    .bind(&input.text)
    .bind(&input.next_scene_id)
    .bind(&input.condition)
    .bind(&input.effect)
    .bind(input.sort_order)
    .execute(executor)
    .await?;
    Ok(())
}

/** 게임 선택지 생성/업데이트 (다음 노드 제외) */
pub async fn upsert_game_choice_without_links<'e, E>(executor: E, input: &GameChoiceInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(
        r#"
        INSERT INTO game_choices ( id, scene_id, text, next_scene_id, condition, effect, sort_order )
        VALUES (?, ?, ?, NULL, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            scene_id=excluded.scene_id, 
            text=excluded.text, 
            condition=excluded.condition, 
            effect=excluded.effect, 
            sort_order=excluded.sort_order
        "#,
    )
    .bind(&input.id)
    .bind(&input.scene_id)
    .bind(&input.text)
    .bind(&input.condition)
    .bind(&input.effect)
    .bind(input.sort_order)
    .execute(executor)
    .await?;
    Ok(())
}

/** 게임 선택지에서 다음노드만 업데이트 */
pub async fn upsert_game_choice_next<'e, E>(executor: E, next_scene_id: &str, choice_id: &str) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(r#"UPDATE game_choices SET next_scene_id =? WHERE id = ?"#)
        .bind(next_scene_id)
        .bind(choice_id)
        .execute(executor)
        .await?;
    Ok(())
}

/** 특정 선택지 삭제 
 * return true: 삭제됨
 *        false: 해당 칼럼을 찾을 수 없음
*/
pub async fn delete_game_choice<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_choices WHERE id = ?")
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// Game branches
// ---------------------------------------------------------
/** 특정 씬의 브런치 모두 불러오기 */
pub async fn get_all_game_branches<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameBranchRow>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, GameBranchRow>("
        SELECT cb.id, cb.scene_id, cb.condition, cb.next_scene_id, cb.sort_order
        FROM game_branches cb
            INNER JOIN game_scenes s ON s.id = cb.scene_id
            WHERE s.game_id = ?
    ")
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/** 게임 브런치 생성/업데이트 */
pub async fn upsert_game_branches<'e, E>(executor: E, input: &GameBranchInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(
        r#"
        INSERT INTO game_branches (
            id,
            scene_id,
            condition,
            next_scene_id,
            sort_order
        )
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            scene_id=excluded.scene_id, 
            condition=excluded.condition, 
            next_scene_id=excluded.next_scene_id, 
            sort_order=excluded.sort_order
        "#,
    )
    .bind(&input.id)
    .bind(&input.scene_id)
    .bind(&input.condition)
    .bind(&input.next_scene_id)
    .bind(input.sort_order)
    .execute(executor)
    .await?;
    Ok(())
}
/** 게임 브런치 생성/업데이트 (다음 노드 제외) */
pub async fn upsert_game_branch_without_links<'e, E>(executor: E, input: &GameBranchInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(
        r#"
        INSERT INTO game_branches (
            id,
            scene_id,
            condition,
            next_scene_id,
            sort_order
        )
        VALUES (?, ?, ?, NULL, ?)
        "#,
    )
    .bind(&input.id)
    .bind(&input.scene_id)
    .bind(&input.condition)
    .bind(input.sort_order)
    .execute(executor)
    .await?;
    Ok(())
}

/** 게임 브런치에서 다음노드만 업데이트 */
pub async fn upsert_game_branch_next<'e, E>(executor: E, next_scene_id: &str, id: &str) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(r#"UPDATE game_branches SET next_scene_id =? WHERE id = ?"#)
        .bind(next_scene_id)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}

/** 특정 브런치 삭제 
 * return true: 삭제됨
 *        false: 해당 칼럼을 찾을 수 없음
*/
pub async fn delete_game_branches<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_branches WHERE id = ?")
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// 게임 엔딩
// ---------------------------------------------------------
pub async fn get_all_game_endings<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameEndings>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEndings>(
    "SELECT id, game_id, ending_key, title, description  FROM game_endings WHERE game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result)
}
pub async fn get_game_ending<'e, E>(executor: E, id: &str) -> Result<GameEndings, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEndings>(
    "SELECT id, game_id, ending_key, title, description  FROM game_endings WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result)
}
pub async fn upsert_game_ending<'e, E>(executor: E, input: GameEndingsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

    sqlx::query(
        r#"
        INSERT INTO game_endings ( id, game_id, ending_key, title, description ) 
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            game_id=excluded.game_id, 
            ending_key=excluded.ending_key, 
            title=excluded.title,
            description=excluded.description
        "#,
    )
    .bind(&id)
    .bind(&input.game_id)
    .bind(&input.ending_key)
    .bind(&input.title)
    .bind(&input.description)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_ending<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_endings WHERE id = ?")
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}
// ---------------------------------------------------------
// 아이템
// ---------------------------------------------------------
pub async fn get_all_game_items<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameItems>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameItemsRow>("
        SELECT 
            id, game_id, name, description, icon, kind, max_stack, is_unique, is_key, consumable, use_effect, use_condition, on_acquire_event_id, settings_json, created_at, updated_at
        FROM game_items
        WHERE game_id = ?
    ")
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn upsert_game_item<'e, E>(executor: E, input: GameItemsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    let _result = sqlx::query("
        INSERT INTO game_items (id, game_id, name, description, icon, kind, max_stack, is_unique, is_key, consumable, use_effect, use_condition, on_acquire_event_id, settings_json, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            game_id=excluded.game_id,
            name=excluded.name,
            description=excluded.description,
            icon=excluded.icon,
            kind=excluded.kind,
            max_stack=excluded.max_stack,
            is_unique=excluded.is_unique,
            is_key=excluded.is_key,
            consumable=excluded.consumable,
            use_effect=excluded.use_effect,
            use_condition=excluded.use_condition,
            on_acquire_event_id=excluded.on_acquire_event_id,
            settings_json=excluded.settings_json,
            updated_at=excluded.updated_at
    ")
    .bind(id)
    .bind(&input.game_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.icon)
    .bind(&input.kind)
    .bind(&input.max_stack)
    .bind(&input.is_unique)
    .bind(&input.is_key)
    .bind(&input.consumable)
    .bind(&input.use_effect)
    .bind(&input.use_condition)
    .bind(&input.on_acquire_event_id)
    .bind(&input.settings_json)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_item<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_items WHERE id = ?")
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}
// ---------------------------------------------------------
// 관계도
// ---------------------------------------------------------
pub async fn get_all_game_relationship_defs<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameRelationshipDefs>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameRelationshipDefsRow>("
        SELECT 
            id, game_id, from_id, to_id, values_json, tags
        FROM game_relationship_defs
        WHERE game_id = ?
    ")
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn upsert_game_relationship_def<'e, E>(executor: E, input: GameRelationshipDefsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let values_json = serde_json::to_string(&input.values_json).unwrap_or_default();
    let tags = serde_json::to_string(&input.tags).unwrap_or_default();

    let _result = sqlx::query("
        INSERT INTO game_relationship_defs (
            id, game_id, from_id, to_id, values_json, tags
        )
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            game_id=excluded.game_id,
            from_id=excluded.from_id,
            to_id=excluded.to_id,
            values_json=excluded.values_json,
            tags=excluded.tags
    ")
    .bind(id)
    .bind(&input.game_id)
    .bind(&input.from_id)
    .bind(&input.to_id)
    .bind(values_json)
    .bind(tags)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_relationship_def<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_relationship_defs WHERE id = ?")
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}
// =========================================================
// 게임 런타임
// =========================================================
// ---------------------------------------------------------
// 게임 플레이 세션
// ---------------------------------------------------------
pub async fn get_all_game_player_sessions<'e, E>(executor: E, game_id: &str) -> Result<Vec<GamePlayerSessions>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GamePlayerSessionsRow>(
    "SELECT 
        id, game_id, player_id, current_scene_id, current_map_id, current_zone_id, current_location_id, current_weather_id, game_time, flags_json, parent_session_id, pending_scenes_json, inventory_json, relationships_json, stage_json, forked_from_seq, head_seq, updated_at
    FROM game_player_sessions WHERE game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_player_sessions<'e, E>(executor: E, id: &str) -> Result<GamePlayerSessions, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GamePlayerSessionsRow>(
    "SELECT 
        id, game_id, player_id, current_scene_id, current_map_id, current_zone_id, current_location_id, current_weather_id, game_time, flags_json, parent_session_id, pending_scenes_json, inventory_json, relationships_json, stage_json, forked_from_seq, head_seq, updated_at
    FROM game_player_sessions 
    WHERE id = ?"
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn upsert_game_player_sessions<'e, E>(executor: E, input: GamePlayerSessionsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    let flags_json = serde_json::to_string(&input.flags_json).unwrap_or_default();
    let pending_scenes_json = serde_json::to_string(&input.pending_scenes_json).unwrap_or_default();
    let inventory_json = serde_json::to_string(&input.inventory_json).unwrap_or_default();
    let relationships_json = serde_json::to_string(&input.relationships_json).unwrap_or_default();
    let stage_json = serde_json::to_string(&input.stage_json).unwrap_or_default();
    sqlx::query(
        r#"
        INSERT INTO game_player_sessions ( id, game_id, player_id, current_scene_id, current_map_id, current_zone_id, current_location_id, current_weather_id, game_time, flags_json, parent_session_id,  pending_scenes_json, inventory_json, relationships_json, stage_json, forked_from_seq, head_seq, updated_at ) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            game_id=excluded.game_id, 
            player_id=excluded.player_id, 
            current_scene_id=excluded.current_scene_id, 
            current_map_id=excluded.current_map_id, 
            current_zone_id=excluded.current_zone_id, 
            current_location_id=excluded.current_location_id, 
            current_weather_id=excluded.current_weather_id, 
            game_time=excluded.game_time, 
            flags_json=excluded.flags_json,
            parent_session_id=excluded.parent_session_id,
            pending_scenes_json=excluded.pending_scenes_json,
            inventory_json=excluded.inventory_json,
            relationships_json=excluded.relationships_json,
            relationships_json=excluded.relationships_json,
            stage_json=excluded.stage_json,
            head_seq=excluded.head_seq,
            updated_at=excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(&input.game_id)
    .bind(&input.player_id)
    .bind(&input.current_scene_id)
    .bind(&input.current_map_id)
    .bind(&input.current_zone_id)
    .bind(&input.current_location_id)
    .bind(&input.current_weather_id)
    .bind(&input.game_time)
    .bind(flags_json)
    .bind(&input.parent_session_id)
    .bind(pending_scenes_json)
    .bind(inventory_json)
    .bind(relationships_json)
    .bind(stage_json)
    .bind(&input.forked_from_seq)
    .bind(&input.head_seq)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_player_sessions<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_player_sessions WHERE id = ?")
    .bind(&id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// 게임 세이브
// ---------------------------------------------------------
pub async fn get_all_game_save<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameSave>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows= sqlx::query_as::<_, GameSaveRow>(
        r#"
        SELECT
            s.name,
            s.seq, 
            s.session_id,
            s.slot_number,
            s.current_scene_id,
            s.current_map_id,
            s.current_zone_id,
            s.current_location_id,
            s.current_weather_id,
            s.game_time,
            s.state_snapshot,
            s.updated_at
        FROM game_saves s
        INNER JOIN game_player_sessions ps ON ps.id = s.session_id
        WHERE ps.game_id = ?
        "#,
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_session_game_saves<'e, E>(executor: E, session_id: &str) -> Result<Vec<GameSave>, Error> where E: Executor<'e, Database = Sqlite> {
    let row = sqlx::query_as::<_, GameSaveRow>(
        r#"
        SELECT 
            name, seq, session_id, slot_number, current_scene_id, current_map_id, current_zone_id, current_location_id, current_weather_id, game_time, state_snapshot, updated_at
        FROM game_saves
        WHERE session_id = ?
        "#,
    )
    .bind(session_id)
    .fetch_all(executor)
    .await?;
    Ok(row.into_iter().map(Into::into).collect())
}
pub async fn get_game_save<'e, E>(executor: E, session_id: &str, slot_number: i64) -> Result<GameSave, Error> where E: Executor<'e, Database = Sqlite> {
    let row = sqlx::query_as::<_, GameSaveRow>(
        r#"
        SELECT
            name,
            seq, 
            session_id,
            slot_number,
            current_scene_id,
            current_map_id,
            current_zone_id,
            current_location_id,
            current_weather_id,
            game_time,
            state_snapshot,
            updated_at
        FROM game_saves
        WHERE session_id = ?
            AND slot_number = ?
        "#,
    )
    .bind(session_id)
    .bind(slot_number)
    .fetch_optional(executor)
    .await?
    .ok_or_else(|| Error::RowNotFound)?;
    Ok(row.into())
}
pub async fn forked_game_save<'e, E>(executor: E, session_id: &str, new_session_id: &str, slot_number: i64) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let _result = sqlx::query(
        r#"
        INSERT INTO game_saves (
            name, seq,  session_id, slot_number, current_scene_id, current_map_id, current_zone_id, current_location_id, current_weather_id, game_time, state_snapshot, updated_at
        )
        SELECT 
            name, seq,  ?, slot_number, current_scene_id, current_map_id, current_zone_id, current_location_id, current_weather_id, game_time, state_snapshot, updated_at
        FROM game_saves
        WHERE session_id = ?
            AND slot_number = ?
        "#,
    )
    .bind(new_session_id)
    .bind(session_id)
    .bind(slot_number)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn upsert_game_save<'e, E>(executor: E, game_id: &str,input: &GameSaveInput, snapshot: &str) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        r#"
        INSERT INTO game_saves (
            name,
            seq, 
            session_id,
            slot_number,
            current_scene_id,
            current_map_id,
            current_zone_id,
            current_location_id,
            current_weather_id,
            game_time,
            state_snapshot,
            updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(session_id, slot_number) DO UPDATE SET
            state_snapshot = excluded.state_snapshot,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(game_id)
    .bind(input.slot_number)
    .bind(&input.current_scene_id)
    .bind(&input.current_map_id)
    .bind(&input.current_zone_id)
    .bind(&input.current_location_id)
    .bind(&input.game_time)
    .bind(&input.current_weather_id)
    .bind(snapshot)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}

/** 특정 세이브 삭제 
 * return true: 삭제됨
 *        false: 해당 칼럼을 찾을 수 없음
*/
pub async fn delete_game_save<'e, E>(executor: E, session_id: &str, slot_number: i64) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_saves WHERE session_id = ? AND slot_number = ?")
    .bind(session_id)
    .bind(slot_number)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}


// ---------------------------------------------------------
// 게임 세션 기록
// ---------------------------------------------------------
pub async fn get_all_game_journals<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameJournal>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, GameJournalRow>(
    "SELECT 
        j.session_id,
        j.seq,
        j.kind,
        j.payload_json,
        j.game_time,
        j.created_at
    FROM game_journal j
    INNER JOIN game_player_sessions s ON s.id = j.session_id
    WHERE ps.game_id = ?
    "
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}
pub async fn get_session_game_journals<'e, E>(executor: E, session_id: &str) -> Result<Vec<GameJournal>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, GameJournalRow>(
    "SELECT 
        session_id, seq, kind, payload_json, game_time, created_at
    FROM game_journal 
    WHERE session_id = ? 
    "
    )
    .bind(session_id)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}
pub async fn get_game_journal_lookup<'e, E>(executor: E, session_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<Vec<GameJournal>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, GameJournalRow>(
    "SELECT 
        session_id, seq, kind, payload_json, game_time, created_at
    FROM game_journal 
    WHERE session_id = ? 
        AND (seq BETWEEN ? and ?)
    "
    )
    .bind(session_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}
pub async fn get_game_journal<'e, E>(executor: E, session_id: &str) -> Result<Vec<GameJournal>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, GameJournalRow>(
    "SELECT 
        session_id, seq, kind, payload_json, game_time, created_at
    FROM game_journal 
    WHERE session_id = ?
    "
    )
    .bind(session_id)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}
pub async fn forked_game_journal<'e, E>(executor: E, session_id: &str, new_session_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let _result = sqlx::query(
    "
    INSERT INTO game_journal (
        session_id, seq, kind, payload_json, game_time, created_at
    )
    SELECT 
        ?, seq, kind, payload_json, game_time, created_at
    FROM game_journal 
    WHERE session_id = ? 
        AND (seq BETWEEN ? and ?)
    "
    )
    .bind(new_session_id)
    .bind(session_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn upsert_game_journal<'e, E>(executor: E, input: GameJournalInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let payload_json = serde_json::to_string(&input.payload_json).unwrap_or_default();
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        r#"
        INSERT INTO game_journal (session_id, seq, kind, payload_json, game_time, created_at) 
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(session_id, seq) DO UPDATE SET
            kind=excluded.kind, 
            payload_json=excluded.payload_json, 
            game_time=excluded.game_time
        "#,
    )
    .bind(&input.session_id)
    .bind(&input.seq)
    .bind(&input.kind)
    .bind(payload_json)
    .bind(&input.game_time)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
// ---------------------------------------------------------
// 게임 세션 내 스토리 계획
// ---------------------------------------------------------
pub async fn get_all_game_story_plans<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameStoryPlans>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameStoryPlansRow>(
    "SELECT 
        p.session_id, p.seq, p.plan_json, p.reason
    FROM game_story_plans p
        INNER JOIN game_player_sessions s ON p.session_id = s.id
        WHERE s.game_id = ?
    "
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_story_plans<'e, E>(executor: E, session_id: &str) -> Result<Vec<GameStoryPlans>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameStoryPlansRow>(
    "SELECT 
        session_id, seq, plan_json, reason
    FROM game_story_plans 
    WHERE session_id = ?
    "
    )
    .bind(session_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_story_plans_lookup<'e, E>(executor: E, session_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<Vec<GameStoryPlans>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameStoryPlansRow>(
    "SELECT 
        session_id, seq, plan_json, reason
    FROM game_story_plans 
    WHERE session_id = ?
        AND seq BETWEEN ? and ?"
    )
    .bind(session_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_story_plan<'e, E>(executor: E, session_id: &str, seq: u32) -> Result<GameStoryPlans, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameStoryPlansRow>(
    "SELECT 
        session_id, seq, plan_json, reason
    FROM game_story_plans 
    WHERE session_id = ? AND seq = ?"
    )
    .bind(session_id)
    .bind(seq)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn forked_game_story_plans<'e, E>(executor: E, session_id: &str, new_session_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let _result = sqlx::query(
    "
    INSERT INTO game_story_plans (
        session_id, seq, plan_json, reason
    )
    SELECT 
        ?, seq, plan_json, reason
    FROM game_story_plans 
    WHERE session_id = ?
        AND seq BETWEEN ? and ?"
    )
    .bind(new_session_id)
    .bind(session_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn upsert_game_story_plan<'e, E>(executor: E, input: GameStoryPlansInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let plan_json = serde_json::to_string(&input.plan_json).unwrap_or_default();
    let reason = serde_json::to_string(&input.reason).unwrap_or_default();
    sqlx::query(
        r#"
        INSERT INTO game_story_plans ( session_id, seq, plan_json, reason ) 
        VALUES (?, ?, ?, ?)
        ON CONFLICT(seq, session_id) DO UPDATE SET
            plan_json=excluded.plan_json, 
            reason=excluded.reason
        "#,
    )
    .bind(&input.session_id)
    .bind(&input.seq)
    .bind(&plan_json)
    .bind(&reason)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_story_plans<'e, E>(executor: E, session_id: &str, seq: u32) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_story_plans WHERE session_id = ? AND seq = ?")
    .bind(session_id)
    .bind(seq)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}
// ---------------------------------------------------------
// 게임 세션 내 캐릭터 관계도
// ---------------------------------------------------------
pub async fn get_all_game_relationship_snapshots<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameRelationshipSnapshots>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameRelationshipSnapshotsRow>(
    "SELECT rs.session_id, rs.seq, rs.relationships_json, rs.created_at
    FROM game_relationship_snapshots rs
        INNER JOIN game_player_sessions s ON rs.session_id = s.id
        WHERE s.game_id = ?
    "
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_session_game_relationship_snapshots<'e, E>(executor: E, session_id: &str) -> Result<Vec<GameRelationshipSnapshots>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameRelationshipSnapshotsRow>(
    "SELECT session_id, seq, relationships_json, created_at
    FROM game_relationship_snapshots 
    WHERE session_id = ?
    "
    )
    .bind(session_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_relationship_snapshots_lookup<'e, E>(executor: E, session_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<Vec<GameRelationshipSnapshots>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameRelationshipSnapshotsRow>(
    "SELECT session_id, seq, relationships_json, created_at
    FROM game_relationship_snapshots 
    WHERE session_id = ?
        AND seq BETWEEN ? and ?"
    )
    .bind(session_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_relationship_snapshot<'e, E>(executor: E, session_id: &str, seq: u32) -> Result<GameRelationshipSnapshots, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameRelationshipSnapshotsRow>(
    "SELECT session_id, seq, relationships_json, created_at
    FROM game_relationship_snapshots 
    WHERE session_id = ? AND seq = ?"
    )
    .bind(session_id)
    .bind(seq)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn forked_game_relationship_snapshots<'e, E>(executor: E, session_id: &str, new_session_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let _result = sqlx::query(
    "
    INSERT INTO game_relationship_snapshots (
        session_id, seq, relationships_json, created_at
    )
    SELECT 
        ?, seq, relationships_json, created_at
    FROM game_relationship_snapshots 
    WHERE session_id = ?
        AND seq BETWEEN ? and ?"
    )
    .bind(new_session_id)
    .bind(session_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn upsert_game_relationship_snapshot<'e, E>(executor: E, input: GameRelationshipSnapshots) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let now = chrono::Utc::now().timestamp_millis();
    let relationships_json = serde_json::to_string(&input.relationships_json).unwrap_or_default();
    sqlx::query(
        r#"
        INSERT INTO game_relationship_snapshots ( session_id, seq, relationships_json, created_at ) 
        VALUES (?, ?, ?, ?)
        ON CONFLICT(seq, session_id) DO UPDATE SET
            relationships_json=excluded.relationships_json
        "#,
    )
    .bind(&input.session_id)
    .bind(&input.seq)
    .bind(&relationships_json)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_relationship_snapshot<'e, E>(executor: E, session_id: &str, seq: u32) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_relationship_snapshots WHERE session_id = ? AND seq = ?")
    .bind(session_id)
    .bind(seq)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}

// ---------------------------------------------------------
// 게임 세션 내 캐릭터 상태
// ---------------------------------------------------------
pub async fn get_all_game_character_state_snapshots<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameCharacterStatesSnapshots>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameCharacterStatesSnapshotsRow>(
    "SELECT 
        cs.session_id, cs.seq, cs.game_character_id, cs.current_location_id, cs.status, cs.state_json, cs.updated_at
    FROM game_character_state_snapshots cs
        INNER JOIN game_player_sessions s ON cs.session_id = s.id
        WHERE s.game_id = ?
    "
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_session_game_character_state_snapshots<'e, E>(executor: E, session_id: &str) -> Result<Vec<GameCharacterStatesSnapshots>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameCharacterStatesSnapshotsRow>(
    "SELECT 
        session_id, seq, game_character_id, current_location_id, status, state_json, updated_at
    FROM game_character_state_snapshots 
    WHERE session_id = ?
    "
    )
    .bind(session_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_character_state_snapshots_lookup<'e, E>(executor: E, session_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<Vec<GameCharacterStatesSnapshots>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameCharacterStatesSnapshotsRow>(
    "SELECT 
        session_id, seq, game_character_id, current_location_id, status, state_json, updated_at
    FROM game_character_state_snapshots 
    WHERE session_id = ?
        AND seq BETWEEN ? and ?"
    )
    .bind(session_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_character_state<'e, E>(executor: E, session_id: &str, seq: u32, game_character_id: &str) -> Result<GameCharacterStatesSnapshots, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameCharacterStatesSnapshotsRow>(
    "SELECT 
        session_id, seq, game_character_id, current_location_id, status, state_json, updated_at
    FROM game_character_state_snapshots 
    WHERE session_id = ? AND seq = ? AND game_character_id=?"
    )
    .bind(session_id)
    .bind(seq)
    .bind(game_character_id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn forked_game_character_states<'e, E>(executor: E, session_id: &str, new_session_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let _result = sqlx::query(
    "
    INSERT INTO game_character_state_snapshots (
        session_id, seq, game_character_id, current_location_id, status, state_json, updated_at
    )
    SELECT 
        ?, seq, game_character_id, current_location_id, status, state_json, updated_at
    FROM game_character_state_snapshots 
    WHERE session_id = ?
        AND seq BETWEEN ? and ?"
    )
    .bind(new_session_id)
    .bind(session_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn upsert_game_character_state<'e, E>(executor: E, input: GameCharacterStatesSnapshotsInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let now = chrono::Utc::now().timestamp_millis();
    let state_json = serde_json::to_string(&input.state_json).unwrap_or_default();
    sqlx::query(
        r#"
        INSERT INTO game_character_state_snapshots ( session_id, seq, game_character_id, current_location_id, status, state_json, updated_at ) 
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(session_id, seq, game_character_id) DO UPDATE SET
            current_location_id=excluded.current_location_id, 
            status=excluded.status, 
            state_json=excluded.state_json, 
            updated_at=excluded.updated_at
        "#,
    )
    .bind(&input.session_id)
    .bind(&input.seq)
    .bind(&input.game_character_id)
    .bind(&input.current_location_id)
    .bind(&input.status)
    .bind(&state_json)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_character_state_snapshot<'e, E>(executor: E, session_id: &str, seq: u32) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_character_state_snapshots WHERE session_id = ? AND seq = ?")
    .bind(session_id)
    .bind(seq)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}


// ---------------------------------------------------------
// 게임 이벤트 로그
// ---------------------------------------------------------
pub async fn get_all_game_event_logs<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameEventLog>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEventLogRow>(
    "SELECT l.session_id, l.seq, l.event_id, l.weather_id, l.triggered_at, l.result_json  
    FROM game_event_log l
        INNER JOIN game_player_sessions s ON l.session_id = s.id
        WHERE s.game_id = ?
    "
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_session_game_event_logs<'e, E>(executor: E, session_id: &str) -> Result<Vec<GameEventLog>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEventLogRow>(
    "SELECT 
        session_id, seq, event_id, weather_id, triggered_at, result_json 
    FROM game_event_log 
    WHERE session_id = ?
    "
    )
    .bind(session_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_event_logs_lookup<'e, E>(executor: E, session_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<Vec<GameEventLog>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEventLogRow>(
    "SELECT 
        session_id, seq, event_id, weather_id, triggered_at, result_json  
    FROM game_event_log 
    WHERE session_id = ?
        AND seq BETWEEN ? and ?"
    )
    .bind(session_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_event_log<'e, E>(executor: E, session_id: &str, seq: u32) -> Result<GameEventLog, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEventLogRow>(
    "SELECT 
        session_id, seq, event_id, weather_id, triggered_at, result_json 
    FROM game_event_log 
    WHERE session_id = ? AND seq = ?"
    )
    .bind(session_id)
    .bind(seq)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn forked_game_event_logs<'e, E>(executor: E, session_id: &str, new_session_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let _result = sqlx::query(
    "
    INSERT INTO game_event_log (
        session_id, seq, event_id, weather_id, triggered_at, result_json
    )
    SELECT 
        ?, seq, event_id, weather_id, triggered_at, result_json  
    FROM game_event_log 
    WHERE session_id = ?
        AND seq BETWEEN ? and ?"
    )
    .bind(new_session_id)
    .bind(session_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn upsert_game_event_log<'e, E>(executor: E, input: GameEventLogInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let result_json = serde_json::to_string(&input.result_json).unwrap_or_default();
    sqlx::query(
        r#"
        INSERT INTO game_event_log ( session_id, seq, event_id, weather_id, triggered_at, result_json ) 
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(session_id, seq) DO UPDATE SET
            event_id=excluded.event_id, 
            weather_id=excluded.weather_id, 
            triggered_at=excluded.triggered_at, 
            result_json=excluded.result_json
        "#,
    )
    .bind(&input.session_id)
    .bind(&input.seq)
    .bind(&input.event_id)
    .bind(&input.weather_id)
    .bind(&input.triggered_at)
    .bind(&result_json)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn delete_game_event_log<'e, E>(executor: E, session_id: &str, seq: u32) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_event_log WHERE session_id = ? AND seq = ?")
    .bind(session_id)
    .bind(seq)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() != 0)
}

pub async fn get_all_game_event_log_characters<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameEventLogCharacters>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEventLogCharacters>(
    "SELECT c.session_id, c.seq, c.game_character_id, c.role, c.sort_order
    FROM game_event_log_characters c
        INNER JOIN game_player_sessions s ON c.session_id = s.id
        WHERE s.game_id = ?
    "
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result)
}
pub async fn get_game_event_log_characters<'e, E>(executor: E, session_id: &str, seq: u32) -> Result<Vec<GameEventLogCharacters>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameEventLogCharacters>(
    "SELECT session_id, seq, game_character_id, role, sort_order 
    FROM game_event_log_characters 
    WHERE session_id = ? AND seq = ?"
    )
    .bind(session_id)
    .bind(seq)
    .fetch_all(executor)
    .await?;
    Ok(result)
}
pub async fn upsert_game_event_log_character<'e, E>(executor: E, input: GameEventLogCharactersInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(
        r#"
        INSERT INTO game_event_log_characters ( session_id, seq, game_character_id, role, sort_order ) 
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(session_id, seq, game_character_id) DO UPDATE SET
            role=excluded.role, 
            sort_order=excluded.sort_order
        "#,
    )
    .bind(&input.session_id)
    .bind(&input.seq)
    .bind(&input.game_character_id)
    .bind(&input.role)
    .bind(&input.sort_order)
    .execute(executor)
    .await?;
    Ok(())
}

// ---------------------------------------------------------
// 게임 대화 세션
// ---------------------------------------------------------
pub async fn get_all_game_conversations<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameConversations>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, GameConversationsRow>(
        r#"
        SELECT
            id,
            game_id,
            session_id,
            branch_key,
            participants_key,
            title,
            created_at,
            updated_at
        FROM game_conversations
        WHERE game_id = ?
        "#,
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_game_conversation_conversation_id<'e, E>(executor: E, game_id: &str, session_id: &str, participants_key: &str, branch_key: &str) -> Result<Option<String>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT id FROM game_conversations WHERE game_id = ? AND session_id = ? AND participants_key = ? AND branch_key = ?
        "#,
    )
    .bind(game_id)
    .bind(session_id)
    .bind(participants_key)
    .bind(branch_key)
    .fetch_optional(executor)
    .await?;
    Ok(rows)
}
pub async fn forked_game_conversations(tx: &mut Transaction<'_, Sqlite>, session_id: &str, new_session_id: &str) -> Result<Vec<(String, String)>, Error> {
    // 기존 데이터의 id와 기타 필요한 정보 조회
    let old_records: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM game_conversations WHERE session_id = ?"
    )
    .bind(session_id)
    .fetch_all(&mut **tx)
    .await?;

    // 각 행마다 대응될 새로운 id(문자열)들을 미리 생성
    let mut paired_ids = Vec::with_capacity(old_records.len());
    let mut old_ids = Vec::new();
    let mut new_ids = Vec::new();

    for record in old_records {
        let old_id = record;
        let new_id = Uuid::new_v4().to_string(); // 새로운 문자열 ID 생성 (예: UUID)
        
        old_ids.push(old_id.clone());
        new_ids.push(new_id.clone());
        paired_ids.push((old_id, new_id));
    }

    // INSERT 쿼리 실행
    for (old_id, new_id) in paired_ids.clone() {
        let _result = sqlx::query(
            "
            INSERT INTO game_conversations (
                id, game_id, session_id, branch_key, participants_key, title, created_at, updated_at 
                FROM game_conversations 
            )
            SELECT 
                ?,game_id, ?, branch_key, participants_key, title, created_at, updated_at 
            FROM game_conversations 
            WHERE id = ?
            ")
            .bind(&new_id)
            .bind(new_session_id)
            .bind(&old_id)
            .execute(&mut **tx)
            .await?;
    }

    Ok(paired_ids)
}

pub async fn upsert_game_conversation<'e, E>(executor: E, input: GameConversationsInput) -> Result<String, Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        r#"
        INSERT INTO game_conversations (
            id,
            game_id,
            session_id,
            branch_key,
            participants_key,
            title,
            created_at,
            updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(&input.game_id)
    .bind(&input.session_id)
    .bind(&input.branch_key)
    .bind(&input.participants_key)
    .bind(&input.title)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(id)
}

// ------ messages ---------
pub async fn get_all_game_messages<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameMessages>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameMessagesRow>(
    "SELECT m.id, m.seq, m.conversation_id, m.role, m.name, m.content, m.character_id, m.meta, m.created_at, m.updated_at
    FROM game_messages m
    INNER JOIN game_conversations c ON m.conversation_id = c.id 
    WHERE c.game_id = ?
    ORDER BY created_at, rowid"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_messages_lookup<'e, E>(executor: E, conversation_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<Vec<GameMessages>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameMessagesRow>(
    "SELECT 
        id, seq, conversation_id, role, name, content, character_id, meta, created_at, updated_at  FROM game_messages 
    WHERE conversation_id = ? 
        AND seq BETWEEN ? and ?
    ORDER BY created_at, rowid"
    )
    .bind(conversation_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_messages<'e, E>(executor: E, conversation_id: &str) -> Result<Vec<GameMessages>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameMessagesRow>(
    "SELECT id, seq, conversation_id, role, name, content, character_id, meta, created_at, updated_at  FROM game_messages WHERE conversation_id = ? ORDER BY created_at, rowid"
    )
    .bind(conversation_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_message_meta<'e, E>(executor: E, id: &str) -> Result<Option<serde_json::Value>, Error> where E: Executor<'e, Database = Sqlite>,{
    let row = sqlx::query_scalar::<_, String>("SELECT meta FROM game_messages WHERE id = ?")
        .bind(id)
        .fetch_optional(executor)
        .await?;
    Ok(row.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
}
pub async fn forked_game_messages(tx: &mut Transaction<'_, Sqlite>, conversation_id: &str, new_conversation_id: &str, from_seq: Option<u32>, to_seq: u32) -> Result<Vec<(String, String)>, Error> {
    // 기존 데이터의 id와 기타 필요한 정보 조회
    let old_records: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM game_messages 
        WHERE conversation_id = ? 
            AND seq BETWEEN ? and ?
        ORDER BY created_at, rowid
        "
    )
    .bind(conversation_id)
    .bind(from_seq.unwrap_or(0))
    .bind(to_seq)
    .fetch_all(&mut **tx)
    .await?;

    // 각 행마다 대응될 새로운 id(문자열)들을 미리 생성
    let mut paired_ids = Vec::with_capacity(old_records.len());
    let mut old_ids = Vec::new();
    let mut new_ids = Vec::new();

    for record in old_records {
        let old_id = record;
        let new_id = Uuid::new_v4().to_string(); // 새로운 문자열 ID 생성 (예: UUID)
        
        old_ids.push(old_id.clone());
        new_ids.push(new_id.clone());
        paired_ids.push((old_id, new_id));
    }

    // INSERT 쿼리 실행
    for (old_id, new_id) in paired_ids.clone() {
        let _result = sqlx::query(
        "
        INSERT INTO game_messages (
            id, seq, conversation_id, role, name, content, character_id, meta, created_at, updated_at  FROM game_messages 
        )
        SELECT 
            ?, seq, ?, role, name, content, character_id, meta, created_at, updated_at  FROM game_messages 
        WHERE id=?
        "
        )
        .bind(&new_id)
        .bind(new_conversation_id)
        .bind(&old_id)
        .execute(&mut **tx)
        .await?;
    }

    Ok(paired_ids)
}
pub async fn upsert_game_message<'e, E>(executor: E, conversation_id: &str, input: GameMessagesInput) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let meta = serde_json::to_string(&input.meta).unwrap_or_default();
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        r#"
        INSERT INTO game_messages ( id, seq, conversation_id, role, name, content, character_id, meta, created_at, updated_at ) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            seq=excluded.seq, 
            conversation_id=excluded.conversation_id, 
            role=excluded.role, 
            name=excluded.name, 
            content=excluded.content, 
            character_id=excluded.character_id, 
            meta=excluded.meta, 
            updated_at=excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(&input.seq)
    .bind(conversation_id)
    .bind(&input.role)
    .bind(&input.name)
    .bind(&input.content)
    .bind(&input.character_id)
    .bind(meta)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn update_game_message_meta<'e, E>(executor: E, id: &str, meta: serde_json::Value) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite>,{
    let meta_string = serde_json::to_string(&meta).unwrap_or_default();
    let result = sqlx::query("UPDATE game_messages SET meta = ? WHERE id = ?")
        .bind(meta_string)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}
pub async fn delete_game_message<'e, E>(executor: E, id: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query("DELETE FROM game_messages WHERE id = ?")
        .bind(&id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}

// ---------참여자---------
pub async fn get_all_game_conversations_participants<'e, E>(executor: E, game_id: &str) -> Result<Vec<GameConversationParticipants>, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameConversationParticipants>(
    "SELECT p.conversation_id, p.game_character_id, p.sort_order
    FROM game_conversation_participants p
    INNER JOIN game_conversation c ON p.conversation_id = c.id 
    WHERE c.game_id = ?"
    )
    .bind(game_id)
    .fetch_all(executor)
    .await?;
    Ok(result.into_iter().map(Into::into).collect())
}
pub async fn get_game_conversations_participant<'e, E>(executor: E, conversation_id: &str) -> Result<GameConversationParticipants, Error> where E: Executor<'e, Database = Sqlite> {
    let result = sqlx::query_as::<_, GameConversationParticipants>(
    "SELECT conversation_id, game_character_id, sort_order FROM game_conversation_participants WHERE conversation_id = ?"
    )
    .bind(conversation_id)
    .fetch_one(executor)
    .await?;
    Ok(result.into())
}
pub async fn forked_game_conversations_participant<'e, E>(executor: E, conversation_id: &str, new_conversation_id: &str) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(
    "
    INSERT INTO game_conversation_participants (
        conversation_id, game_character_id, sort_order
    )
    SELECT 
        ?, game_character_id, sort_order 
    FROM game_conversation_participants 
    WHERE conversation_id = ?"
    )
    .bind(new_conversation_id)
    .bind(conversation_id)
    .execute(executor)
    .await?;
    Ok(())
}
pub async fn upsert_game_conversations_participant<'e, E>(executor: E, conversation_id: &str, game_character_id: &str, sort_order: i64) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
    sqlx::query(
        r#"
        INSERT INTO game_conversation_participants ( conversation_id, game_character_id, sort_order ) 
        VALUES (?, ?, ?)
        ON CONFLICT(conversation_id, game_character_id) DO UPDATE SET
            sort_order=excluded.sort_order 
        "#,
    )
    .bind(conversation_id)
    .bind(game_character_id)
    .bind(sort_order)
    .execute(executor)
    .await?;
    Ok(())
}