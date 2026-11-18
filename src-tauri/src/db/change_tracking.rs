//! 변경 추적 (CLAUDE.md 2-A-2, ADR-011).
//!
//! 동기화 대상 테이블은 모두 `updated_at`(밀리초)을 갖고, 마이그레이션 156의 트리거가 모든 쓰기에서 갱신한다:
//! - INSERT에서 `updated_at`이 비었거나 0이면 지금 시각
//! - UPDATE에서 `updated_at`을 바꾸지 않았으면 지금 시각, 직접 바꿨으면 그 값을 둔다(동기화 병합이 원래 시각을 쓸 수 있게)
//!
//! 지우기는 마이그레이션 157의 트리거가 `sync_tombstones`에 (테이블, 키)로 남기고, 같은 키로 다시 들어오면 지운다.
//! 휴지통(되살리기)은 `service/data/trash.rs`.
//!
//! 동기화하지 않는 테이블은 아래 목록에만 적는다. 새 테이블은 둘 중 하나에 들어가야 테스트가 통과한다.

/// 동기화하지 않는 테이블 (ADR-011 표의 실행 기록·캐시·기기 전용)
pub const UNSYNCED_TABLES: &[&str] = &[
    // 실행 기록 — 기기마다 따로 센다
    "generation_tasks",
    "task_attempts",
    "ai_submissions",
    "ai_attempts",
    "conversation_events",
    "message_prompts",
    "usage_log",
    "usage_daily",
    "agent_states",
    "agent_tool_calls",
    // 캐시 — 다시 만들 수 있다
    "translation_cache",
    "memory_embeddings",
    // 기기 전용 (위젯·음악)
    "widget_characters",
    "widget_character_settings",
    "music_meta",
    "music_album_art_pos",
    "playlist",
    "playlist_entries",
    // 기기마다의 휴지통과 지운 기록 자체
    "trash_items",
    "sync_tombstones",
    // 동기화 상태 자체 (기기마다)
    "sync_peers",
    "sync_conflicts",
    "sync_runs",
    "sync_row_origin",
    "sync_id_map",
    // 커뮤니티 플러그인의 켜짐·허락 — 기기마다 (다른 기기에서 받은 코드가 저절로 돌지 않게, ADR-012)
    "user_plugins",
    "user_plugin_storage",
    // 고치기 제안 (6-D-3) — 적용한 결과는 원래 테이블에서 동기화된다
    "change_proposals",
];

/// 동기화 대상 테이블 이름 (DB에서 읽음 — 마이그레이션이 늘어도 따로 고칠 목록이 없다)
pub async fn synced_tables<'e, E>(executor: E) -> Result<Vec<String>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let all: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '\\_sqlx%' ESCAPE '\\' ORDER BY name",
    )
    .fetch_all(executor)
    .await?;
    Ok(all.into_iter().filter(|t| !UNSYNCED_TABLES.contains(&t.as_str())).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::SqlitePool;

    async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
        sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
        pool
    }

    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    #[tokio::test]
    async fn every_table_is_classified_and_tracked() {
        let pool = pool().await;
        let synced = synced_tables(&pool).await.unwrap();
        assert!(synced.len() > 60, "동기화 대상이 너무 적음: {}", synced.len());
        for t in &synced {
            let cols: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info(?)")
                .bind(t)
                .fetch_all(&pool)
                .await
                .unwrap();
            assert!(
                cols.iter().any(|c| c == "updated_at"),
                "{t}: updated_at이 없음 — 동기화 대상이면 컬럼과 트리거를 붙이고, 아니면 UNSYNCED_TABLES에 적기 (ADR-011)"
            );
            for kind in ["touch_insert", "touch_update", "tombstone", "untombstone"] {
                let name = format!("{t}_{kind}");
                let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND name = ? AND tbl_name = ?")
                    .bind(&name)
                    .bind(t)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                assert_eq!(n, 1, "{name} 트리거가 없음");
            }
        }
        for t in UNSYNCED_TABLES {
            assert!(!synced.contains(&t.to_string()));
        }
    }

    #[tokio::test]
    async fn writes_touch_updated_at() {
        let pool = pool().await;
        let before = now_ms() - 1_000; // SQLite의 "지금"은 Windows에서 약 15ms 단위라 넉넉히

        // 시각 없이 넣으면 지금 시각
        sqlx::query("INSERT INTO message_tags (id, name) VALUES ('t1', 'a')")
            .execute(&pool)
            .await
            .unwrap();
        let at: i64 = sqlx::query_scalar("SELECT updated_at FROM message_tags WHERE id = 't1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(at >= before, "INSERT가 updated_at을 채우지 않음: {at}");

        // 시각을 건드리지 않은 UPDATE도 갱신
        sqlx::query("UPDATE message_tags SET updated_at = 5 WHERE id = 't1'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE message_tags SET name = 'b' WHERE id = 't1'").execute(&pool).await.unwrap();
        let at: i64 = sqlx::query_scalar("SELECT updated_at FROM message_tags WHERE id = 't1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(at >= before, "UPDATE가 updated_at을 갱신하지 않음: {at}");

        // 직접 준 시각은 그대로 (병합이 원래 시각을 쓴다)
        sqlx::query("UPDATE message_tags SET name = 'c', updated_at = 1234 WHERE id = 't1'")
            .execute(&pool)
            .await
            .unwrap();
        let at: i64 = sqlx::query_scalar("SELECT updated_at FROM message_tags WHERE id = 't1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(at, 1234);

        // 복합 키 연결 테이블·키 없는 테이블도 같음
        sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c1', 'A', 0, 0)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO worldinfo_books (id, name, created_at, updated_at) VALUES ('b1', 'B', 0, 0)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO character_worldinfo (character_id, book_id) VALUES ('c1', 'b1')")
            .execute(&pool)
            .await
            .unwrap();
        let at: i64 = sqlx::query_scalar("SELECT updated_at FROM character_worldinfo").fetch_one(&pool).await.unwrap();
        assert!(at >= before);
        let at: i64 = sqlx::query_scalar("SELECT updated_at FROM characters WHERE id = 'c1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(at >= before, "0으로 넣은 시각도 지금 시각으로");
    }

    #[tokio::test]
    async fn db_functions_that_skip_updated_at_still_touch_it() {
        // update_character_is_favorite는 updated_at을 직접 쓰지 않는다 — 트리거가 채워야 한다
        let pool = pool().await;
        sqlx::query("INSERT INTO characters (id, name, created_at, updated_at) VALUES ('c1', 'A', 1, 1)")
            .execute(&pool)
            .await
            .unwrap();
        let before = now_ms() - 1_000; // SQLite의 "지금"은 Windows에서 약 15ms 단위라 넉넉히
        crate::db::conversation::update_character_is_favorite(&pool, "c1", true).await.unwrap();
        let at: i64 = sqlx::query_scalar("SELECT updated_at FROM characters WHERE id = 'c1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(at >= before, "즐겨찾기 바꾸기가 updated_at을 갱신하지 않음: {at}");
    }

    #[tokio::test]
    async fn seconds_were_normalized_to_millis() {
        let pool = pool().await;
        let (c, u): (i64, i64) = sqlx::query_as("SELECT created_at, updated_at FROM personas WHERE id = 'default'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(c > 100_000_000_000 && u > 100_000_000_000, "기본 페르소나 시각이 초 단위로 남음: {c} {u}");
    }
}
