//! 세션(책) 단위 작업: 책갈피에서 분기(fork), 턴 시점(seq)에서 분기.
//!
//! 분기: 책갈피의 일지 위치(seq)까지를 새 세션으로 복사하고, 거기서부터 이어서 플레이한다.
//! 원래 세션/기록은 그대로 남는다. 복사 대상은 일지, 이벤트 로그, 대화(+참여자)와 메시지, 그 책갈피 자신.
//! 상태(위치/시간/날씨/변수/씬/대기열)는 세션 테이블의 "지금" 값이 아니라 책갈피의 스냅샷에서 복원한다.
//! 전부 한 트랜잭션이라 중간에 실패하면 새 세션이 남지 않는다.
use crate::db::game::{
    forked_game_character_states, forked_game_conversations, forked_game_conversations_participant, forked_game_event_logs,
    forked_game_journal, forked_game_messages, forked_game_relationship_snapshots, forked_game_save, forked_game_story_plans, get_game_journal_lookup, get_game_save,
    upsert_game_player_sessions,
};
use crate::game::journal::STATE_KIND;
use crate::error::AppError;
use crate::game::runtime::GameState;
use sqlx::SqlitePool;

/// parent 세션의 slot 책갈피에서 분기한 새 세션 id를 돌려준다.
pub async fn fork_session(pool: &SqlitePool, game_id: &str, parent_session_id: &str, slot: i64) -> Result<String, AppError> {
    let save = get_game_save(pool, parent_session_id, slot).await?;
    // GameState는 cooldowns/auto_active/active_npc_id/turn_fired를 직렬화에서 뺐으므로
    // 불러온 쪽이 비워진 채로 시작한다 (쿨다운은 복사된 이벤트 로그에서 다시 채워진다).
    let engine = save
        .state_snapshot
        .get("engine")
        .filter(|v| !v.is_null())
        .cloned()
        .ok_or_else(|| AppError::game_invalid_state("이 책갈피에는 복원할 상태 스냅샷이 없습니다"))?;
    fork_from(pool, game_id, parent_session_id, serde_json::from_value(engine)?, save.seq, Some(slot)).await
}

/// parent 세션의 seq 시점(턴 끝 상태 스냅샷이 있는 곳)에서 분기한 새 세션 id를 돌려준다.
pub async fn fork_at_seq(pool: &SqlitePool, game_id: &str, parent_session_id: &str, seq: u32) -> Result<String, AppError> {
    let row = get_game_journal_lookup(pool, parent_session_id, Some(seq), seq)
        .await?
        .into_iter()
        .find(|r| r.kind == STATE_KIND)
        .ok_or_else(|| AppError::game_invalid_state("이 시점에는 분기할 상태가 남아 있지 않습니다"))?;
    fork_from(pool, game_id, parent_session_id, serde_json::from_value(row.payload_json)?, seq, None).await
}

/// 상태(state)와 seq를 받아 parent의 기록을 seq까지 복사한 새 세션을 만든다.
/// copy_save_slot이 있으면 그 책갈피도 새 세션으로 복사한다.
async fn fork_from(pool: &SqlitePool, game_id: &str, parent_session_id: &str, mut state: GameState, to_seq: u32, copy_save_slot: Option<i64>) -> Result<String, AppError> {
    let new_session_id = uuid::Uuid::new_v4().to_string();
    state.session_id = new_session_id.clone();
    state.parent_session_id = Some(parent_session_id.to_string());
    state.forked_from_seq = Some(to_seq);
    state.head_seq = to_seq; // 복사된 일지가 to_seq까지라서, 새 번호는 그 다음부터 발급된다.

    let mut tx = pool.begin().await?;
    // 세션 row를 먼저 만들어야 아래 복사들의 FK(session_id)가 걸린다.
    upsert_game_player_sessions(&mut *tx, state.to_session_input(game_id.to_string())).await?;
    forked_game_journal(&mut *tx, parent_session_id, &new_session_id, None, to_seq).await?;
    forked_game_event_logs(&mut *tx, parent_session_id, &new_session_id, None, to_seq).await?;
    // 관계도/NPC 상태/스토리 플랜은 seq별로 쌓이는 기록이라 같은 구간까지 복사한다.
    forked_game_relationship_snapshots(&mut *tx, parent_session_id, &new_session_id, None, to_seq).await?;
    forked_game_character_states(&mut *tx, parent_session_id, &new_session_id, None, to_seq).await?;
    forked_game_story_plans(&mut *tx, parent_session_id, &new_session_id, None, to_seq).await?;

    // 대화는 세션에 묶여 있어서 통째로 복사하고, 메시지는 to_seq까지만 복사한다.
    let conversation_pairs = forked_game_conversations(&mut tx, parent_session_id, &new_session_id).await?;
    for (old_id, new_id) in &conversation_pairs {
        forked_game_messages(&mut tx, old_id, new_id, None, to_seq).await?;
        forked_game_conversations_participant(&mut *tx, old_id, new_id).await?;
    }
    if let Some(slot) = copy_save_slot {
        forked_game_save(&mut *tx, parent_session_id, &new_session_id, slot).await?;
    }
    crate::db::game::forked_game_session_options(&mut *tx, parent_session_id, &new_session_id).await?;
    tx.commit().await?;

    Ok(new_session_id)
}
