//! 턴이 끝난 뒤 "무슨 일이 있었는지"를 DB에 남기는 공용 함수. dispatch_command(커맨드)와
//! npc_say(AI가 바꾼 변수)가 같이 쓴다.
//!   - game_journal: 커맨드 한 줄 + 결과 이벤트들 (세션 전체 seq 순서)
//!   - game_event_log(+_characters): 발동된 이벤트. 앱을 다시 켰을 때 쿨다운/once가 이어지려면 DB에 있어야 한다.
//!   - 세션 저장: 위치/시간/날씨/씬/대기열/head_seq
//! ⚠ 여러 테이블에 순서대로 쓰고 한 트랜잭션으로 묶지는 않았다 — 중간에 실패하면 일지만 남을 수 있다.
//!   (분기 복사가 이 데이터를 믿고 자르므로 나중에 트랜잭션으로 묶는 걸 추천.)
use crate::db::game::upsert_game_journal;
use crate::error::AppError;
use crate::game::runtime::{
    GameCommand, GameEvent, GameRepository, GameRuntime, SqlxGameRepository,
};
use crate::models::game::{GameEventLogCharactersInput, GameEventLogInput, GameJournalInput};
use sqlx::SqlitePool;

pub fn journal_input<T: serde::Serialize>(
    session_id: &str,
    seq: u32,
    kind: &str,
    payload: &T,
    game_time: i64,
) -> GameJournalInput {
    GameJournalInput {
        session_id: session_id.to_string(),
        seq,
        kind: kind.to_string(),
        payload_json: serde_json::to_value(payload).unwrap_or(serde_json::Value::Null),
        game_time,
    }
}

pub async fn record(
    db: &SqlitePool,
    runtime: &mut GameRuntime,
    game_id: &str,
    command: Option<&GameCommand>,
    events: &[GameEvent],
) -> Result<(), AppError> {
    let session_id = runtime.state().session_id.clone();
    let weather_id = runtime.state().weather_id.clone();
    let game_time = runtime.state().game_time;

    let mut journal = Vec::new();
    if let Some(cmd) = command {
        journal.push(journal_input(
            &session_id,
            runtime.next_seq(),
            "command",
            cmd,
            game_time,
        ));
    }
    let mut event_logs = Vec::new();
    for event in events {
        let seq = runtime.next_seq();
        journal.push(journal_input(&session_id, seq, "event", event, game_time));
        if let GameEvent::EventTriggered {
            event_id, npc_id, ..
        } = event
        {
            event_logs.push((seq, event_id.clone(), npc_id.clone()));
        }
    }

    let repo = SqlxGameRepository::new(db);
    for j in journal {
        upsert_game_journal(db, j).await?;
    }
    for (seq, event_id, npc_id) in event_logs {
        repo.append_event_log(GameEventLogInput {
            seq,
            session_id: session_id.clone(),
            event_id,
            weather_id: weather_id.clone(),
            triggered_at: game_time as f32,
            result_json: serde_json::json!({}),
        })
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
        if let Some(npc) = npc_id {
            // game_event_log에 id가 없어졌으니 로그는 (session_id, seq)로 가리킨다.
            // ⚠ [가정] GameEventLogCharactersInput { session_id, seq, game_character_id, role, sort_order }
            repo.append_event_log_character(GameEventLogCharactersInput {
                session_id: session_id.clone(),
                seq,
                game_character_id: npc,
                role: None,
                sort_order: 0,
            })
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
    }
    // 세션 저장 (위치/시간/날씨/씬/대기열/head_seq).
    runtime
        .save(&repo, game_id)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}
