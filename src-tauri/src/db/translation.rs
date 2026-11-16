//! 공용 번역 캐시 (translation_cache). 키는 (원문 해시, 언어)이고 원문도 함께 비교해 해시 충돌을 막는다.
use sqlx::{Error, Executor, Sqlite};

/// FNV-1a 64비트. Rust 버전·실행마다 값이 바뀌면 안 되는 저장용 키라 표준 Hasher를 쓰지 않는다
pub fn source_hash(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// 캐시된 번역 (원문이 정확히 같을 때만)
pub async fn get_translation<'e, E>(executor: E, source: &str, lang: &str) -> Result<Option<String>, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_scalar::<_, String>("SELECT text FROM translation_cache WHERE source_hash = ? AND lang = ? AND source = ?")
        .bind(source_hash(source))
        .bind(lang)
        .bind(source)
        .fetch_optional(executor)
        .await
}

pub async fn put_translation<'e, E>(executor: E, source: &str, lang: &str, text: &str) -> Result<(), Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query(
        "INSERT INTO translation_cache (source_hash, lang, source, text, created_at) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(source_hash, lang) DO UPDATE SET source = excluded.source, text = excluded.text, created_at = excluded.created_at",
    )
    .bind(source_hash(source))
    .bind(lang)
    .bind(source)
    .bind(text)
    .bind(chrono::Utc::now().timestamp_millis())
    .execute(executor)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[test]
    fn hash_is_stable() {
        // 저장된 키가 바뀌지 않도록 값 자체를 고정해 둔다
        assert_eq!(source_hash(""), "cbf29ce484222325");
        assert_eq!(source_hash("a"), "af63dc4c8601ec8c");
        assert_ne!(source_hash("안녕"), source_hash("안녕 "));
    }

    #[sqlx::test(migrations = "src/db/migrations")]
    async fn put_then_get_per_language(pool: SqlitePool) {
        assert_eq!(get_translation(&pool, "Hello", "한국어").await.unwrap(), None);
        put_translation(&pool, "Hello", "한국어", "안녕").await.unwrap();
        put_translation(&pool, "Hello", "日本語", "こんにちは").await.unwrap();
        assert_eq!(get_translation(&pool, "Hello", "한국어").await.unwrap().as_deref(), Some("안녕"));
        assert_eq!(get_translation(&pool, "Hello", "日本語").await.unwrap().as_deref(), Some("こんにちは"));
        put_translation(&pool, "Hello", "한국어", "안녕하세요").await.unwrap();
        assert_eq!(get_translation(&pool, "Hello", "한국어").await.unwrap().as_deref(), Some("안녕하세요"));
    }
}
