use sqlx::{Error, Executor, Sqlite};

use crate::models::task::*;

pub async fn get_all_generation_tasks<'e, E>(executor: E) -> Result<Vec<GenerationTask>, Error> where E: Executor<'e, Database = Sqlite> {
    let rows = sqlx::query_as::<_, GenerationTaskRow>(
        r#"
        SELECT 
            id,
            task_type,
            status,
            payload_json,
            error_message,
            created_at,
            started_at,
            updated_at,
            finished_at
        FROM generation_tasks
        ORDER BY updated_at DESC
        "#,
    )
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

