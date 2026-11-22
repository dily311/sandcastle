//! 빠른 답장 세트 (5-E-20-5-4, ADR-025)
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub use crate::models::settings::QuickReply;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct QuickReplySet {
    pub id: String,
    pub name: String,
    /// global / character / preset
    pub scope: String,
    /// character_id / preset_id (전역이면 없음)
    pub scope_id: Option<String>,
    pub enabled: bool,
    /// 스크립트가 아닌 글: true면 바로 보냄(AI 답장 요금), false면 입력칸에 넣기만
    pub auto_send: bool,
    pub items: Vec<QuickReply>,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(FromRow)]
pub struct QuickReplySetRow {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub scope_id: Option<String>,
    pub enabled: bool,
    pub auto_send: bool,
    pub items: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<QuickReplySetRow> for QuickReplySet {
    fn from(r: QuickReplySetRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            scope: r.scope,
            scope_id: r.scope_id,
            enabled: r.enabled,
            auto_send: r.auto_send,
            items: serde_json::from_str(&r.items).unwrap_or_default(),
            sort_order: r.sort_order,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct QuickReplySetInput {
    /// 없으면 새로 만든다
    pub id: Option<String>,
    pub name: String,
    pub scope: String,
    pub scope_id: Option<String>,
    pub enabled: bool,
    pub auto_send: bool,
    pub items: Vec<QuickReply>,
    pub sort_order: i64,
}
