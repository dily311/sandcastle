//! 성능 측정 (CLAUDE.md 1-D-5). 평소 테스트에서는 빠지고, 잴 때만:
//!   cargo test --release --lib perf_tests -- --ignored --nocapture
//! 결과와 기준은 docs/성능 측정.md
use std::time::Instant;

use crate::{
    db::{self, conn::DbPool},
    models::conversation::MessageInput,
    service::chat::regex_processor::{RegexMacros, RegexProcessor},
};

async fn app() -> (tempfile::TempDir, std::path::PathBuf, DbPool) {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("appdata");
    let pool = db::conn::init_pool(base.clone()).await.unwrap();
    (dir, base, pool)
}

fn ms(t: Instant) -> u128 {
    t.elapsed().as_millis()
}

#[tokio::test]
#[ignore = "성능 측정 (수동)"]
async fn long_conversation_load_and_display() {
    let (_d, _base, pool) = app().await;
    sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c1', 'C', 0, 0)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO conversations (id, title, character_id, persona_id, mode, created_at, updated_at) VALUES ('v1', 't', 'c1', NULL, 'chat', 0, 0)")
        .execute(&pool)
        .await
        .unwrap();
    // 정규식 몇 개 (표시용)
    for (i, find) in ["/\\*\\*(.+?)\\*\\*/g", "/\"([^\"]+)\"/g", "/<status>[\\s\\S]*?<\\/status>/g"]
        .iter()
        .enumerate()
    {
        sqlx::query(
            "INSERT INTO regex_scripts (id, name, find_regex, replace_string, trim_strings, placement, target, scope, enabled, sort_order, created_at, updated_at)
             VALUES (?, ?, ?, '<b>$1</b>', '[]', '[\"ai_output\"]', 'display', 'global', 1, ?, 0, 0)",
        )
        .bind(format!("r{i}"))
        .bind(format!("r{i}"))
        .bind(find)
        .bind(i as i64)
        .execute(&pool)
        .await
        .unwrap();
    }
    let body = "긴 메시지 본문입니다. **굵게**와 *기울임*, 그리고 대사 \"안녕?\" 이 섞여 있습니다. ".repeat(6);
    let t = Instant::now();
    let mut tx = pool.begin().await.unwrap();
    for i in 0..5000i64 {
        let input = MessageInput {
            conversation_id: "v1".into(),
            seq: i + 1,
            role: if i % 2 == 0 { "user".into() } else { "assistant".into() },
            name: None,
            content: format!("#{i} {body}"),
            character_id: Some("c1".into()),
            meta: None,
        };
        db::conversation::insert_message_at(&mut *tx, input, i).await.unwrap();
    }
    tx.commit().await.unwrap();
    println!("[perf] 메시지 5000개 넣기: {}ms", ms(t));

    let t = Instant::now();
    let msgs = db::conversation::get_messages(&pool, "v1", 2000, None).await.unwrap();
    println!("[perf] 대화 열기 (최근 2000개 읽기): {}ms, {}개", ms(t), msgs.len());

    let t = Instant::now();
    let regex = RegexProcessor::load(
        &pool,
        Some("c1"),
        None,
        RegexMacros {
            char_name: "C".into(),
            user_name: "U".into(),
        },
    )
    .await
    .unwrap();
    let n = msgs.len();
    let shown: Vec<String> = msgs
        .iter()
        .enumerate()
        .map(|(i, m)| regex.for_display(&m.content, &m.role, n - 1 - i))
        .collect();
    println!(
        "[perf] 표시용 정규식 2000개 (display_messages): {}ms ({}자)",
        ms(t),
        shown.iter().map(String::len).sum::<usize>()
    );

    let t = Instant::now();
    let _ = db::conversation::get_messages(&pool, "v1", 1, None).await.unwrap();
    println!("[perf] 마지막 메시지 하나 (사이드바 미리보기): {}ms", ms(t));
}

#[tokio::test]
#[ignore = "성능 측정 (수동)"]
async fn gallery_listing() {
    let (_d, base, _pool) = app().await;
    let dir = base.join("generated");
    std::fs::create_dir_all(&dir).unwrap();
    let png = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/sillyTavern/character/Seraphina/joy.png")).unwrap();
    for i in 0..3000 {
        std::fs::write(dir.join(format!("img-{i:04}.png")), &png).unwrap();
        std::fs::write(
            dir.join(format!("img-{i:04}.json")),
            format!("{{\"path\":\"x\",\"prompt\":\"p{i}\",\"model\":\"m\",\"source\":\"upload\",\"createdAt\":{i}}}"),
        )
        .unwrap();
    }
    let t = Instant::now();
    let list = crate::service::plugins::list_images(&dir);
    println!("[perf] 갤러리 3000장 목록: {}ms, {}장", ms(t), list.len());
}
