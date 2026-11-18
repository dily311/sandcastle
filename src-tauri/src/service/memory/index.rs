//! 색인 미리보기와 실행 (CLAUDE.md 3-C-3, ADR-015).
//!
//! - 미리보기는 요금이 없다(공급자를 부르지 않음, 파일이 없으면 만들지도 않음).
//! - 실행은 부르는 커맨드가 TaskManager 작업 안에서 한다. 64개씩 배치, 배치마다 취소 확인·자기 트랜잭션으로 저장 →
//!   중간에 멈추거나 실패해도 쓴 배치는 남고 다음 실행은 캐시를 건너뛴다. 실패해도 자동으로 다시 시도하지 않는다.
use std::collections::HashSet;

use serde::Serialize;
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use super::{
    chunker::{self, Existing, Planned},
    store::{encode_vector, normalize, MemoryStore},
    *,
};
use crate::service::{
    ai::{AiService, EmbedInput},
    prompt::token::count_tokens,
};
use crate::{adapter::EmbedTask, error::ErrorCode};

/// 임베딩 한 번에 보내는 chunk 수
pub const BATCH: usize = 64;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IndexPreview {
    pub conversations: u32,
    /// 계획한 chunk 수
    pub chunks: u32,
    /// 이미 벡터가 있어 건너뛸 chunk 수
    pub cached: u32,
    /// 새로 임베딩할 글 수 (같은 글은 한 번)
    pub to_embed: u32,
    /// 새로 임베딩할 입력 토큰 (앱이 센 추정 — 채팅 모델의 토크나이저 기준)
    pub est_tokens: u64,
    pub token_cap: u32,
    pub over_cap: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct IndexResult {
    pub conversations: u32,
    pub chunks: u32,
    pub embedded: u32,
    pub cached: u32,
    /// 공급자가 알려 준(없으면 추정) 입력 토큰
    pub tokens: u64,
}

fn with_prefix(prefix: &str, text: &str) -> String {
    if prefix.is_empty() {
        text.to_string()
    } else {
        format!("{prefix}{text}")
    }
}

async fn existing_rows(pool: Option<&SqlitePool>, key: &str, conv: &str) -> Result<Vec<Existing>> {
    let Some(pool) = pool else { return Ok(vec![]) };
    let rows: Vec<(String, String, i64)> =
        sqlx::query_as("SELECT first_message_id, last_message_id, closed FROM chunks WHERE model_key = ? AND conversation_id = ?")
            .bind(key)
            .bind(conv)
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(first_message_id, last_message_id, closed)| Existing {
            first_message_id,
            last_message_id,
            closed: closed != 0,
        })
        .collect())
}

pub async fn plan_conversation(db: &SqlitePool, pool: Option<&SqlitePool>, key: &str, conv: &str) -> Result<Vec<Planned>> {
    let msgs = load_messages(db, conv).await?;
    let existing = existing_rows(pool, key, conv).await?;
    Ok(chunker::plan(&msgs, &existing))
}

pub async fn preview(db: &SqlitePool, store: &MemoryStore, profile: &ApiProfiles, cfg: &MemorySetting, scope: &MemoryScope) -> Result<IndexPreview> {
    let key = model_key(profile, cfg);
    let pool = store.pool_if_exists().await?;
    let convs = eligible_conversations(db, scope).await?;
    let (mut chunks, mut cached, mut est) = (0u32, 0u32, 0u64);
    let mut seen_missing: HashSet<String> = HashSet::new();
    for (id, _) in &convs {
        let planned = plan_conversation(db, pool.as_ref(), &key, id).await?;
        let hashes: Vec<String> = planned.iter().map(|p| p.text_hash.clone()).collect();
        let have = cached_hashes(pool.as_ref(), &key, &hashes).await?;
        for p in &planned {
            chunks += 1;
            if have.contains(&p.text_hash) {
                cached += 1;
            } else if seen_missing.insert(p.text_hash.clone()) {
                est += count_tokens(&with_prefix(&cfg.doc_prefix, &p.text)) as u64;
            } else {
                cached += 1; // 같은 글이 이 실행 안에서 이미 새로 임베딩 대상이면 다시 부르지 않음
            }
        }
    }
    Ok(IndexPreview {
        conversations: convs.len() as u32,
        chunks,
        cached,
        to_embed: seen_missing.len() as u32,
        est_tokens: est,
        token_cap: cfg.token_cap,
        over_cap: est > cfg.token_cap as u64,
    })
}

fn cancelled() -> AppError {
    AppError::ai_provider(ErrorCode::AiRequestCancelled, "메모리 색인이 취소되었습니다 (이미 저장한 배치는 남음)", false)
}

async fn upsert_row(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, conv: &str, key: &str, p: &Planned, cfg: &MemorySetting) -> Result<()> {
    sqlx::query(
        "INSERT INTO chunks (conversation_id, first_message_id, last_message_id, text_hash, model_key, closed, tokens, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (conversation_id, first_message_id, model_key) DO UPDATE SET last_message_id = excluded.last_message_id, text_hash = excluded.text_hash, closed = excluded.closed, tokens = excluded.tokens",
    )
    .bind(conv)
    .bind(&p.first_message_id)
    .bind(&p.last_message_id)
    .bind(&p.text_hash)
    .bind(key)
    .bind(p.closed as i64)
    .bind(count_tokens(&with_prefix(&cfg.doc_prefix, &p.text)) as i64)
    .bind(chrono::Utc::now().timestamp_millis())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// 색인 실행. 한도(`token_cap`)를 넘는 계획은 `allow_over_cap`이 있어야 돈다.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    db: &SqlitePool,
    store: &MemoryStore,
    ai: &AiService,
    profile: &ApiProfiles,
    cfg: &MemorySetting,
    scope: &MemoryScope,
    allow_over_cap: bool,
    cancel: &CancellationToken,
) -> Result<IndexResult> {
    let prof_id = check_config(cfg)?;
    let pv = preview(db, store, profile, cfg, scope).await?;
    if pv.over_cap && !allow_over_cap {
        return Err(AppError::validation(
            format!(
                "예상 {}토큰이 실행당 상한({})을 넘음 — 상한을 늘리거나 넘겨서 진행을 확인하세요",
                pv.est_tokens, pv.token_cap
            ),
            None,
        ));
    }
    let key = model_key(profile, cfg);
    let pool = store.pool().await?;
    let convs = eligible_conversations(db, scope).await?;
    let mut res = IndexResult {
        conversations: convs.len() as u32,
        chunks: 0,
        embedded: 0,
        cached: 0,
        tokens: 0,
    };

    for (conv, _) in &convs {
        let planned = plan_conversation(db, Some(&pool), &key, conv).await?;
        let hashes: Vec<String> = planned.iter().map(|p| p.text_hash.clone()).collect();
        let have = cached_hashes(Some(&pool), &key, &hashes).await?;
        // 새로 임베딩할 chunk (같은 글은 첫 번째만)
        let mut todo: Vec<&Planned> = vec![];
        let mut queued: HashSet<&str> = HashSet::new();
        for p in &planned {
            if !have.contains(&p.text_hash) && queued.insert(p.text_hash.as_str()) {
                todo.push(p);
            }
        }
        for batch in todo.chunks(BATCH) {
            if cancel.is_cancelled() {
                return Err(cancelled());
            }
            let inputs: Vec<String> = batch.iter().map(|p| with_prefix(&cfg.doc_prefix, &p.text)).collect();
            let out = ai
                .embed(
                    EmbedInput {
                        profile_id: prof_id.clone(),
                        model: cfg.model.trim().to_string(),
                        inputs,
                        task: EmbedTask::Document,
                        dimensions: cfg.dimensions,
                    },
                    cancel,
                )
                .await?;
            let mut tx = pool.begin().await?;
            for (p, mut v) in batch.iter().zip(out.vectors) {
                if !normalize(&mut v) {
                    return Err(AppError::validation(
                        "공급자가 0이거나 숫자가 아닌 벡터를 돌려줌 — 이 배치는 저장하지 않음",
                        None,
                    ));
                }
                sqlx::query("INSERT OR REPLACE INTO vectors (model_key, text_hash, dim, vector) VALUES (?, ?, ?, ?)")
                    .bind(&key)
                    .bind(&p.text_hash)
                    .bind(v.len() as i64)
                    .bind(encode_vector(&v))
                    .execute(&mut *tx)
                    .await?;
                upsert_row(&mut tx, conv, &key, p, cfg).await?;
            }
            tx.commit().await?;
            res.embedded += batch.len() as u32;
            res.tokens += out.input_tokens;
        }
        // 나머지(캐시로 건너뛴 것·같은 글) 행을 쓰고, 계획에 없는 옛 행(바뀐 열린 chunk 등)을 지운다
        let mut tx = pool.begin().await?;
        for p in &planned {
            upsert_row(&mut tx, conv, &key, p, cfg).await?;
        }
        let keep: HashSet<&str> = planned.iter().map(|p| p.first_message_id.as_str()).collect();
        let rows: Vec<String> = sqlx::query_scalar("SELECT first_message_id FROM chunks WHERE conversation_id = ? AND model_key = ?")
            .bind(conv)
            .bind(&key)
            .fetch_all(&mut *tx)
            .await?;
        for first in rows.iter().filter(|f| !keep.contains(f.as_str())) {
            sqlx::query("DELETE FROM chunks WHERE conversation_id = ? AND model_key = ? AND first_message_id = ?")
                .bind(conv)
                .bind(&key)
                .bind(first)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        res.chunks += planned.len() as u32;
        res.cached += (planned.len() - todo.len()) as u32;
    }
    Ok(res)
}
