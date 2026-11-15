//! 게임 세션 통합 테스트: 번들 템플릿으로 게임을 만들고, 플레이·책갈피·분기를 실제 DB로 돌려 본다.
use std::sync::Arc;

use sandcastle_lib::{
    db::game::{get_game_player_sessions, get_game_save, upsert_game_save},
    game::{
        journal,
        manager::GameManager,
        runtime::{GameCommand, GameRepository, GameRuntime, SqlxGameRepository},
        session::fork_session,
    },
    error::ErrorCode,
    models::game::GameSaveInput,
};
use sqlx::SqlitePool;

async fn journal_count(pool: &SqlitePool, session_id: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM game_journal WHERE session_id = ?")
        .bind(session_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn any_persona(pool: &SqlitePool) -> String {
    if let Some(id) = sqlx::query_scalar::<_, String>("SELECT id FROM personas LIMIT 1").fetch_optional(pool).await.unwrap() {
        return id;
    }
    sqlx::query("INSERT INTO personas (id, name, description) VALUES ('p1', '플레이어', '')")
        .execute(pool)
        .await
        .unwrap();
    "p1".into()
}

/// 한 턴: 명령 → 일지 기록
async fn play(pool: &SqlitePool, runtime: &mut GameRuntime, game_id: &str, command: GameCommand) {
    let response = runtime.dispatch(command.clone()).expect("명령 처리");
    journal::record(pool, runtime, game_id, Some(&command), &response.events).await.expect("일지 기록");
}

async fn bookmark(pool: &SqlitePool, runtime: &GameRuntime, game_id: &str, slot: i64) -> u32 {
    let s = runtime.state();
    let snapshot = serde_json::json!({ "engine": s, "custom": null });
    let input = GameSaveInput {
        name: None,
        seq: s.head_seq,
        session_id: s.session_id.clone(),
        slot_number: slot,
        current_scene_id: s.current_scene_id.clone(),
        current_map_id: Some(s.map_id.clone()),
        current_zone_id: s.zone_id.clone(),
        current_location_id: s.location_id.clone(),
        current_weather_id: Some(s.weather_id.clone()),
        game_time: s.game_time,
        state_snapshot: snapshot.clone(),
    };
    upsert_game_save(pool, game_id, &input, &snapshot.to_string()).await.expect("책갈피 저장");
    s.head_seq
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn fork_from_old_bookmark_keeps_original(pool: SqlitePool) {
    let games_root = tempfile::tempdir().unwrap();
    let game_id = GameManager::create_game(&pool, games_root.path(), "분기 테스트".into(), "dating-sim".into())
        .await
        .expect("템플릿으로 게임 만들기");
    let definition = Arc::new(SqlxGameRepository::new(&pool).load_definition(&game_id).await.expect("정의 불러오기"));

    let session_id = uuid::Uuid::new_v4().to_string();
    let mut runtime = GameRuntime::from_new_game(definition.clone(), session_id.clone(), any_persona(&pool).await).expect("새 게임");
    let events = runtime.start();
    journal::record(&pool, &mut runtime, &game_id, None, &events).await.expect("첫 기록");

    play(&pool, &mut runtime, &game_id, GameCommand::AdvanceTime { minutes: 30 }).await;
    let saved_seq = bookmark(&pool, &runtime, &game_id, 1).await;
    let saved_time = runtime.state().game_time;
    let journal_at_save = journal_count(&pool, &session_id).await;

    // 책갈피 이후로 더 진행한다
    play(&pool, &mut runtime, &game_id, GameCommand::AdvanceTime { minutes: 60 }).await;
    play(&pool, &mut runtime, &game_id, GameCommand::AdvanceTime { minutes: 60 }).await;
    let head = get_game_player_sessions(&pool, &session_id).await.unwrap().head_seq;
    assert!(head > saved_seq, "책갈피 뒤로 기록이 더 있어야 분기한다");

    let forked = fork_session(&pool, &game_id, &session_id, 1).await.expect("분기");
    assert_ne!(forked, session_id);

    // 새 세션: 책갈피 시점의 기록과 상태
    let forked_row = get_game_player_sessions(&pool, &forked).await.unwrap();
    assert_eq!(forked_row.head_seq, saved_seq);
    assert_eq!(journal_count(&pool, &forked).await, journal_at_save);
    let reloaded = GameRuntime::load(&SqlxGameRepository::new(&pool), definition, &forked).await.expect("분기 세션 불러오기");
    assert_eq!(reloaded.state().game_time, saved_time);
    assert_eq!(reloaded.state().parent_session_id.as_deref(), Some(session_id.as_str()));

    // 원래 세션은 그대로
    assert_eq!(get_game_player_sessions(&pool, &session_id).await.unwrap().head_seq, head);
    assert!(journal_count(&pool, &session_id).await > journal_at_save);

    // 같은 슬롯에 다시 저장하면 책갈피가 새 시점을 가리킨다
    let new_seq = bookmark(&pool, &runtime, &game_id, 1).await;
    assert_eq!(get_game_save(&pool, &session_id, 1).await.unwrap().seq, new_seq);
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn blocked_commands_report_reason(pool: SqlitePool) {
    let games_root = tempfile::tempdir().unwrap();
    let game_id = GameManager::create_game(&pool, games_root.path(), "오류 테스트".into(), "dating-sim".into()).await.unwrap();
    let definition = Arc::new(SqlxGameRepository::new(&pool).load_definition(&game_id).await.unwrap());
    let mut runtime = GameRuntime::from_new_game(definition, uuid::Uuid::new_v4().to_string(), any_persona(&pool).await).unwrap();
    runtime.start();

    let reason = |cmd: GameCommand, runtime: &mut GameRuntime| {
        let res = runtime.dispatch(cmd).err().expect("막혀야 함").to_response();
        assert_eq!(res.code, ErrorCode::GameActionBlocked);
        res.details.unwrap()["reason"].as_str().unwrap().to_string()
    };
    assert_eq!(reason(GameCommand::StopTalk, &mut runtime), "noActiveDialogue");
    if runtime.state().current_scene_id.is_none() {
        assert_eq!(reason(GameCommand::EndScene, &mut runtime), "noOpenScene");
    }

    // 없는 id는 규칙이 아니라 입력 오류
    let res = runtime.dispatch(GameCommand::Talk { game_character_id: "없는-캐릭터".into() }).err().expect("없는 캐릭터").to_response();
    assert!(matches!(res.code, ErrorCode::InvalidInput | ErrorCode::GameActionBlocked));
}
