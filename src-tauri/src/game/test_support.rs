//! 테스트 전용 도우미: 메모리 SQLite에 마이그레이션을 돌리고 번들 템플릿(Dating Sim)을 import해서
//! 실제 앱과 같은 경로로 만든 GameDefinition을 얻는다. (정의를 손으로 조립하면 모델이 바뀔 때마다 깨진다.)
use crate::game::runtime::{GameDefinition, GameRepository, SqlxGameRepository};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;

pub const TEMPLATE_ID: &str = "dating-sim";

/// 메모리 DB. 연결마다 다른 DB가 되므로 연결은 하나만 쓴다.
pub async fn pool() -> SqlitePool {
    let opts = SqliteConnectOptions::from_str("sqlite::memory:").unwrap().foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
    pool
}

/// 번들 템플릿으로 새 게임을 만들고 (pool, game_id)를 돌려준다. 파일은 games_root/<game_id>/ 아래에 풀린다.
pub async fn dating_sim(games_root: &Path) -> (SqlitePool, String) {
    let pool = pool().await;
    let bytes = crate::game::template::package_bytes(TEMPLATE_ID).expect("번들 템플릿");
    let game_id = crate::game::package::import_bytes(&pool, games_root, bytes, None).await.expect("템플릿 import");
    (pool, game_id)
}

pub async fn definition(pool: &SqlitePool, game_id: &str) -> GameDefinition {
    SqlxGameRepository::new(pool).load_definition(game_id).await.expect("정의 로드")
}
