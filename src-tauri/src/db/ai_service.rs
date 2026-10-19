use crate::models::ai_service::{ApiProfile, ApiProfileInput};
use sqlx::{Error, Executor, Sqlite};
use uuid::Uuid;

// =======================================
// API Profile
// =======================================
pub async fn get_all_api_profile<'e, E>(executor: E) -> Result<Vec<ApiProfile>, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let rows = sqlx::query_as::<_, ApiProfile>(
        r#"
        SELECT * FROM api_profile
    "#,
    )
    .fetch_all(executor)
    .await?;
    Ok(rows)
}
pub async fn get_api_profile<'e, E>(executor: E, id: &str) -> Result<ApiProfile, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let row = sqlx::query_as::<_, ApiProfile>(
        r#"
        SELECT * FROM api_profile WHERE id = ?
    "#,
    )
    .bind(id)
    .fetch_one(executor)
    .await?;
    Ok(row)
}

pub async fn create_api_profile<'e, E>(
    executor: E,
    input: ApiProfileInput,
) -> Result<ApiProfile, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();

    sqlx::query(r#"
        INSERT INTO api_profile (id, name, provider,format_type, base_url, model, selected_preset_id, created_at, updated_at) 
        VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?)
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

    Ok(ApiProfile {
        id,
        name: input.name,
        provider: input.provider,
        format_type: input.format_type,
        base_url: input.base_url,
        model: input.model,
        selected_preset_id: None,
        created_at: now,
        updated_at: now,
    })
}

pub async fn update_api_profile<'e, E>(
    executor: E,
    id: &str,
    input: ApiProfileInput,
) -> Result<(), Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let now = chrono::Utc::now().timestamp();
    sqlx::query(r#"
        UPDATE api_profile SET (name, provider,format_type, base_url, model, selected_preset_id, updated_at) 
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        WHERE id=?

    "#)
    .bind(&input.name)
    .bind(&input.provider)
    .bind(&input.format_type)
    .bind(&input.base_url)
    .bind(&input.model)
    .bind(&input.selected_preset_id)
    .bind(now)
    .bind(id)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn delete_api_profile<'e, E>(executor: E, id: &str) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let result = sqlx::query("DELETE FROM api_profile WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() != 0)
}
