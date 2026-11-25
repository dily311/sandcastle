//! 빠른 답장 세트 (5-E-20-5-4)
use sqlx::{Error, Executor, Sqlite};
use uuid::Uuid;

use crate::models::quick_reply::*;

const COLUMNS: &str = "id, name, scope, scope_id, enabled, auto_send, items, sort_order, created_at, updated_at";

pub async fn get_all_quick_reply_sets<'e, E>(executor: E) -> Result<Vec<QuickReplySet>, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let sql = format!("SELECT {COLUMNS} FROM quick_reply_sets ORDER BY scope, sort_order, created_at");
    let rows = sqlx::query_as::<_, QuickReplySetRow>(sqlx::AssertSqlSafe(sql)).fetch_all(executor).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_quick_reply_set<'e, E>(executor: E, id: &str) -> Result<QuickReplySet, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let sql = format!("SELECT {COLUMNS} FROM quick_reply_sets WHERE id = ?");
    Ok(sqlx::query_as::<_, QuickReplySetRow>(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_one(executor)
        .await?
        .into())
}

/// 넣거나 고친다. 저장한 id를 돌려준다
pub async fn upsert_quick_reply_set<'e, E>(executor: E, input: &QuickReplySetInput) -> Result<String, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    let id = input.id.clone().filter(|s| !s.is_empty()).unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = chrono::Utc::now().timestamp_millis();
    let scope_id = if input.scope == "global" { None } else { input.scope_id.clone() };
    sqlx::query(
        "INSERT INTO quick_reply_sets (id, name, scope, scope_id, enabled, auto_send, items, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, scope = excluded.scope, scope_id = excluded.scope_id,
           enabled = excluded.enabled, auto_send = excluded.auto_send, items = excluded.items, sort_order = excluded.sort_order,
           updated_at = excluded.updated_at",
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.scope)
    .bind(scope_id)
    .bind(input.enabled)
    .bind(input.auto_send)
    .bind(serde_json::to_string(&input.items).unwrap_or_else(|_| "[]".into()))
    .bind(input.sort_order)
    .bind(now)
    .bind(now)
    .execute(executor)
    .await?;
    Ok(id)
}

pub async fn delete_quick_reply_set<'e, E>(executor: E, id: &str) -> Result<bool, Error>
where
    E: Executor<'e, Database = Sqlite>,
{
    Ok(sqlx::query("DELETE FROM quick_reply_sets WHERE id = ?")
        .bind(id)
        .execute(executor)
        .await?
        .rows_affected()
        > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::QuickReply;

    #[tokio::test]
    async fn sets_keep_scope_and_items_and_global_drops_scope_id() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::conn::init_pool(dir.path().to_path_buf()).await.unwrap();
        let item = QuickReply {
            id: "q".into(),
            label: "인사".into(),
            message: "안녕".into(),
            hidden: false,
        };
        let input = |scope: &str, scope_id: Option<&str>| QuickReplySetInput {
            id: None,
            name: format!("{scope} 세트"),
            scope: scope.into(),
            scope_id: scope_id.map(Into::into),
            enabled: true,
            auto_send: false,
            items: vec![item.clone()],
            sort_order: 0,
        };
        let g = upsert_quick_reply_set(&db, &input("global", Some("무시됨"))).await.unwrap();
        let c = upsert_quick_reply_set(&db, &input("character", Some("ch"))).await.unwrap();
        let all = get_all_quick_reply_sets(&db).await.unwrap();
        assert_eq!(all.len(), 2);
        let gs = all.iter().find(|s| s.id == g).unwrap();
        assert_eq!(gs.scope_id, None, "전역은 대상 없음");
        assert_eq!(gs.items[0].message, "안녕");
        assert_eq!(all.iter().find(|s| s.id == c).unwrap().scope_id.as_deref(), Some("ch"));

        // 고치기 (같은 id) → 한 줄 그대로, 지우면 지운 기록이 남음 (동기화)
        let mut again = input("preset", Some("p"));
        again.id = Some(c.clone());
        upsert_quick_reply_set(&db, &again).await.unwrap();
        assert_eq!(get_quick_reply_set(&db, &c).await.unwrap().scope, "preset");
        assert!(delete_quick_reply_set(&db, &c).await.unwrap());
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync_tombstones WHERE tbl = 'quick_reply_sets'")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(n, 1);
    }
}
