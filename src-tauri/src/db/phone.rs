//! 폰 화면: 페르소나별 연락처
use sqlx::{Error, SqlitePool};

use crate::models::conversation::PhoneContact;

pub async fn list_contacts(pool: &SqlitePool, persona_id: &str) -> Result<Vec<PhoneContact>, Error> {
    sqlx::query_as::<_, PhoneContact>("SELECT * FROM phone_contacts WHERE persona_id = ? ORDER BY sort_order, created_at")
        .bind(persona_id)
        .fetch_all(pool)
        .await
}

/// 이미 있으면 그대로 둔다
pub async fn add_contact(pool: &SqlitePool, persona_id: &str, character_id: &str) -> Result<PhoneContact, Error> {
    let next: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM phone_contacts WHERE persona_id = ?")
        .bind(persona_id)
        .fetch_one(pool)
        .await?;
    sqlx::query(
        "INSERT OR IGNORE INTO phone_contacts (persona_id, character_id, status, sort_order, created_at) VALUES (?, ?, '', ?, ?)",
    )
    .bind(persona_id)
    .bind(character_id)
    .bind(next)
    .bind(chrono::Utc::now().timestamp_millis())
    .execute(pool)
    .await?;
    sqlx::query_as::<_, PhoneContact>("SELECT * FROM phone_contacts WHERE persona_id = ? AND character_id = ?")
        .bind(persona_id)
        .bind(character_id)
        .fetch_one(pool)
        .await
}

pub async fn remove_contact(pool: &SqlitePool, persona_id: &str, character_id: &str) -> Result<bool, Error> {
    let r = sqlx::query("DELETE FROM phone_contacts WHERE persona_id = ? AND character_id = ?")
        .bind(persona_id)
        .bind(character_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn set_status(pool: &SqlitePool, persona_id: &str, character_id: &str, status: &str) -> Result<bool, Error> {
    let r = sqlx::query("UPDATE phone_contacts SET status = ? WHERE persona_id = ? AND character_id = ?")
        .bind(status)
        .bind(persona_id)
        .bind(character_id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}
