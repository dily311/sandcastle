//! 폰 화면: 주인(페르소나·캐릭터)별 연락처
use sqlx::{Error, SqlitePool};

use crate::models::conversation::PhoneContact;

/// 주인이나 연락처 하나 (종류 + id)
#[derive(Debug, Clone, Copy)]
pub struct Who<'a> {
    pub kind: &'a str,
    pub id: &'a str,
}

pub fn valid_kind(kind: &str) -> bool {
    matches!(kind, "persona" | "character")
}

pub async fn list_contacts(pool: &SqlitePool, owner: Who<'_>) -> Result<Vec<PhoneContact>, Error> {
    sqlx::query_as::<_, PhoneContact>("SELECT * FROM phone_links WHERE owner_kind = ? AND owner_id = ? ORDER BY sort_order, created_at")
        .bind(owner.kind)
        .bind(owner.id)
        .fetch_all(pool)
        .await
}

/// 이미 있으면 그대로 둔다
pub async fn add_contact(pool: &SqlitePool, owner: Who<'_>, contact: Who<'_>) -> Result<PhoneContact, Error> {
    let next: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM phone_links WHERE owner_kind = ? AND owner_id = ?")
        .bind(owner.kind)
        .bind(owner.id)
        .fetch_one(pool)
        .await?;
    sqlx::query(
        "INSERT OR IGNORE INTO phone_links (owner_kind, owner_id, contact_kind, contact_id, status, sort_order, created_at)
         VALUES (?, ?, ?, ?, '', ?, ?)",
    )
    .bind(owner.kind)
    .bind(owner.id)
    .bind(contact.kind)
    .bind(contact.id)
    .bind(next)
    .bind(chrono::Utc::now().timestamp_millis())
    .execute(pool)
    .await?;
    sqlx::query_as::<_, PhoneContact>("SELECT * FROM phone_links WHERE owner_kind = ? AND owner_id = ? AND contact_kind = ? AND contact_id = ?")
        .bind(owner.kind)
        .bind(owner.id)
        .bind(contact.kind)
        .bind(contact.id)
        .fetch_one(pool)
        .await
}

pub async fn remove_contact(pool: &SqlitePool, owner: Who<'_>, contact: Who<'_>) -> Result<bool, Error> {
    let r = sqlx::query("DELETE FROM phone_links WHERE owner_kind = ? AND owner_id = ? AND contact_kind = ? AND contact_id = ?")
        .bind(owner.kind)
        .bind(owner.id)
        .bind(contact.kind)
        .bind(contact.id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn set_status(pool: &SqlitePool, owner: Who<'_>, contact: Who<'_>, status: &str) -> Result<bool, Error> {
    let r = sqlx::query("UPDATE phone_links SET status = ? WHERE owner_kind = ? AND owner_id = ? AND contact_kind = ? AND contact_id = ?")
        .bind(status)
        .bind(owner.kind)
        .bind(owner.id)
        .bind(contact.kind)
        .bind(contact.id)
        .execute(pool)
        .await?;
    Ok(r.rows_affected() > 0)
}
