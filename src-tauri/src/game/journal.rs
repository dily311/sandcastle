//! 턴이 끝난 뒤 "무슨 일이 있었는지"를 DB에 남기는 공용 함수. dispatch_command(커맨드)와
//! npc_say(AI가 바꾼 변수)가 같이 쓴다.
//!   - game_journal: 커맨드 한 줄 + 결과 이벤트들 (세션 전체 seq 순서)
//!   - game_event_log(+_characters): 발동된 이벤트. 앱을 다시 켰을 때 쿨다운/once가 이어지려면 DB에 있어야 한다.
//!   - 세션 저장: 위치/시간/날씨/씬/대기열/head_seq
//! 전부 한 트랜잭션이다 — 분기 복사가 일지를 믿고 seq로 자르기 때문에, 일부만 남은 턴이 있으면 안 된다.
//! (실패하면 메모리의 head_seq만 앞서가는데, 번호에 구멍이 생길 뿐이라 무해하다.)
use crate::db::game::{upsert_game_character_state, upsert_game_event_log, upsert_game_event_log_character, upsert_game_journal, upsert_game_player_sessions, upsert_game_relationship_snapshot};
use crate::error::AppError;
use crate::game::runtime::{GameCommand, GameEvent, GameRuntime};
use crate::models::game::{GameRelationshipSnapshots, GameCharacterStatesSnapshotsInput, GameEventLogCharactersInput, GameEventLogInput, GameJournalInput};
use sqlx::SqlitePool;

pub fn journal_input<T: serde::Serialize>(session_id: &str, seq: u32, kind: &str, payload: &T, game_time: i64) -> GameJournalInput {
    GameJournalInput {
        session_id: session_id.to_string(),
        seq,
        kind: kind.to_string(),
        payload_json: serde_json::to_value(payload).unwrap_or(serde_json::Value::Null),
        game_time,
    }
}

pub async fn record(db: &SqlitePool, runtime: &mut GameRuntime, game_id: &str, command: Option<&GameCommand>, events: &[GameEvent]) -> Result<(), AppError> {
    let session_id = runtime.state().session_id.clone();
    let weather_id = runtime.state().weather_id.clone();
    let game_time = runtime.state().game_time;

    let mut journal = Vec::new();
    if let Some(cmd) = command {
        journal.push(journal_input(&session_id, runtime.next_seq(), "command", cmd, game_time));
    }
    let mut event_logs = Vec::new();
    let mut npc_rows = Vec::new();
    let mut rel_rows = Vec::new();
    for event in events {
        let seq = runtime.next_seq();
        journal.push(journal_input(&session_id, seq, "event", event, game_time));
        if let GameEvent::EventTriggered { event_id, npc_id, .. } = event {
            event_logs.push((seq, event_id.clone(), npc_id.clone()));
        }
        // NPC 위치는 변화량만 쌓는다 (바뀐 NPC 한 명당 한 행). 최신 상태는 캐릭터별 가장 큰 seq의 행이다.
        // 관계도는 변화량만 쌓는다 (바뀐 축 하나당 한 행: {"from>to": {축: 변화량}}). 현재 값은 세션의 relationships_json이 가진다.
        if let GameEvent::RelationChanged { from, to, axis, old, new } = event {
            rel_rows.push((seq, format!("{from}>{to}"), axis.clone(), new - old));
        }
        if let GameEvent::NpcMoved { game_character_id, to, .. } = event {
            npc_rows.push((seq, game_character_id.clone(), to.clone()));
        }
    }

    let mut tx = db.begin().await?;
    // 세션 행을 가장 먼저 쓴다: 새 게임의 첫 기록은 세션이 아직 없는 상태라, 아래 행들의 session_id FK가 걸리려면 먼저 있어야 한다.
    upsert_game_player_sessions(&mut *tx, runtime.state().to_session_input(game_id.to_string())).await?;
    for j in journal {
        upsert_game_journal(&mut *tx, j).await?;
    }
    for (seq, event_id, npc_id) in event_logs {
        upsert_game_event_log(
            &mut *tx,
            GameEventLogInput { seq, session_id: session_id.clone(), event_id, weather_id: weather_id.clone(), triggered_at: game_time as f32, result_json: serde_json::json!({}) },
        )
        .await?;
        if let Some(npc) = npc_id {
            // game_event_log에 id가 없어서 로그는 (session_id, seq)로 가리킨다.
            upsert_game_event_log_character(&mut *tx, GameEventLogCharactersInput { session_id: session_id.clone(), seq, game_character_id: npc, role: None, sort_order: 0 }).await?;
        }
    }
    for (seq, game_character_id, to) in npc_rows {
        let status = if to.is_some() { "active" } else { "absent" };
        upsert_game_character_state(
            &mut *tx,
            GameCharacterStatesSnapshotsInput { session_id: session_id.clone(), seq, game_character_id, current_location_id: to, status: status.to_string(), state_json: serde_json::json!({}) },
        )
        .await?;
    }
    for (seq, key, axis, delta) in rel_rows {
        upsert_game_relationship_snapshot(
            &mut *tx,
            GameRelationshipSnapshots { session_id: session_id.clone(), seq, relationships_json: serde_json::json!({ key: { axis: delta } }), created_at: chrono::Utc::now().timestamp() },
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
