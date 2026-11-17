//! DB 구조 읽기 (pragma): 테이블의 열·키·FK. 휴지통(`service/data/trash.rs`)과 동기화 묶음(`service/data/sync_bundle.rs`)이 같이 쓴다.
//! 구조를 DB에서 읽으므로 테이블이 늘어도 고칠 곳이 없다.
use std::collections::HashMap;

use serde_json::{Map, Value};
use sqlx::SqliteConnection;

use crate::error::{AppError, Result};

#[derive(Debug, Clone)]
pub struct Fk {
    pub child: String,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub on_delete: String,
}

pub struct Schema {
    pub columns: HashMap<String, Vec<String>>,
    /// 기본 키, 없으면 첫 UNIQUE 묶음
    pub keys: HashMap<String, Vec<String>>,
    /// 부모 테이블 → 그 테이블을 가리키는 FK
    pub incoming: HashMap<String, Vec<Fk>>,
}

pub fn quote(id: &str) -> String {
    format!("\"{}\"", id.replace('"', "\"\""))
}

impl Schema {
    pub async fn load(conn: &mut SqliteConnection) -> Result<Self> {
        let tables: Vec<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '\\_sqlx%' ESCAPE '\\'")
                .fetch_all(&mut *conn)
                .await?;
        let mut columns = HashMap::new();
        let mut keys = HashMap::new();
        let mut incoming: HashMap<String, Vec<Fk>> = HashMap::new();
        for t in &tables {
            let cols: Vec<(String, i64)> = sqlx::query_as("SELECT name, pk FROM pragma_table_info(?) ORDER BY cid")
                .bind(t)
                .fetch_all(&mut *conn)
                .await?;
            let mut pk: Vec<(i64, String)> = cols.iter().filter(|c| c.1 > 0).map(|c| (c.1, c.0.clone())).collect();
            pk.sort();
            let mut key: Vec<String> = pk.into_iter().map(|p| p.1).collect();
            if key.is_empty() {
                // 기본 키가 없으면 UNIQUE 묶음(게임 일지·세이브 등)
                let idx: Option<String> = sqlx::query_scalar("SELECT name FROM pragma_index_list(?) WHERE \"unique\" = 1 ORDER BY seq LIMIT 1")
                    .bind(t)
                    .fetch_optional(&mut *conn)
                    .await?;
                if let Some(idx) = idx {
                    key = sqlx::query_scalar("SELECT name FROM pragma_index_info(?) ORDER BY seqno")
                        .bind(&idx)
                        .fetch_all(&mut *conn)
                        .await?;
                }
            }
            columns.insert(t.clone(), cols.into_iter().map(|c| c.0).collect());
            keys.insert(t.clone(), key);
        }
        for t in &tables {
            let fks: Vec<(i64, String, String, Option<String>, String)> =
                sqlx::query_as("SELECT id, \"table\", \"from\", \"to\", on_delete FROM pragma_foreign_key_list(?) ORDER BY id, seq")
                    .bind(t)
                    .fetch_all(&mut *conn)
                    .await?;
            let mut grouped: Vec<(i64, Fk, String)> = vec![];
            for (id, parent, from, to, on_delete) in fks {
                match grouped.last_mut() {
                    Some(g) if g.0 == id => {
                        g.1.from.push(from);
                        g.1.to.extend(to);
                    }
                    _ => grouped.push((
                        id,
                        Fk {
                            child: t.clone(),
                            from: vec![from],
                            to: to.into_iter().collect(),
                            on_delete: on_delete.to_uppercase(),
                        },
                        parent,
                    )),
                }
            }
            for (_, mut fk, parent) in grouped {
                if fk.to.len() != fk.from.len() {
                    // "REFERENCES parent" 처럼 열을 안 적으면 부모의 기본 키
                    fk.to = keys.get(&parent).cloned().unwrap_or_default();
                }
                incoming.entry(parent).or_default().push(fk);
            }
        }
        Ok(Self { columns, keys, incoming })
    }

    /// `cols = vals`인 행들을 {열: 값} 객체로
    pub async fn select(&self, conn: &mut SqliteConnection, table: &str, cols: &[String], vals: &[Value]) -> Result<Vec<Map<String, Value>>> {
        let Some(all) = self.columns.get(table) else { return Ok(vec![]) };
        let obj = all
            .iter()
            .map(|c| format!("'{}', {}", c.replace('\'', "''"), quote(c)))
            .collect::<Vec<_>>()
            .join(", ");
        let cond = cols.iter().map(|c| format!("{} = ?", quote(c))).collect::<Vec<_>>().join(" AND ");
        let sql = format!("SELECT json_object({obj}) FROM {} WHERE {cond}", quote(table));
        let mut q = sqlx::query_scalar::<_, String>(sqlx::AssertSqlSafe(sql));
        for v in vals {
            q = bind_value(q, v);
        }
        let rows = q.fetch_all(&mut *conn).await?;
        rows.into_iter()
            .map(|r| match serde_json::from_str::<Value>(&r) {
                Ok(Value::Object(m)) => Ok(m),
                _ => Err(AppError::internal(format!("{table} 행을 읽지 못함"))),
            })
            .collect()
    }

    pub fn key_of(&self, table: &str, row: &Map<String, Value>) -> Map<String, Value> {
        self.keys
            .get(table)
            .map(|k| k.iter().map(|c| (c.clone(), row.get(c).cloned().unwrap_or(Value::Null))).collect())
            .unwrap_or_default()
    }
}

pub fn bind_value<'q, O>(
    q: sqlx::query::QueryScalar<'q, sqlx::Sqlite, O, sqlx::sqlite::SqliteArguments>,
    v: &Value,
) -> sqlx::query::QueryScalar<'q, sqlx::Sqlite, O, sqlx::sqlite::SqliteArguments> {
    match v {
        Value::Null => q.bind(None::<String>),
        Value::Bool(b) => q.bind(*b as i64),
        Value::Number(n) if n.is_i64() => q.bind(n.as_i64()),
        Value::Number(n) => q.bind(n.as_f64()),
        Value::String(s) => q.bind(s.clone()),
        other => q.bind(other.to_string()),
    }
}

pub fn bind_query<'q>(
    q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments>,
    v: &Value,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments> {
    match v {
        Value::Null => q.bind(None::<String>),
        Value::Bool(b) => q.bind(*b as i64),
        Value::Number(n) if n.is_i64() => q.bind(n.as_i64()),
        Value::Number(n) => q.bind(n.as_f64()),
        Value::String(s) => q.bind(s.clone()),
        other => q.bind(other.to_string()),
    }
}
