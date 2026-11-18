//! 메모리 임베딩 검색 (CLAUDE.md 3-C, ADR-015).
//!
//! - `chunker`: 메시지 → chunk 계획 (순수)
//! - `store`: memory.db (벡터 캐시 + chunk 표, app.db와 분리)
//! - `index`: 미리보기와 색인 실행 (임베딩은 `AiService::embed`)
//! - `search`: 질의 임베딩 + 쪽 단위 내적 검색
//!
//! 유료 호출(색인·질의 임베딩)은 부르는 커맨드가 TaskManager 작업(`TaskType::Embedding`, feature `memory`) 안에서 한다.
pub mod chunker;
pub mod index;
pub mod search;
pub mod store;
#[cfg(test)]
mod tests;

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::{
    error::{AppError, Result},
    models::{ai_service::ApiProfiles, settings::MemorySetting},
};
use chunker::MemMsg;

pub const FEATURE: &str = "memory";

/// 색인·검색 범위. 대화 id가 없으면 전체
#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MemoryScope {
    pub conversation_id: Option<String>,
}

fn short_hash(s: &str) -> String {
    Sha256::digest(s.as_bytes()).iter().take(6).map(|b| format!("{b:02x}")).collect()
}

/// 벡터를 구분하는 키. 프로필 id가 아니라 (공급자 종류 + 주소 + 모델 + 차원 + 접두어) — 프로필을 다시 만들어도 같은 모델이면 재사용하고,
/// 호환 서버가 모델 이름을 겹쳐 써도 섞이지 않는다
pub fn model_key(profile: &ApiProfiles, cfg: &MemorySetting) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        profile.format_type,
        short_hash(profile.base_url.trim_end_matches('/')),
        cfg.model.trim(),
        cfg.dimensions.unwrap_or(0),
        short_hash(&format!("{}\u{0}{}", cfg.doc_prefix, cfg.query_prefix))
    )
}

/// 설정이 색인·검색에 쓸 수 있게 채워져 있는지
pub fn check_config(cfg: &MemorySetting) -> Result<String> {
    let id = cfg.api_profile_id.clone().filter(|s| !s.is_empty());
    match (id, cfg.model.trim().is_empty()) {
        (Some(id), false) => Ok(id),
        _ => Err(AppError::validation("임베딩 API와 모델을 먼저 고르세요 (설정 > 메모리)", None)),
    }
}

/// 색인할 수 있는 대화: 메모리에서 뺀(`meta.memory = "off"`) 대화와 캐릭터 폰 엿보기(`meta.phone`)는 제외
pub async fn eligible_conversations(db: &SqlitePool, scope: &MemoryScope) -> Result<Vec<(String, String)>> {
    let rows: Vec<(String, Option<String>, Option<String>)> = match &scope.conversation_id {
        Some(id) => {
            sqlx::query_as("SELECT id, title, meta FROM conversations WHERE id = ?")
                .bind(id)
                .fetch_all(db)
                .await?
        }
        None => {
            sqlx::query_as("SELECT id, title, meta FROM conversations ORDER BY updated_at DESC")
                .fetch_all(db)
                .await?
        }
    };
    let out: Vec<(String, String)> = rows
        .into_iter()
        .filter(|(_, _, meta)| {
            let m: Value = meta.as_deref().and_then(|r| serde_json::from_str(r).ok()).unwrap_or(Value::Null);
            m.get("memory").and_then(Value::as_str) != Some("off") && m.get("phone").is_none()
        })
        .map(|(id, title, _)| (id, title.unwrap_or_default()))
        .collect();
    if out.is_empty() && scope.conversation_id.is_some() {
        return Err(AppError::validation("이 대화는 메모리에서 제외되어 있거나 없음", None));
    }
    Ok(out)
}

type MessageRow = (String, String, Option<String>, String, Option<String>);

pub async fn load_messages(db: &SqlitePool, conversation_id: &str) -> Result<Vec<MemMsg>> {
    let rows: Vec<MessageRow> = sqlx::query_as("SELECT id, role, name, content, meta FROM messages WHERE conversation_id = ? ORDER BY seq")
        .bind(conversation_id)
        .fetch_all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(id, role, name, content, meta)| {
            let hidden = meta
                .as_deref()
                .and_then(|r| serde_json::from_str::<Value>(r).ok())
                .and_then(|m| m.get("hidden").and_then(Value::as_bool))
                .unwrap_or(false);
            MemMsg {
                id,
                role,
                name,
                content,
                hidden,
            }
        })
        .collect())
}

/// 이 모델로 이미 벡터가 있는 글 해시들
pub async fn cached_hashes(pool: Option<&SqlitePool>, model_key: &str, hashes: &[String]) -> Result<HashSet<String>> {
    let mut out = HashSet::new();
    let Some(pool) = pool else { return Ok(out) };
    for h in hashes {
        let hit: Option<i64> = sqlx::query_scalar("SELECT 1 FROM vectors WHERE model_key = ? AND text_hash = ?")
            .bind(model_key)
            .bind(h)
            .fetch_optional(pool)
            .await?;
        if hit.is_some() {
            out.insert(h.clone());
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStatus {
    /// 지금 설정(모델)으로 색인된 chunk 수
    pub chunks_current: u64,
    /// 다른 모델·차원·접두어로 색인된 chunk 수 (다시 색인하거나 지워야 검색됨)
    pub chunks_other: u64,
    pub vectors: u64,
    pub file_bytes: u64,
}

pub async fn status(store: &store::MemoryStore, key: Option<&str>) -> Result<MemoryStatus> {
    let Some(pool) = store.pool_if_exists().await? else {
        return Ok(MemoryStatus {
            chunks_current: 0,
            chunks_other: 0,
            vectors: 0,
            file_bytes: 0,
        });
    };
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chunks").fetch_one(&pool).await?;
    let current: i64 = match key {
        Some(k) => {
            sqlx::query_scalar("SELECT COUNT(*) FROM chunks WHERE model_key = ?")
                .bind(k)
                .fetch_one(&pool)
                .await?
        }
        None => 0,
    };
    let vectors: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vectors").fetch_one(&pool).await?;
    Ok(MemoryStatus {
        chunks_current: current as u64,
        chunks_other: (total - current) as u64,
        vectors: vectors as u64,
        file_bytes: store.file_bytes(),
    })
}

/// 정리: 다른 모델의 chunk·벡터 / 없어진 대화의 chunk와 어디서도 가리키지 않는 벡터. 지운 chunk 수를 돌려준다
pub async fn purge(store: &store::MemoryStore, db: &SqlitePool, keep_key: Option<&str>, other_models: bool) -> Result<u64> {
    let Some(pool) = store.pool_if_exists().await? else { return Ok(0) };
    let mut removed = 0u64;
    if other_models {
        let r = match keep_key {
            Some(k) => sqlx::query("DELETE FROM chunks WHERE model_key <> ?").bind(k).execute(&pool).await?,
            None => sqlx::query("DELETE FROM chunks").execute(&pool).await?,
        };
        removed += r.rows_affected();
    }
    // 없어진 대화
    let alive: HashSet<String> = sqlx::query_scalar::<_, String>("SELECT id FROM conversations")
        .fetch_all(db)
        .await?
        .into_iter()
        .collect();
    let convs: Vec<String> = sqlx::query_scalar("SELECT DISTINCT conversation_id FROM chunks").fetch_all(&pool).await?;
    for c in convs.iter().filter(|c| !alive.contains(*c)) {
        removed += sqlx::query("DELETE FROM chunks WHERE conversation_id = ?")
            .bind(c)
            .execute(&pool)
            .await?
            .rows_affected();
    }
    sqlx::query("DELETE FROM vectors WHERE NOT EXISTS (SELECT 1 FROM chunks c WHERE c.model_key = vectors.model_key AND c.text_hash = vectors.text_hash)")
        .execute(&pool)
        .await?;
    Ok(removed)
}
