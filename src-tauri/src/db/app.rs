// use sqlx::{Error, Executor, Sqlite};

// pub async fn get_all_app_settings<'e, E>(executor: E) -> Result<Vec<FeatureSetting>, Error> where E: Executor<'e, Database = Sqlite> {
//     let rows = sqlx::query_as::<_, FeatureSettingRaw>(
//         r#"
//         SELECT key, value FROM app_settings
//     "#,
//     )
//     .fetch_all(executor)
//     .await?;
//     Ok(rows.into_iter().map(Into::into).collect())
// }

// pub async fn upsert_app_settings<'e, E>(executor: E, input: FeatureSetting) -> Result<(), Error> where E: Executor<'e, Database = Sqlite> {
//     let now = chrono::Utc::now().timestamp();

//     sqlx::query(
//         r#"
//         INSERT INTO app_settings (key, value, updated_at) 
//         VALUES (?, ?, ?)
//         ON CONFLICT(key) DO UPDATE SET
//             value = excluded.value,
//             updated_at = excluded.updated_at
//     "#,
//     )
//     .bind(&input.id)
//     .bind(&input.enabled.to_string())
//     .bind(now)
//     .execute(executor)
//     .await?;
//     Ok(())
// }


// pub async fn delete_app_settings<'e, E>(executor: E, key: &str) -> Result<bool, Error> where E: Executor<'e, Database = Sqlite> {
//     let result = sqlx::query("DELETE FROM app_settings WHERE key= ?")
//         .bind(key)
//         .execute(executor)
//         .await?;
//     Ok(result.rows_affected() != 0)
// }
