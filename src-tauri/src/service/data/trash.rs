//! 휴지통 (CLAUDE.md 2-A-3, ADR-011).
//!
//! 지우기 = 항목과 그 항목을 지우면 같이 지워지는 행(FK CASCADE)을 통째로 떠서 `trash_items`에 넣고, 실제로 지운다.
//! 목록 조회는 바꾸지 않는다(지운 행은 DB에 없으므로 어디서도 보이지 않음). 파일(채팅 이미지 폴더·게임 폴더·
//! 페르소나 아바타)은 `trash/<휴지통 id>/`로 옮긴다.
//!
//! - 되살리기: 뜬 행을 다시 넣고(updated_at은 지금 시각 — 다른 기기의 "지운 기록"보다 새것이 되게), 지울 때 끊긴
//!   연결(FK SET NULL)을 다시 잇고, 파일을 제자리로 옮긴다. 연결된 항목이 이미 없으면 되살리지 않고 이유를 알린다.
//! - 완전히 지우기: 휴지통 행과 파일만 지운다.
//! - 자동 정리: 보존 기간(설정 `trashRetentionDays`, 기본 30일)이 지난 것은 휴지통을 열거나 새로 지울 때 정리한다
//!   (시작할 때 따로 돌지 않음 — Lazy Initialization).
//!
//! 행 구조는 DB에서 읽는다(pragma). 테이블·FK가 늘어도 이 파일을 고칠 필요가 없다.
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{SqliteConnection, SqlitePool};

use crate::error::{AppError, Result};

pub const DEFAULT_RETENTION_DAYS: u32 = 30;
const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type, strum_macros::EnumString, strum_macros::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TrashKind {
    Character,
    Persona,
    Conversation,
    WorldinfoBook,
    PromptPreset,
    Game,
    GameSession,
}

impl TrashKind {
    fn table(self) -> &'static str {
        match self {
            Self::Character => "characters",
            Self::Persona => "personas",
            Self::Conversation => "conversations",
            Self::WorldinfoBook => "worldinfo_books",
            Self::PromptPreset => "prompt_presets",
            Self::Game => "game_projects",
            Self::GameSession => "game_player_sessions",
        }
    }
    /// phone_links는 FK가 아니라 트리거로 지워진다(캐릭터·페르소나)
    fn phone_kind(self) -> Option<&'static str> {
        match self {
            Self::Character => Some("character"),
            Self::Persona => Some("persona"),
            _ => None,
        }
    }
}

/// 휴지통 목록의 한 줄
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct TrashItem {
    pub id: String,
    pub kind: TrashKind,
    pub item_id: String,
    pub label: String,
    pub deleted_at: i64,
    /// 같이 지운 행 수 (항목 자신 포함)
    pub rows: u32,
    pub files: u32,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SnapRow {
    table: String,
    values: Map<String, Value>,
}

/// 지울 때 NULL로 바뀐 바깥 행의 연결 (되살릴 때 다시 잇는다)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Relink {
    table: String,
    key: Map<String, Value>,
    column: String,
    value: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Snapshot {
    version: u32,
    rows: Vec<SnapRow>,
    relinks: Vec<Relink>,
}

/// 앱 데이터 폴더 기준 상대 경로
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MovedFile {
    from: String,
    to: String,
}

// ---------------------------------------------------------------------------
// 스키마 읽기
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Fk {
    child: String,
    from: Vec<String>,
    to: Vec<String>,
    on_delete: String,
}

struct Schema {
    columns: HashMap<String, Vec<String>>,
    keys: HashMap<String, Vec<String>>,
    /// 부모 테이블 → 그 테이블을 가리키는 FK
    incoming: HashMap<String, Vec<Fk>>,
}

fn quote(id: &str) -> String {
    format!("\"{}\"", id.replace('"', "\"\""))
}

impl Schema {
    async fn load(conn: &mut SqliteConnection) -> Result<Self> {
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

    async fn select(&self, conn: &mut SqliteConnection, table: &str, cols: &[String], vals: &[Value]) -> Result<Vec<Map<String, Value>>> {
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

    fn key_of(&self, table: &str, row: &Map<String, Value>) -> Map<String, Value> {
        self.keys
            .get(table)
            .map(|k| k.iter().map(|c| (c.clone(), row.get(c).cloned().unwrap_or(Value::Null))).collect())
            .unwrap_or_default()
    }
}

fn bind_value<'q, O>(
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

fn bind_query<'q>(
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

// ---------------------------------------------------------------------------
// 뜨기
// ---------------------------------------------------------------------------

async fn snapshot(conn: &mut SqliteConnection, schema: &Schema, kind: TrashKind, id: &str) -> Result<Snapshot> {
    let root_table = kind.table();
    let root_key = schema.keys.get(root_table).cloned().unwrap_or_default();
    let roots = schema.select(conn, root_table, &root_key, &[Value::String(id.into())]).await?;
    let mut snap = Snapshot {
        version: SNAPSHOT_VERSION,
        ..Default::default()
    };
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut queue: VecDeque<(String, Map<String, Value>)> = VecDeque::new();
    for r in roots {
        seen.insert((root_table.to_string(), Value::Object(schema.key_of(root_table, &r)).to_string()));
        queue.push_back((root_table.to_string(), r));
    }
    let mut relinks = vec![];
    while let Some((table, row)) = queue.pop_front() {
        if let Some(fks) = schema.incoming.get(&table) {
            for fk in fks {
                let vals: Vec<Value> = fk.to.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
                if vals.iter().any(Value::is_null) {
                    continue;
                }
                let children = schema.select(conn, &fk.child, &fk.from, &vals).await?;
                for child in children {
                    let key = schema.key_of(&fk.child, &child);
                    match fk.on_delete.as_str() {
                        "CASCADE" => {
                            if seen.insert((fk.child.clone(), Value::Object(key).to_string())) {
                                queue.push_back((fk.child.clone(), child));
                            }
                        }
                        "SET NULL" => {
                            for (col, v) in fk.from.iter().zip(vals.iter()) {
                                relinks.push(Relink {
                                    table: fk.child.clone(),
                                    key: key.clone(),
                                    column: col.clone(),
                                    value: v.clone(),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        snap.rows.push(SnapRow { table, values: row });
    }
    // 같이 지워지는 행 안의 연결은 그 행을 되살리면 따라온다
    snap.relinks = relinks
        .into_iter()
        .filter(|r| !seen.contains(&(r.table.clone(), Value::Object(r.key.clone()).to_string())))
        .collect();

    if let Some(pk) = kind.phone_kind() {
        let sql = "SELECT json_object('owner_kind', owner_kind, 'owner_id', owner_id, 'contact_kind', contact_kind, 'contact_id', contact_id, 'status', status, 'sort_order', sort_order, 'created_at', created_at, 'updated_at', updated_at) FROM phone_links WHERE (owner_kind = ?1 AND owner_id = ?2) OR (contact_kind = ?1 AND contact_id = ?2)";
        let links: Vec<String> = sqlx::query_scalar(sql).bind(pk).bind(id).fetch_all(&mut *conn).await?;
        for l in links {
            if let Ok(Value::Object(values)) = serde_json::from_str(&l) {
                snap.rows.push(SnapRow {
                    table: "phone_links".into(),
                    values,
                });
            }
        }
    }
    Ok(snap)
}

fn label_of(kind: TrashKind, row: &Map<String, Value>, game_title: Option<String>) -> String {
    let s = |k: &str| row.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string();
    let l = match kind {
        TrashKind::Conversation => s("title"),
        TrashKind::Game => s("title"),
        TrashKind::GameSession => format!("{} 플레이 기록", game_title.unwrap_or_default()).trim().to_string(),
        _ => s("name"),
    };
    if l.is_empty() {
        "(이름 없음)".into()
    } else {
        l
    }
}

/// 같이 옮길 파일 (앱 데이터 폴더 기준 상대 경로)
fn owned_files(base: &Path, kind: TrashKind, id: &str, row: &Map<String, Value>) -> Vec<PathBuf> {
    if id.is_empty() || id.contains(['/', '\\', '.']) {
        return vec![]; // 경로 조작 방지
    }
    match kind {
        TrashKind::Character => vec![PathBuf::from("characters").join(id)],
        TrashKind::Game => vec![PathBuf::from("games").join(id)],
        TrashKind::Persona => {
            // 앱이 복사해 둔 아바타만 (사용자가 가진 원본 경로는 건드리지 않음)
            let dir = base.join("characters").join("avatars");
            row.get("avatar")
                .and_then(Value::as_str)
                .map(Path::new)
                .filter(|p| p.starts_with(&dir) && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("persona-")))
                .and_then(|p| p.strip_prefix(base).ok().map(Path::to_path_buf))
                .into_iter()
                .collect()
        }
        _ => vec![],
    }
}

fn rel(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

// ---------------------------------------------------------------------------
// 공개 함수
// ---------------------------------------------------------------------------

/// 지우기 = 휴지통으로. 항목이 없으면 NotFound
pub async fn move_to_trash(db: &SqlitePool, base: &Path, kind: TrashKind, id: &str) -> Result<TrashItem> {
    let mut tx = db.begin().await?;
    let schema = Schema::load(&mut tx).await?;
    let snap = snapshot(&mut tx, &schema, kind, id).await?;
    let Some(root) = snap.rows.first().filter(|r| r.table == kind.table()).map(|r| r.values.clone()) else {
        return Err(AppError::not_found(format!("지울 항목이 없음: {id}")));
    };
    let game_title: Option<String> = if kind == TrashKind::GameSession {
        sqlx::query_scalar("SELECT title FROM game_projects WHERE id = ?")
            .bind(root.get("game_id").and_then(Value::as_str).unwrap_or(""))
            .fetch_optional(&mut *tx)
            .await?
    } else {
        None
    };
    let label = label_of(kind, &root, game_title);
    let payload = serde_json::to_string(&snap).map_err(|e| AppError::internal(e.to_string()))?;
    let trash_id = uuid::Uuid::new_v4().to_string();
    let deleted_at = now_ms();

    let key = schema.keys.get(kind.table()).cloned().unwrap_or_default();
    let sql = format!(
        "DELETE FROM {} WHERE {} = ?",
        quote(kind.table()),
        quote(key.first().map(String::as_str).unwrap_or("id"))
    );
    sqlx::query(sqlx::AssertSqlSafe(sql)).bind(id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO trash_items (id, kind, item_id, label, payload, files, size, row_count, deleted_at) VALUES (?, ?, ?, ?, ?, '[]', ?, ?, ?)")
        .bind(&trash_id)
        .bind(kind.to_string())
        .bind(id)
        .bind(&label)
        .bind(&payload)
        .bind(payload.len() as i64)
        .bind(snap.rows.len() as i64)
        .bind(deleted_at)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    // 파일은 DB가 바뀐 뒤에 옮긴다. 못 옮긴 파일은 제자리에 둔다(되살리면 그대로 쓰임)
    let mut moved = vec![];
    let trash_dir = PathBuf::from("trash").join(&trash_id);
    for (i, from) in owned_files(base, kind, id, &root).into_iter().enumerate() {
        let src = base.join(&from);
        if !src.exists() {
            continue;
        }
        let to = trash_dir.join(format!(
            "{i}-{}",
            from.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        ));
        let dst = base.join(&to);
        let ok = dst.parent().map(std::fs::create_dir_all).transpose().is_ok() && std::fs::rename(&src, &dst).is_ok();
        if ok {
            moved.push(MovedFile {
                from: rel(&from),
                to: rel(&to),
            });
        } else {
            eprintln!("[trash] 휴지통으로 파일을 옮기지 못함: {}", src.display());
        }
    }
    if !moved.is_empty() {
        sqlx::query("UPDATE trash_items SET files = ? WHERE id = ?")
            .bind(serde_json::to_string(&moved).unwrap_or_else(|_| "[]".into()))
            .bind(&trash_id)
            .execute(db)
            .await?;
    }
    Ok(TrashItem {
        id: trash_id,
        kind,
        item_id: id.into(),
        label,
        deleted_at,
        rows: snap.rows.len() as u32,
        files: moved.len() as u32,
        size: payload.len() as i64,
    })
}

/// id, kind, item_id, label, files, size, row_count, deleted_at (payload는 되살릴 때만 읽는다)
type TrashRow = (String, String, String, String, String, i64, i64, i64);

fn to_item(r: &TrashRow) -> Option<TrashItem> {
    let files: Vec<MovedFile> = serde_json::from_str(&r.4).unwrap_or_default();
    Some(TrashItem {
        id: r.0.clone(),
        kind: r.1.parse().ok()?,
        item_id: r.2.clone(),
        label: r.3.clone(),
        deleted_at: r.7,
        rows: r.6 as u32,
        files: files.len() as u32,
        size: r.5,
    })
}

const SELECT_TRASH: &str = "SELECT id, kind, item_id, label, files, size, row_count, deleted_at FROM trash_items";

pub async fn list(db: &SqlitePool) -> Result<Vec<TrashItem>> {
    let rows: Vec<TrashRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT_TRASH} ORDER BY deleted_at DESC")))
        .fetch_all(db)
        .await?;
    Ok(rows.iter().filter_map(to_item).collect())
}

/// 되살리기. 지울 때 연결돼 있던 항목(예: 대화의 캐릭터)이 이미 없으면 되살리지 않고 이유를 알린다
pub async fn restore(db: &SqlitePool, base: &Path, trash_id: &str) -> Result<TrashItem> {
    let row: Option<TrashRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT_TRASH} WHERE id = ?")))
        .bind(trash_id)
        .fetch_optional(db)
        .await?;
    let row = row.ok_or_else(|| AppError::not_found("휴지통에 없는 항목"))?;
    let item = to_item(&row).ok_or_else(|| AppError::internal("휴지통 항목을 읽지 못함"))?;
    let payload: String = sqlx::query_scalar("SELECT payload FROM trash_items WHERE id = ?")
        .bind(trash_id)
        .fetch_one(db)
        .await?;
    let snap: Snapshot = serde_json::from_str(&payload).map_err(|e| AppError::internal(e.to_string()))?;
    let files: Vec<MovedFile> = serde_json::from_str(&row.4).unwrap_or_default();

    let mut tx = db.begin().await?;
    sqlx::query("PRAGMA defer_foreign_keys = ON").execute(&mut *tx).await?;
    let schema = Schema::load(&mut tx).await?;
    let now = now_ms();
    let mut tables: Vec<String> = vec![];
    for r in &snap.rows {
        let Some(cols) = schema.columns.get(&r.table) else { continue }; // 그 사이 없어진 테이블
        let use_cols: Vec<&String> = cols.iter().filter(|c| r.values.contains_key(c.as_str())).collect();
        if use_cols.is_empty() {
            continue;
        }
        let sql = format!(
            "INSERT OR IGNORE INTO {} ({}) VALUES ({})",
            quote(&r.table),
            use_cols.iter().map(|c| quote(c)).collect::<Vec<_>>().join(", "),
            vec!["?"; use_cols.len()].join(", ")
        );
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
        for c in &use_cols {
            // 되살린 행은 지금 고친 것으로 (다른 기기의 지운 기록보다 새것)
            let v = if c.as_str() == "updated_at" {
                Value::from(now)
            } else {
                r.values[c.as_str()].clone()
            };
            q = bind_query(q, &v);
        }
        q.execute(&mut *tx).await?;
        if !tables.contains(&r.table) {
            tables.push(r.table.clone());
        }
    }
    for l in &snap.relinks {
        if !schema.columns.get(&l.table).is_some_and(|c| c.contains(&l.column)) || l.key.is_empty() {
            continue;
        }
        let cond = l.key.keys().map(|k| format!("{} = ?", quote(k))).collect::<Vec<_>>().join(" AND ");
        let sql = format!(
            "UPDATE {} SET {} = ? WHERE {cond} AND {} IS NULL",
            quote(&l.table),
            quote(&l.column),
            quote(&l.column)
        );
        let mut q = bind_query(sqlx::query(sqlx::AssertSqlSafe(sql)), &l.value);
        for v in l.key.values() {
            q = bind_query(q, v);
        }
        q.execute(&mut *tx).await?;
    }
    // 끊긴 연결이 있으면 되살리지 않는다 (예: 대화를 되살리려는데 그 사이 캐릭터를 완전히 지움)
    let mut missing: Vec<String> = vec![];
    for t in &tables {
        let parents: Vec<String> = sqlx::query_scalar("SELECT DISTINCT parent FROM pragma_foreign_key_check(?)")
            .bind(t)
            .fetch_all(&mut *tx)
            .await?;
        missing.extend(parents);
    }
    if !missing.is_empty() {
        missing.sort();
        missing.dedup();
        tx.rollback().await?;
        return Err(AppError::validation(
            format!("연결된 항목이 없어 되살릴 수 없음 ({}) — 먼저 그 항목을 되살리세요", missing.join(", ")),
            None,
        ));
    }
    sqlx::query("DELETE FROM trash_items WHERE id = ?").bind(trash_id).execute(&mut *tx).await?;
    tx.commit().await?;

    for f in files {
        let (src, dst) = (base.join(&f.to), base.join(&f.from));
        if dst.exists() {
            eprintln!("[trash] 되살릴 자리에 이미 파일이 있어 휴지통 사본을 둠: {}", dst.display());
            continue;
        }
        if dst.parent().map(std::fs::create_dir_all).transpose().is_err() || std::fs::rename(&src, &dst).is_err() {
            eprintln!("[trash] 휴지통 파일을 되돌리지 못함: {}", src.display());
        }
    }
    remove_trash_dir(base, trash_id);
    Ok(item)
}

fn remove_trash_dir(base: &Path, trash_id: &str) {
    if trash_id.is_empty() || trash_id.contains(['/', '\\', '.']) {
        return;
    }
    let dir = base.join("trash").join(trash_id);
    if dir.exists() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// 완전히 지우기 (휴지통 항목과 파일)
pub async fn purge(db: &SqlitePool, base: &Path, trash_id: &str) -> Result<bool> {
    let n = sqlx::query("DELETE FROM trash_items WHERE id = ?")
        .bind(trash_id)
        .execute(db)
        .await?
        .rows_affected();
    remove_trash_dir(base, trash_id);
    Ok(n > 0)
}

/// 휴지통 비우기. 지운 개수
pub async fn purge_all(db: &SqlitePool, base: &Path) -> Result<u32> {
    purge_where(db, base, i64::MAX).await
}

/// 보존 기간이 지난 것 정리. None = 계속 보관
pub async fn purge_expired(db: &SqlitePool, base: &Path, retention_days: Option<u32>) -> Result<u32> {
    let Some(days) = retention_days else { return Ok(0) };
    purge_where(db, base, now_ms() - i64::from(days) * 86_400_000).await
}

async fn purge_where(db: &SqlitePool, base: &Path, before: i64) -> Result<u32> {
    let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM trash_items WHERE deleted_at < ?")
        .bind(before)
        .fetch_all(db)
        .await?;
    for id in &ids {
        purge(db, base, id).await?;
    }
    Ok(ids.len() as u32)
}
