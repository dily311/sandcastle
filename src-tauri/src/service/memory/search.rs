//! 뜻으로 찾기 (CLAUDE.md 3-C-4, ADR-015). 질의를 임베딩(작업 하나)하고, 정규화된 벡터끼리 내적으로 상위 K만 남긴다.
//! 벡터를 전부 메모리에 올리지 않고 쪽 단위로 읽는다(모바일 대비). 결과 글은 저장해 둔 것이 아니라 살아 있는 메시지에서 다시 읽는다.
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{LazyLock, Mutex},
};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use super::{
    chunker::{self, Existing},
    store::{decode_vector, dot, normalize, MemoryStore},
    *,
};
use crate::{
    adapter::EmbedTask,
    service::ai::{AiService, EmbedInput},
};

/// 한 번에 훑는 벡터 수 상한 (넘으면 일부만 검색했다고 알림)
pub const SCAN_CAP: u64 = 50_000;
const PAGE: i64 = 500;
pub const MAX_LIMIT: u32 = 20;
const MAX_QUERY_CHARS: usize = 2000;

#[derive(Debug, Clone, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    pub scope: MemoryScope,
    /// 기본 10, 최대 20
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub conversation_id: String,
    pub conversation_title: String,
    pub first_message_id: String,
    pub last_message_id: String,
    /// 코사인 유사도 (−1~1, 모델마다 분포가 달라 절대값보다 순서가 의미 있음)
    pub score: f32,
    /// 지금 메시지에서 다시 읽은 chunk 글
    pub snippet: String,
    /// 색인한 뒤 글이 바뀜 (다시 색인하면 갱신)
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub scanned: u64,
    /// 훑는 상한에 걸려 일부만 검색함
    pub truncated: bool,
    /// 다른 모델로 색인되어 이번에 찾지 못한 chunk 수
    pub other_model_chunks: u64,
}

#[derive(Debug, Clone)]
pub struct RawHit {
    /// chunks.id (색인한 순서 — 시간 순서의 근사)
    pub chunk_id: i64,
    pub conversation_id: String,
    pub first_message_id: String,
    pub last_message_id: String,
    pub text_hash: String,
    pub score: f32,
}

#[derive(Debug, Default)]
pub struct Scan {
    pub hits: Vec<RawHit>,
    pub scanned: u64,
    pub truncated: bool,
    /// 훑은 모든 점수의 평균·분산 (Welford) — 모델마다 점수 분포가 달라 "튀는 것"을 가릴 때 쓴다
    pub mean: f64,
    pub m2: f64,
}

impl Scan {
    pub fn std(&self) -> f64 {
        if self.scanned < 2 {
            0.0
        } else {
            (self.m2 / (self.scanned - 1) as f64).sqrt()
        }
    }
    fn push_stat(&mut self, x: f64) {
        // scanned는 이미 올린 뒤에 부른다
        let n = self.scanned as f64;
        let d = x - self.mean;
        self.mean += d / n;
        self.m2 += d * (x - self.mean);
    }
}

type QueryCache = Mutex<VecDeque<(String, Vec<f32>)>>;
type ChunkRow = (i64, String, String, String, String, i64, Vec<u8>);

/// 같은 질의는 다시 임베딩하지 않는다 (작은 LRU, 저장하지 않음)
static QUERY_CACHE: LazyLock<QueryCache> = LazyLock::new(|| Mutex::new(VecDeque::new()));
const QUERY_CACHE_SIZE: usize = 16;

fn cache_get(key: &str) -> Option<Vec<f32>> {
    QUERY_CACHE.lock().ok()?.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}
fn cache_put(key: String, v: Vec<f32>) {
    if let Ok(mut c) = QUERY_CACHE.lock() {
        c.retain(|(k, _)| *k != key);
        c.push_back((key, v));
        while c.len() > QUERY_CACHE_SIZE {
            c.pop_front();
        }
    }
}

/// 상위 K 유지 (K는 작아 정렬된 배열에 끼워 넣음). 같은 글(해시)은 점수가 높은 하나만
pub(crate) fn offer(top: &mut Vec<RawHit>, k: usize, hit: RawHit) {
    if let Some(i) = top.iter().position(|h| h.text_hash == hit.text_hash) {
        if top[i].score >= hit.score {
            return;
        }
        top.remove(i);
    }
    if top.len() >= k && top.last().is_some_and(|l| l.score >= hit.score) {
        return;
    }
    let at = top.iter().position(|h| h.score < hit.score).unwrap_or(top.len());
    top.insert(at, hit);
    top.truncate(k);
}

/// chunk를 쪽 단위로 읽어 질의 벡터(정규화됨)와 내적. `allowed`에 든 대화만 (지워졌거나 제외한 대화는 건너뜀)
/// `exclude`: 이 메시지 id에서 시작하거나 끝나는 chunk는 건너뜀 (이미 프롬프트에 든 대화 등 — 통계에도 넣지 않음)
pub async fn scan(pool: &SqlitePool, key: &str, allowed: &HashSet<String>, exclude: &HashSet<String>, q: &[f32], k: usize, scan_cap: u64) -> Result<Scan> {
    let mut out = Scan::default();
    let mut last_id = 0i64;
    loop {
        let rows: Vec<ChunkRow> = sqlx::query_as(
            "SELECT c.id, c.conversation_id, c.first_message_id, c.last_message_id, c.text_hash, v.dim, v.vector
             FROM chunks c JOIN vectors v ON v.model_key = c.model_key AND v.text_hash = c.text_hash
             WHERE c.model_key = ? AND c.id > ? ORDER BY c.id LIMIT ?",
        )
        .bind(key)
        .bind(last_id)
        .bind(PAGE)
        .fetch_all(pool)
        .await?;
        if rows.is_empty() {
            break;
        }
        for (id, conversation_id, first_message_id, last_message_id, text_hash, dim, blob) in rows {
            last_id = id;
            if !allowed.contains(&conversation_id) || exclude.contains(&first_message_id) || exclude.contains(&last_message_id) {
                continue;
            }
            if out.scanned >= scan_cap {
                out.truncated = true;
                return Ok(out);
            }
            out.scanned += 1;
            let Some(v) = decode_vector(&blob).filter(|v| v.len() as i64 == dim && v.len() == q.len()) else {
                continue;
            };
            let score = dot(q, &v);
            out.push_stat(score as f64);
            offer(
                &mut out.hits,
                k,
                RawHit {
                    chunk_id: id,
                    conversation_id,
                    first_message_id,
                    last_message_id,
                    text_hash,
                    score,
                },
            );
        }
    }
    Ok(out)
}

/// 찾은 chunk를 지금 메시지로 풀어 글을 읽고, 바뀌었는지 확인한다. 메시지가 없어진 chunk는 뺀다
pub async fn resolve(db: &SqlitePool, hits: Vec<RawHit>, titles: &HashMap<String, String>) -> Result<Vec<SearchHit>> {
    let mut by_conv: HashMap<String, Vec<chunker::MemMsg>> = HashMap::new();
    let mut out = vec![];
    for h in hits {
        if !by_conv.contains_key(&h.conversation_id) {
            by_conv.insert(h.conversation_id.clone(), load_messages(db, &h.conversation_id).await?);
        }
        let msgs = &by_conv[&h.conversation_id];
        let fixed = [Existing {
            first_message_id: h.first_message_id.clone(),
            last_message_id: h.last_message_id.clone(),
            closed: true,
        }];
        let Some(p) = chunker::plan(msgs, &fixed)
            .into_iter()
            .find(|p| p.first_message_id == h.first_message_id && p.last_message_id == h.last_message_id)
        else {
            continue; // 메시지가 지워졌거나 숨겨짐
        };
        out.push(SearchHit {
            conversation_title: titles.get(&h.conversation_id).cloned().unwrap_or_default(),
            conversation_id: h.conversation_id,
            first_message_id: h.first_message_id,
            last_message_id: h.last_message_id,
            score: h.score,
            snippet: p.body,
            stale: p.text_hash != h.text_hash,
        });
    }
    Ok(out)
}

/// 같은 모델·같은 질의의 벡터가 이미 있으면 (공급자를 부르지 않으므로 작업도 만들 필요가 없다)
pub fn cached_query(key: &str, query: &str) -> Option<Vec<f32>> {
    cache_get(&format!("{key}\u{0}{query}"))
}

/// 질의 벡터(정규화됨): 캐시에 없으면 공급자를 한 번 부른다. 부르는 쪽이 작업 안에서 부른다
pub async fn query_vector(ai: &AiService, cfg: &MemorySetting, profile_id: &str, key: &str, query: &str, cancel: &CancellationToken) -> Result<Vec<f32>> {
    let cache_key = format!("{key}\u{0}{query}");
    if let Some(v) = cache_get(&cache_key) {
        return Ok(v);
    }
    let out = ai
        .embed(
            EmbedInput {
                profile_id: profile_id.to_string(),
                model: cfg.model.trim().to_string(),
                inputs: vec![format!("{}{query}", cfg.query_prefix)],
                task: EmbedTask::Query,
                dimensions: cfg.dimensions,
            },
            cancel,
        )
        .await?;
    let mut v = out.vectors.into_iter().next().ok_or_else(|| AppError::internal("질의 임베딩이 비어 있음"))?;
    if !normalize(&mut v) {
        return Err(AppError::validation("공급자가 0이거나 숫자가 아닌 벡터를 돌려줌", None));
    }
    cache_put(cache_key, v.clone());
    Ok(v)
}

/// 질의 임베딩(공급자 호출) → 검색. 부르는 쪽이 작업 안에서 부른다
pub async fn search(
    db: &SqlitePool,
    store: &MemoryStore,
    ai: &AiService,
    profile: &ApiProfiles,
    cfg: &MemorySetting,
    req: &SearchRequest,
    cancel: &CancellationToken,
) -> Result<SearchResult> {
    let prof_id = check_config(cfg)?;
    let query = req.query.trim();
    if query.is_empty() {
        return Err(AppError::validation("찾을 말을 입력하세요", None));
    }
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(AppError::validation(format!("찾을 말은 {MAX_QUERY_CHARS}자까지"), None));
    }
    let key = model_key(profile, cfg);
    let Some(pool) = store.pool_if_exists().await? else {
        return Ok(SearchResult {
            hits: vec![],
            scanned: 0,
            truncated: false,
            other_model_chunks: 0,
        });
    };
    let convs = eligible_conversations(db, &req.scope).await?;
    let allowed: HashSet<String> = convs.iter().map(|(id, _)| id.clone()).collect();
    let titles: HashMap<String, String> = convs.into_iter().collect();

    let q = query_vector(ai, cfg, &prof_id, &key, query, cancel).await?;
    let limit = req.limit.unwrap_or(10).clamp(1, MAX_LIMIT) as usize;
    let s = scan(&pool, &key, &allowed, &HashSet::new(), &q, limit, SCAN_CAP).await?;
    let other: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chunks WHERE model_key <> ?")
        .bind(&key)
        .fetch_one(&pool)
        .await?;
    Ok(SearchResult {
        hits: resolve(db, s.hits, &titles).await?,
        scanned: s.scanned,
        truncated: s.truncated,
        other_model_chunks: other as u64,
    })
}
