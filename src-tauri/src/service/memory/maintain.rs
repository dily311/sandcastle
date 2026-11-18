//! 색인 무효화 (CLAUDE.md 3-C-6, ADR-015 추가). 메시지를 고치거나 지우거나 대화를 지우면 그 chunk의 색인을 맞춘다.
//!
//! - 공급자를 부르지 않는다(요금 없음). 바뀐 chunk의 **행만** 지우고 벡터 캐시는 남긴다 — 되돌려 놓거나 휴지통에서 되살리면 다시 임베딩하지 않아도 된다.
//! - 다시 색인하면(색인 실행) 없어진 chunk만 새로 임베딩한다.
//! - 메모리 플러그인이 꺼져 있거나 `memory.db`가 없으면 아무것도 하지 않는다. 단, 대화를 지울 때는 플러그인이 꺼져 있어도(파일이 있으면) 그 대화의 색인 행을 지운다 — 지운 대화의 파생 데이터가 남지 않게.
//! - 지금 하는 일은 모두 백그라운드(커맨드를 기다리게 하지 않음)이고 실패는 무시한다(다음 검색·색인이 어차피 검증).
use std::collections::HashMap;

use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use super::{
    chunker::{self, Existing},
    store::MemoryStore,
    *,
};

/// 한 대화의 색인 행을 지금 메시지와 맞춘다: 끝 메시지가 없어졌거나 글(해시)이 달라진 chunk의 행을 지운다. 지운 행 수
pub async fn reconcile(db: &SqlitePool, store: &MemoryStore, conversation_id: &str) -> Result<u64> {
    let Some(pool) = store.pool_if_exists().await? else { return Ok(0) };
    let rows: Vec<(i64, String, String, String)> =
        sqlx::query_as("SELECT id, first_message_id, last_message_id, text_hash FROM chunks WHERE conversation_id = ?")
            .bind(conversation_id)
            .fetch_all(&pool)
            .await?;
    if rows.is_empty() {
        return Ok(0);
    }
    let alive: Option<i64> = sqlx::query_scalar("SELECT 1 FROM conversations WHERE id = ?")
        .bind(conversation_id)
        .fetch_optional(db)
        .await?;
    if alive.is_none() {
        return forget_conversation(store, conversation_id).await;
    }
    let msgs = load_messages(db, conversation_id).await?;
    // 모든 행의 경계를 고정해 다시 계획하면 행마다 지금 글(해시)이 나온다
    let mut seen = std::collections::HashSet::new();
    let existing: Vec<Existing> = rows
        .iter()
        .filter(|(_, f, l, _)| seen.insert((f.clone(), l.clone())))
        .map(|(_, f, l, _)| Existing {
            first_message_id: f.clone(),
            last_message_id: l.clone(),
            closed: true,
        })
        .collect();
    let now: HashMap<(String, String), String> = chunker::plan(&msgs, &existing)
        .into_iter()
        .map(|p| ((p.first_message_id, p.last_message_id), p.text_hash))
        .collect();
    let mut removed = 0;
    for (id, first, last, hash) in rows {
        if now.get(&(first, last)) != Some(&hash) {
            removed += sqlx::query("DELETE FROM chunks WHERE id = ?").bind(id).execute(&pool).await?.rows_affected();
        }
    }
    Ok(removed)
}

/// 대화의 색인 행을 모두 지운다 (벡터 캐시는 남김 — 휴지통에서 되살리면 다시 임베딩하지 않아도 됨)
pub async fn forget_conversation(store: &MemoryStore, conversation_id: &str) -> Result<u64> {
    let Some(pool) = store.pool_if_exists().await? else { return Ok(0) };
    Ok(sqlx::query("DELETE FROM chunks WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&pool)
        .await?
        .rows_affected())
}

/// 메시지 id들이 속한 대화 id들 (메시지를 지우기 전에 불러 둔다)
pub async fn conversations_of(db: &SqlitePool, message_ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for id in message_ids {
        if let Ok(Some(c)) = sqlx::query_scalar::<_, String>("SELECT conversation_id FROM messages WHERE id = ?")
            .bind(id)
            .fetch_optional(db)
            .await
        {
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    out
}

fn store_of(app: &AppHandle) -> Option<std::sync::Arc<MemoryStore>> {
    app.try_state::<std::sync::Arc<MemoryStore>>().map(|s| s.inner().clone()).filter(|s| s.exists())
}

/// 메시지가 바뀐 뒤 (고침·지움·숨김·스와이프): 그 대화들의 색인을 백그라운드에서 맞춘다. 플러그인이 꺼져 있으면 아무것도 하지 않는다
pub fn after_message_change(app: &AppHandle, db: &SqlitePool, conversation_ids: Vec<String>) {
    if conversation_ids.is_empty() || !crate::sys::plugins::is_enabled(app, crate::sys::plugins::id::MEMORY) {
        return;
    }
    let Some(store) = store_of(app) else { return };
    let db = db.clone();
    tauri::async_runtime::spawn(async move {
        for c in conversation_ids {
            if let Err(e) = reconcile(&db, &store, &c).await {
                eprintln!("[memory] 색인 맞추기 실패 (다음 색인·검색 때 다시 검증): {e}");
            }
        }
    });
}

/// 대화를 지운 뒤: 그 대화의 색인 행을 지운다 (플러그인이 꺼져 있어도 — 지운 대화의 파생 데이터가 남지 않게)
pub fn after_conversation_deleted(app: &AppHandle, conversation_id: &str) {
    let Some(store) = store_of(app) else { return };
    let id = conversation_id.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = forget_conversation(&store, &id).await {
            eprintln!("[memory] 지운 대화의 색인을 지우지 못함 (정리 버튼으로 지울 수 있음): {e}");
        }
    });
}

/// 휴지통에서 완전히 지운 뒤: 어디서도 가리키지 않는 벡터를 지운다 (지운 대화의 파생 데이터가 파일에 남지 않게)
pub fn after_permanent_delete(app: &AppHandle, db: &SqlitePool) {
    let Some(store) = store_of(app) else { return };
    let db = db.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = purge(&store, &db, None, false).await {
            eprintln!("[memory] 남은 벡터를 정리하지 못함: {e}");
        }
    });
}
