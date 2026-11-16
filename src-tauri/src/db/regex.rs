use sqlx::{Error, Executor, Sqlite};
use uuid::Uuid;

use crate::models::regex::*;

const COLUMNS: &str = r#"
    id, name, find_regex, replace_string, trim_strings, placement, target, scope, scope_id,
    substitute_macros, run_on_edit, min_depth, max_depth, enabled, sort_order, source_format,
    extra_json, created_at, updated_at
"#;

pub async fn get_all_regex_scripts<'e, E>(executor: E) -> Result<Vec<RegexScript>, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let sql = format!("SELECT {COLUMNS} FROM regex_scripts ORDER BY scope, sort_order, created_at");
    let rows = sqlx::query_as::<_, RegexScriptRow>(sqlx::AssertSqlSafe(sql)).fetch_all(executor).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// 이 문맥에서 적용될 수 있는 스크립트 (전역 + 해당 캐릭터 + 해당 프리셋). 꺼진 것 포함.
pub async fn get_regex_scripts_for<'e, E>(executor: E, character_id: Option<&str>, preset_id: Option<&str>) -> Result<Vec<RegexScript>, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let sql = format!(
        r#"
        SELECT {COLUMNS} FROM regex_scripts
        WHERE scope = 'global'
           OR (scope = 'character' AND scope_id = ?)
           OR (scope = 'preset' AND scope_id = ?)
        ORDER BY sort_order, created_at
        "#
    );
    let rows = sqlx::query_as::<_, RegexScriptRow>(sqlx::AssertSqlSafe(sql))
        .bind(character_id.unwrap_or_default())
        .bind(preset_id.unwrap_or_default())
        .fetch_all(executor)
        .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_regex_script<'e, E>(executor: E, id: &str) -> Result<RegexScript, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let sql = format!("SELECT {COLUMNS} FROM regex_scripts WHERE id = ?");
    let row = sqlx::query_as::<_, RegexScriptRow>(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_one(executor)
        .await?;
    Ok(row.into())
}

pub async fn upsert_regex_script<'e, E>(executor: E, input: &RegexScriptInput) -> Result<String, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    let placement = serde_json::to_string(&input.placement).unwrap_or_else(|_| "[]".into());
    let trim = serde_json::to_string(&input.trim_strings).unwrap_or_else(|_| "[]".into());
    let extra = input.extra_json.as_ref().map(|v| v.to_string());
    sqlx::query(
        r#"
        INSERT INTO regex_scripts (
            id, name, find_regex, replace_string, trim_strings, placement, target, scope, scope_id,
            substitute_macros, run_on_edit, min_depth, max_depth, enabled, sort_order, source_format,
            extra_json, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            find_regex = excluded.find_regex,
            replace_string = excluded.replace_string,
            trim_strings = excluded.trim_strings,
            placement = excluded.placement,
            target = excluded.target,
            scope = excluded.scope,
            scope_id = excluded.scope_id,
            substitute_macros = excluded.substitute_macros,
            run_on_edit = excluded.run_on_edit,
            min_depth = excluded.min_depth,
            max_depth = excluded.max_depth,
            enabled = excluded.enabled,
            sort_order = excluded.sort_order,
            source_format = excluded.source_format,
            extra_json = excluded.extra_json,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.find_regex)
    .bind(&input.replace_string)
    .bind(trim)
    .bind(placement)
    .bind(input.target.to_string())
    .bind(input.scope.to_string())
    .bind(&input.scope_id)
    .bind(input.substitute_macros.to_string())
    .bind(input.run_on_edit)
    .bind(input.min_depth)
    .bind(input.max_depth)
    .bind(input.enabled)
    .bind(input.sort_order)
    .bind(input.source_format.clone().unwrap_or_else(|| "native".into()))
    .bind(extra)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(id)
}

pub async fn delete_regex_script<'e, E>(executor: E, id: &str) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let res = sqlx::query("DELETE FROM regex_scripts WHERE id = ?").bind(id).execute(executor).await?;
    Ok(res.rows_affected() != 0)
}
