//! memory.db — 임베딩 벡터와 chunk 표 (ADR-015). `app.db`와 분리된 파일이라 백업·동기화·휴지통이 보지 못한다.
//! 파생 데이터라 지우거나 깨져도 다시 색인하면 된다. 처음 쓸 때 연다(플러그인이 꺼져 있으면 열지 않음).
use std::{path::PathBuf, str::FromStr};

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use tokio::sync::Mutex;

use crate::error::{AppError, Result};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS vectors (
  model_key TEXT NOT NULL,
  text_hash TEXT NOT NULL,
  dim INTEGER NOT NULL,
  vector BLOB NOT NULL,
  PRIMARY KEY (model_key, text_hash)
) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS chunks (
  id INTEGER PRIMARY KEY,
  conversation_id TEXT NOT NULL,
  first_message_id TEXT NOT NULL,
  last_message_id TEXT NOT NULL,
  text_hash TEXT NOT NULL,
  model_key TEXT NOT NULL,
  closed INTEGER NOT NULL,
  tokens INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  UNIQUE (conversation_id, first_message_id, model_key)
);
CREATE INDEX IF NOT EXISTS chunks_model ON chunks (model_key, conversation_id);
PRAGMA user_version = 1;
";

pub struct MemoryStore {
    path: PathBuf,
    pool: Mutex<Option<SqlitePool>>,
}

impl MemoryStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path, pool: Mutex::new(None) }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn file_bytes(&self) -> u64 {
        std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)
    }

    async fn open(&self) -> Result<SqlitePool> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", self.path.display()))?.create_if_missing(true);
        let pool = SqlitePoolOptions::new().max_connections(2).connect_with(opts).await?;
        sqlx::raw_sql(SCHEMA).execute(&pool).await?;
        Ok(pool)
    }

    /// 없으면 만든다 (색인·검색처럼 실제로 쓰는 순간에만)
    pub async fn pool(&self) -> Result<SqlitePool> {
        let mut g = self.pool.lock().await;
        if g.is_none() {
            *g = Some(self.open().await?);
        }
        Ok(g.as_ref().expect("방금 열었음").clone())
    }

    /// 파일이 있을 때만 연다 (상태 보기·미리보기가 파일을 만들지 않게)
    pub async fn pool_if_exists(&self) -> Result<Option<SqlitePool>> {
        if !self.exists() && self.pool.lock().await.is_none() {
            return Ok(None);
        }
        self.pool().await.map(Some)
    }

    /// 전부 지우기: 연결을 닫고 파일을 지운다
    pub async fn wipe(&self) -> Result<()> {
        let mut g = self.pool.lock().await;
        if let Some(p) = g.take() {
            p.close().await;
        }
        for suffix in ["", "-wal", "-shm"] {
            let p = PathBuf::from(format!("{}{suffix}", self.path.display()));
            match std::fs::remove_file(&p) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(AppError::io(format!("메모리 파일을 지우지 못함: {e}"))),
            }
        }
        Ok(())
    }
}

/// 길이 1로 맞춘다. 길이가 0이거나 숫자가 아닌 값이 있으면 false
pub fn normalize(v: &mut [f32]) -> bool {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if !n.is_finite() || n == 0.0 {
        return false;
    }
    v.iter_mut().for_each(|x| *x /= n);
    true
}

pub fn encode_vector(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// 정렬을 가정하지 않고 4바이트씩 읽는다 (BLOB은 정렬이 보장되지 않음)
pub fn decode_vector(b: &[u8]) -> Option<Vec<f32>> {
    if !b.len().is_multiple_of(4) {
        return None;
    }
    Some(b.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect())
}

pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vectors_round_trip_and_normalize() {
        let mut v = vec![3.0, 4.0];
        assert!(normalize(&mut v));
        assert!((v[0] - 0.6).abs() < 1e-6 && (v[1] - 0.8).abs() < 1e-6);
        assert!(!normalize(&mut [0.0, 0.0]) && !normalize(&mut [f32::NAN, 1.0]));
        let back = decode_vector(&encode_vector(&v)).unwrap();
        assert_eq!(back, v);
        assert!(decode_vector(&[1, 2, 3]).is_none());
        assert!((dot(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn opens_lazily_and_wipes() {
        let dir = tempfile::tempdir().unwrap();
        let s = MemoryStore::new(dir.path().join("memory.db"));
        assert!(!s.exists() && s.pool_if_exists().await.unwrap().is_none(), "열지 않으면 파일도 없음");
        let p = s.pool().await.unwrap();
        sqlx::query("INSERT INTO vectors VALUES ('m','h',1,x'00000000')").execute(&p).await.unwrap();
        assert!(s.exists() && s.pool_if_exists().await.unwrap().is_some());
        s.wipe().await.unwrap();
        assert!(!s.exists());
        // 지운 뒤 다시 쓰면 빈 새 파일
        let p = s.pool().await.unwrap();
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vectors").fetch_one(&p).await.unwrap();
        assert_eq!(n, 0);
    }
}
