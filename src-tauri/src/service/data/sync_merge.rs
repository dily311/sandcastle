//! 동기화 병합 (CLAUDE.md 2-B-2, ADR-011).
//!
//! 받은 묶음(2-B-1)을 이 기기 DB에 행 단위로 합친다. 같은 묶음을 미리보기(`dry_run`)로 돌리면 모든 변경을 한
//! 트랜잭션 안에서 해 보고 되돌린다 — 화면(2-B-3)이 "추가 n · 변경 n · 충돌 n"을 먼저 보여준다.
//!
//! **시계를 섞지 않는다**: 기기마다 시계가 다를 수 있어, 받은 행의 시각은 보낸 기기의 시각(`sync_peers.last_bundle_created_at`)
//! 하고만, 이 기기 행의 시각은 이 기기 시각(`last_merge_at`)하고만 비교한다. 받아서 쓴 행의 `updated_at`은 이 기기의
//! 지금 시각으로 둔다(이 DB의 모든 시각이 한 시계).
//!
//! 규칙 (ADR-011 ②③)
//! - 받은 행이 그 기기에서 마지막으로 합친 뒤에 바뀌지 않았으면 → 건너뜀
//! - 이 기기에 없음 → 넣음 (이 기기에서 지운 것이면 "고친 쪽을 살린다" → 되살림)
//! - 내용이 같음 → 그대로
//! - 이 기기는 그 뒤로 안 고침 → 받은 것으로 바꿈
//! - 양쪽 다 고침 → **둘 다 남김**: 이 기기 것을 두고 받은 행을 `sync_conflicts`에
//! - 같은 대화에 양쪽이 이어 씀(메시지 번호가 겹침) → 받은 메시지를 **분기 대화**로
//! - 받은 지운 기록: 이 기기가 그 뒤로 고쳤으면 고친 쪽을 살리고, 아니면 지운다(CASCADE)
//! - 같이 쓰는 설정 칸은 받은 것으로 (기기 전용 칸은 그대로 — settings_scope)
//!
//! 넣지 못한 받은 행(다른 UNIQUE 규칙, 이 기기에 없는 항목을 가리킴)은 버리지 않고 `sync_conflicts`에 남긴다.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{SqliteConnection, SqlitePool};

use super::settings_scope;
use super::sync_bundle::{Bundle, SyncManifest, Tombstone};
use crate::db::{
    change_tracking,
    schema::{bind_query, quote, Schema},
};
use crate::error::{AppError, Result};

/// 묶음에서 읽은 내용 (zip을 연 채로 두지 않게 한 번에 읽는다)
#[derive(Debug, Clone)]
pub struct BundleData {
    pub manifest: SyncManifest,
    pub tables: BTreeMap<String, Vec<Map<String, Value>>>,
    pub tombstones: Vec<Tombstone>,
    pub settings: Map<String, Value>,
}

impl BundleData {
    /// 이 기기가 동기화하는 테이블만 읽는다
    pub async fn read(bundle: &mut Bundle, db: &SqlitePool) -> Result<Self> {
        let synced: HashSet<String> = change_tracking::synced_tables(db).await?.into_iter().collect();
        let mut tables = BTreeMap::new();
        for t in bundle.manifest.tables.keys().cloned().collect::<Vec<_>>() {
            if synced.contains(&t) {
                tables.insert(t.clone(), bundle.rows(&t)?);
            }
        }
        Ok(Self {
            manifest: bundle.manifest.clone(),
            tables,
            tombstones: bundle.tombstones()?,
            settings: bundle.settings()?,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MergeReport {
    pub added: u32,
    pub updated: u32,
    pub unchanged: u32,
    pub deleted: u32,
    /// 받은 지우기를 이 기기에서 고친 것이 이겨서 지우지 않음
    pub kept_local_edits: u32,
    /// 이 기기에서 지운 것을 받은 고침이 되살림
    pub restored: u32,
    /// 둘 다 남긴 것 (sync_conflicts, 분기 제외)
    pub conflicts: u32,
    /// 양쪽이 이어 써서 만든 분기 대화
    pub branches: u32,
    /// 이 기기가 모르는 테이블·열 등으로 넘긴 것
    pub skipped: u32,
    pub settings_applied: bool,
    pub dry_run: bool,
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn key_text(vals: &[Value]) -> String {
    serde_json::to_string(vals).unwrap_or_default()
}

fn same_content(local: &Map<String, Value>, remote: &Map<String, Value>) -> bool {
    // 시각 칸은 비교하지 않는다 (처음 만든 기본 페르소나·프리셋은 기기마다 만든 시각만 다름)
    remote
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "updated_at" | "created_at"))
        .all(|(k, v)| match local.get(k) {
            Some(l) => l == v || (l.as_f64().is_some() && l.as_f64() == v.as_f64()),
            None => true, // 이 기기에 없는 열은 비교하지 않는다
        })
}

/// 부모 테이블이 먼저 오게 (FK 순서). 순환은 남은 순서대로
fn table_order(schema: &Schema, tables: &[String]) -> Vec<String> {
    let set: HashSet<&String> = tables.iter().collect();
    let mut parents: HashMap<&String, HashSet<String>> = HashMap::new();
    for (parent, fks) in &schema.incoming {
        for fk in fks {
            if fk.child != *parent && set.contains(&fk.child) && set.contains(parent) {
                parents
                    .entry(tables.iter().find(|t| **t == fk.child).unwrap())
                    .or_default()
                    .insert(parent.clone());
            }
        }
    }
    let mut done: Vec<String> = vec![];
    let mut left: Vec<String> = tables.to_vec();
    while !left.is_empty() {
        let ready: Vec<String> = left
            .iter()
            .filter(|t| parents.get(t).is_none_or(|ps| ps.iter().all(|p| done.contains(p))))
            .cloned()
            .collect();
        let next = if ready.is_empty() { vec![left[0].clone()] } else { ready };
        left.retain(|t| !next.contains(t));
        done.extend(next);
    }
    done
}

/// 한 열짜리 FK 하나: 이 열이 부모 테이블의 행을 가리킨다
#[derive(Debug, Clone)]
struct OutFk {
    col: String,
    parent: String,
    cascade: bool,
}

/// 테이블 → 그 테이블이 가리키는 FK (한 열짜리만 — id 바꾸기·같이 살리기용)
fn outgoing(schema: &Schema) -> HashMap<String, Vec<OutFk>> {
    let mut out: HashMap<String, Vec<OutFk>> = HashMap::new();
    for (parent, fks) in &schema.incoming {
        for fk in fks.iter().filter(|f| f.from.len() == 1) {
            out.entry(fk.child.clone()).or_default().push(OutFk {
                col: fk.from[0].clone(),
                parent: parent.clone(),
                cascade: fk.on_delete == "CASCADE",
            });
        }
    }
    out
}

/// 이 행이 CASCADE로 매달린 부모 중 하나가 `set`에 있나 ((테이블, 키 글자))
fn cascade_parent_in(fks: &[OutFk], row: &Map<String, Value>, set: &HashSet<(String, String)>) -> bool {
    fks.iter().filter(|f| f.cascade).any(|f| {
        row.get(&f.col)
            .filter(|v| !v.is_null())
            .is_some_and(|v| set.contains(&(f.parent.clone(), key_text(std::slice::from_ref(v)))))
    })
}

struct Ctx<'a> {
    schema: &'a Schema,
    out: HashMap<String, Vec<OutFk>>,
    device: String,
    now: i64,
    /// 그 기기에서 이미 본 시각 (그 기기 시계)
    remote_seen: i64,
    /// 이 기기가 마지막으로 합친 시각 (이 기기 시계)
    local_base: i64,
    /// 분기로 옮기며 바꾼 id: (테이블, 옛 id) → 새 id
    id_map: HashMap<(String, String), String>,
    /// 이번에 되살린 행 (자식도 같이 살린다)
    restored: HashSet<(String, String)>,
    /// 보낸 기기가 이 기기 것 대신 자기 것을 둔 행 (manifest.held) — 받는 쪽도 둘 다 남긴다
    held: HashSet<(String, String)>,
    /// 보낸 기기가 합친 다른 기기 묶음의 시각 (manifest.acks, 그 기기들 시계)
    acks: BTreeMap<String, i64>,
    report: MergeReport,
}

async fn local_row(conn: &mut SqliteConnection, schema: &Schema, table: &str, key_cols: &[String], key: &[Value]) -> Result<Option<Map<String, Value>>> {
    Ok(schema.select(conn, table, key_cols, key).await?.into_iter().next())
}

/// 이 기기 행이 다른 기기(C)에게서 받은 그대로이고(그 뒤 안 고침), 보낸 기기가 C의 그 판을 이미 봤는가.
/// 그렇다면 이 행은 "이 기기에서 고친 것"이 아니다 — 기기가 셋 이상일 때 같은 고침을 충돌로 보지 않게
async fn seen_by_sender(conn: &mut SqliteConnection, ctx: &Ctx<'_>, table: &str, key: &[Value], local_at: i64) -> Result<bool> {
    let origin: Option<(String, i64, i64)> = sqlx::query_as("SELECT device_id, remote_at, merged_at FROM sync_row_origin WHERE tbl = ? AND key = ?")
        .bind(table)
        .bind(key_text(key))
        .fetch_optional(&mut *conn)
        .await?;
    Ok(origin.is_some_and(|(dev, remote_at, merged_at)| merged_at == local_at && ctx.acks.get(&dev).is_some_and(|a| *a >= remote_at)))
}

async fn has_tombstone(conn: &mut SqliteConnection, table: &str, key: &[Value]) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sync_tombstones WHERE tbl = ? AND key = ?")
        .bind(table)
        .bind(key_text(key))
        .fetch_one(&mut *conn)
        .await?
        > 0)
}

async fn record_conflict(
    conn: &mut SqliteConnection,
    ctx: &Ctx<'_>,
    kind: &str,
    table: &str,
    key: &str,
    local: Option<&Map<String, Value>>,
    remote: Option<Value>,
) -> Result<()> {
    sqlx::query("INSERT INTO sync_conflicts (id, kind, tbl, key, local_row, remote_row, device_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(kind)
        .bind(table)
        .bind(key)
        .bind(local.map(|l| Value::Object(l.clone()).to_string()))
        .bind(remote.map(|r| r.to_string()))
        .bind(&ctx.device)
        .bind(ctx.now)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// 받은 행 넣기·바꾸기 (`INSERT … ON CONFLICT(키) DO UPDATE` — REPLACE처럼 지웠다 넣지 않아 자식 행이 CASCADE로 사라지지 않는다).
/// 다른 UNIQUE 규칙에 걸리면 Ok(false)
async fn upsert(conn: &mut SqliteConnection, ctx: &Ctx<'_>, table: &str, key_cols: &[String], row: &Map<String, Value>) -> Result<bool> {
    let Some(cols) = ctx.schema.columns.get(table) else { return Ok(false) };
    let use_cols: Vec<&String> = cols.iter().filter(|c| row.contains_key(c.as_str())).collect();
    let updates: Vec<String> = use_cols
        .iter()
        .filter(|c| !key_cols.contains(c))
        .map(|c| format!("{q} = excluded.{q}", q = quote(c)))
        .collect();
    let conflict = if updates.is_empty() {
        "DO NOTHING".to_string()
    } else {
        format!("DO UPDATE SET {}", updates.join(", "))
    };
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT({}) {conflict}",
        quote(table),
        use_cols.iter().map(|c| quote(c)).collect::<Vec<_>>().join(", "),
        vec!["?"; use_cols.len()].join(", "),
        key_cols.iter().map(|c| quote(c)).collect::<Vec<_>>().join(", "),
    );
    let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
    for c in &use_cols {
        // 이 DB의 시각은 이 기기 시계로
        let v = if c.as_str() == "updated_at" {
            Value::from(ctx.now)
        } else {
            row[c.as_str()].clone()
        };
        q = bind_query(q, &v);
    }
    match q.execute(&mut *conn).await {
        Ok(_) => {
            // 어느 기기의 어느 판을 받았나 (seen_by_sender)
            let key: Vec<Value> = key_cols.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
            let remote_at = row.get("updated_at").and_then(Value::as_i64).unwrap_or(0);
            sqlx::query("INSERT OR REPLACE INTO sync_row_origin (tbl, key, device_id, remote_at, merged_at) VALUES (?, ?, ?, ?, ?)")
                .bind(table)
                .bind(key_text(&key))
                .bind(&ctx.device)
                .bind(remote_at)
                .bind(ctx.now)
                .execute(&mut *conn)
                .await?;
            Ok(true)
        }
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// 지울 후보: (테이블, 키 열, 키 값, 이 기기의 행)
type PendingDelete = (String, Vec<String>, Vec<Value>, Map<String, Value>);

/// 받은 지운 기록 적용. 이 기기에서 그 뒤로 고친 행은 살리고(고친 쪽을 살린다), 살린 행에 CASCADE로 매달린 행도 살린다
/// (대화를 고쳤는데 그 메시지만 지워지면 안 되므로)
async fn apply_tombstones(conn: &mut SqliteConnection, ctx: &mut Ctx<'_>, tombstones: &[Tombstone]) -> Result<()> {
    let mut kept: HashSet<(String, String)> = HashSet::new();
    let mut pending: Vec<PendingDelete> = vec![];
    for t in tombstones {
        if t.deleted_at <= ctx.remote_seen {
            continue; // 이미 본 것
        }
        let Some(key_cols) = ctx.schema.keys.get(&t.tbl).filter(|k| !k.is_empty()).cloned() else {
            ctx.report.skipped += 1;
            continue;
        };
        let key: Vec<Value> = serde_json::from_str(&t.key).unwrap_or_default();
        if key.len() != key_cols.len() {
            ctx.report.skipped += 1;
            continue;
        }
        let Some(local) = local_row(conn, ctx.schema, &t.tbl, &key_cols, &key).await? else {
            continue;
        };
        let local_at = local.get("updated_at").and_then(Value::as_i64).unwrap_or(i64::MAX);
        if local_at > ctx.local_base && !seen_by_sender(conn, ctx, &t.tbl, &key, local_at).await? {
            ctx.report.kept_local_edits += 1;
            kept.insert((t.tbl.clone(), key_text(&key)));
        } else {
            pending.push((t.tbl.clone(), key_cols, key, local));
        }
    }
    // 살린 부모에 매달린 행은 지우지 않는다 (부모의 부모까지 따라가도록 바뀌지 않을 때까지)
    loop {
        let before = pending.len();
        let mut rest = vec![];
        for p in pending {
            let fks = ctx.out.get(&p.0).cloned().unwrap_or_default();
            if cascade_parent_in(&fks, &p.3, &kept) {
                kept.insert((p.0.clone(), key_text(&p.2)));
                ctx.report.kept_local_edits += 1;
            } else {
                rest.push(p);
            }
        }
        pending = rest;
        if pending.len() == before {
            break;
        }
    }
    for (tbl, key_cols, key, _) in pending {
        let cond = key_cols.iter().map(|c| format!("{} = ?", quote(c))).collect::<Vec<_>>().join(" AND ");
        let mut q = sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {} WHERE {cond}", quote(&tbl))));
        for v in &key {
            q = bind_query(q, v);
        }
        if q.execute(&mut *conn).await?.rows_affected() > 0 {
            ctx.report.deleted += 1;
        }
    }
    Ok(())
}

/// 같은 대화에 양쪽이 이어 쓴 곳 찾기: (대화 id → 처음 겹친 번호)
/// 그 기기에서 이미 본 행(remote_seen 이전)은 빼고 본다 — 같은 내용을 다시 받아도 분기를 또 만들지 않게
async fn diverged_conversations(conn: &mut SqliteConnection, rows: &[Map<String, Value>], remote_seen: i64) -> Result<BTreeMap<String, i64>> {
    let mut out: BTreeMap<String, i64> = BTreeMap::new();
    for r in rows {
        let (Some(id), Some(conv), Some(seq)) = (
            r.get("id").and_then(Value::as_str),
            r.get("conversation_id").and_then(Value::as_str),
            r.get("seq").and_then(Value::as_i64),
        ) else {
            continue;
        };
        if r.get("updated_at").and_then(Value::as_i64).is_some_and(|t| t <= remote_seen) {
            continue;
        }
        let taken: Option<String> = sqlx::query_scalar("SELECT id FROM messages WHERE conversation_id = ? AND seq = ?")
            .bind(conv)
            .bind(seq)
            .fetch_optional(&mut *conn)
            .await?;
        if taken.is_some_and(|t| t != id) {
            let e = out.entry(conv.to_string()).or_insert(seq);
            *e = (*e).min(seq);
        }
    }
    Ok(out)
}

/// 분기 대화 만들기: 겹치기 전까지의 이 기기 메시지를 복사한 새 대화. 새 대화 id
async fn make_branch(conn: &mut SqliteConnection, ctx: &Ctx<'_>, conv: &str, from_seq: i64) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let branch = serde_json::json!({ "parentId": conv, "seq": from_seq - 1, "fromDevice": ctx.device }).to_string();
    sqlx::query(
        "INSERT INTO conversations (id,title,character_id,persona_id,prompt_preset_id,world_info_ids,mode,meta,pinned,images_enabled,folder_id,api_profile_id,created_at,updated_at)
         SELECT ?, COALESCE(title, '') || ' · 다른 기기', character_id,persona_id,prompt_preset_id,world_info_ids,mode,
                json_set(COALESCE(meta, '{}'), '$.branch', json(?)), 0, images_enabled,folder_id,api_profile_id, ?, ?
         FROM conversations WHERE id = ?",
    )
    .bind(&id)
    .bind(&branch)
    .bind(ctx.now)
    .bind(ctx.now)
    .bind(conv)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "INSERT INTO messages (id,conversation_id,seq,role,name,content,character_id,meta,created_at,updated_at)
         SELECT lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' || substr(lower(hex(randomblob(2))),2) || '-' ||
                substr('89ab', 1 + (abs(random()) % 4), 1) || substr(lower(hex(randomblob(2))),2) || '-' || lower(hex(randomblob(6))),
                ?, seq, role, name, content, character_id, meta, created_at, ?
         FROM messages WHERE conversation_id = ? AND seq < ? ORDER BY seq",
    )
    .bind(&id)
    .bind(ctx.now)
    .bind(conv)
    .bind(from_seq)
    .execute(&mut *conn)
    .await?;
    Ok(id)
}

async fn apply_rows(conn: &mut SqliteConnection, ctx: &mut Ctx<'_>, table: &str, rows: &[Map<String, Value>]) -> Result<()> {
    let Some(key_cols) = ctx.schema.keys.get(table).filter(|k| !k.is_empty()).cloned() else {
        ctx.report.skipped += rows.len() as u32;
        return Ok(());
    };
    let fks = ctx.out.get(table).cloned().unwrap_or_default();

    // 같은 대화에 양쪽이 이어 씀 → 받은 메시지는 분기 대화로
    let mut branch_of: HashMap<String, (String, i64)> = HashMap::new();
    if table == "messages" {
        for (conv, from_seq) in diverged_conversations(conn, rows, ctx.remote_seen).await? {
            let b = make_branch(conn, ctx, &conv, from_seq).await?;
            let key = key_text(&[Value::from(conv.clone())]);
            record_conflict(
                conn,
                ctx,
                "branch",
                "conversations",
                &key,
                None,
                Some(serde_json::json!({ "branchId": b, "fromSeq": from_seq })),
            )
            .await?;
            ctx.report.branches += 1;
            branch_of.insert(conv, (b, from_seq));
        }
    }

    for original in rows {
        let mut row = original.clone();
        // 분기로 옮긴 행을 가리키면 새 id로
        for f in &fks {
            if let Some(Value::String(v)) = row.get(&f.col) {
                if let Some(n) = ctx.id_map.get(&(f.parent.clone(), v.clone())) {
                    row.insert(f.col.clone(), Value::String(n.clone()));
                }
            }
        }
        let key: Vec<Value> = key_cols.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
        if key.iter().any(Value::is_null) {
            ctx.report.skipped += 1;
            continue;
        }
        let remote_at = row.get("updated_at").and_then(Value::as_i64).unwrap_or(i64::MAX);
        if remote_at <= ctx.remote_seen {
            // 그 기기에서 이미 받은 그대로. 단 이번에 되살린 부모에 매달린 행이고 이 기기에서 지운 것이면 같이 살린다
            let revive = cascade_parent_in(&fks, &row, &ctx.restored)
                && local_row(conn, ctx.schema, table, &key_cols, &key).await?.is_none()
                && has_tombstone(conn, table, &key).await?;
            if !revive {
                ctx.report.unchanged += 1;
                continue;
            }
        }
        let local = local_row(conn, ctx.schema, table, &key_cols, &key).await?;

        // 분기로 갈 메시지: 이 기기에 없는 메시지 중 겹친 번호부터
        if let (None, Some(conv)) = (&local, row.get("conversation_id").and_then(Value::as_str).map(String::from)) {
            if let Some((b, from_seq)) = branch_of.get(&conv) {
                if row.get("seq").and_then(Value::as_i64).is_some_and(|s| s >= *from_seq) {
                    let new_id = uuid::Uuid::new_v4().to_string();
                    if let Some(Value::String(old)) = row.get("id") {
                        ctx.id_map.insert((table.to_string(), old.clone()), new_id.clone());
                    }
                    row.insert("id".into(), Value::String(new_id));
                    row.insert("conversation_id".into(), Value::String(b.clone()));
                    let new_key = vec![row["id"].clone()];
                    if upsert(conn, ctx, table, &key_cols, &row).await? {
                        ctx.report.added += 1;
                    } else {
                        record_conflict(conn, ctx, "unique", table, &key_text(&new_key), None, Some(Value::Object(original.clone()))).await?;
                        ctx.report.conflicts += 1;
                    }
                    continue;
                }
            }
        }

        match local {
            None => {
                let restored = has_tombstone(conn, table, &key).await?;
                if upsert(conn, ctx, table, &key_cols, &row).await? {
                    if restored {
                        ctx.report.restored += 1; // 고친 쪽을 살린다
                        ctx.restored.insert((table.to_string(), key_text(&key)));
                    } else {
                        ctx.report.added += 1;
                    }
                } else {
                    record_conflict(conn, ctx, "unique", table, &key_text(&key), None, Some(Value::Object(original.clone()))).await?;
                    ctx.report.conflicts += 1;
                }
            }
            Some(l) if same_content(&l, &row) => ctx.report.unchanged += 1,
            Some(l) => {
                let local_at = l.get("updated_at").and_then(Value::as_i64).unwrap_or(i64::MAX);
                let held = ctx.held.contains(&(table.to_string(), key_text(&key)));
                if (local_at <= ctx.local_base || seen_by_sender(conn, ctx, table, &key, local_at).await?) && !held {
                    if upsert(conn, ctx, table, &key_cols, &row).await? {
                        ctx.report.updated += 1;
                    } else {
                        record_conflict(conn, ctx, "unique", table, &key_text(&key), Some(&l), Some(Value::Object(original.clone()))).await?;
                        ctx.report.conflicts += 1;
                    }
                } else {
                    // 양쪽 다 고침 → 둘 다 남김 (이 기기 것을 두고 받은 것은 충돌 목록에)
                    record_conflict(conn, ctx, "edit", table, &key_text(&key), Some(&l), Some(Value::Object(original.clone()))).await?;
                    ctx.report.conflicts += 1;
                }
            }
        }
    }
    Ok(())
}

/// 넣은 행이 이 기기에 없는 항목을 가리키면: 비워도 되는 연결(SET NULL)은 비우고, 아니면 충돌로 남기고 뺀다.
/// 빼면서 생긴 지운 기록은 지운다(다른 기기에 "지웠다"고 전하면 안 되므로)
async fn fix_orphans(conn: &mut SqliteConnection, ctx: &mut Ctx<'_>, tables: &[String]) -> Result<()> {
    for _ in 0..5 {
        let started = now_ms() - 1;
        let before: HashSet<(String, String)> = sqlx::query_as::<_, (String, String)>("SELECT tbl, key FROM sync_tombstones WHERE deleted_at >= ?")
            .bind(started)
            .fetch_all(&mut *conn)
            .await?
            .into_iter()
            .collect();
        let mut found = 0;
        for t in tables {
            let bad: Vec<(i64, i64)> = sqlx::query_as("SELECT rowid, fkid FROM pragma_foreign_key_check(?)")
                .bind(t)
                .fetch_all(&mut *conn)
                .await?;
            for (rowid, fkid) in bad {
                found += 1;
                let fk: Vec<(String, String)> = sqlx::query_as("SELECT \"from\", on_delete FROM pragma_foreign_key_list(?) WHERE id = ? ORDER BY seq")
                    .bind(t)
                    .bind(fkid)
                    .fetch_all(&mut *conn)
                    .await?;
                if fk.iter().all(|(_, a)| a.eq_ignore_ascii_case("SET NULL")) {
                    let sets = fk.iter().map(|(c, _)| format!("{} = NULL", quote(c))).collect::<Vec<_>>().join(", ");
                    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE {} SET {sets} WHERE rowid = ?", quote(t))))
                        .bind(rowid)
                        .execute(&mut *conn)
                        .await?;
                } else {
                    let row = ctx
                        .schema
                        .select(conn, t, &["rowid".to_string()], &[Value::from(rowid)])
                        .await
                        .unwrap_or_default();
                    let key = row.first().map(|r| ctx.schema.key_of(t, r)).unwrap_or_default();
                    record_conflict(
                        conn,
                        ctx,
                        "orphan",
                        t,
                        &Value::Object(key).to_string(),
                        None,
                        row.first().map(|r| Value::Object(r.clone())),
                    )
                    .await?;
                    sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM {} WHERE rowid = ?", quote(t))))
                        .bind(rowid)
                        .execute(&mut *conn)
                        .await?;
                    ctx.report.conflicts += 1;
                    if ctx.report.added > 0 {
                        ctx.report.added -= 1;
                    }
                }
            }
        }
        let after: Vec<(String, String)> = sqlx::query_as("SELECT tbl, key FROM sync_tombstones WHERE deleted_at >= ?")
            .bind(started)
            .fetch_all(&mut *conn)
            .await?;
        for (tbl, key) in after.into_iter().filter(|k| !before.contains(k)) {
            sqlx::query("DELETE FROM sync_tombstones WHERE tbl = ? AND key = ?")
                .bind(tbl)
                .bind(key)
                .execute(&mut *conn)
                .await?;
        }
        if found == 0 {
            break;
        }
    }
    Ok(())
}

/// 묶음 합치기. `settings`는 이 기기의 settings.json 전체 — 같이 쓰는 칸을 받은 것으로 바꾼다(dry_run이면 그대로).
/// 호출하는 쪽이 바뀐 settings를 저장한다
pub async fn merge(db: &SqlitePool, data: &BundleData, local_device: &str, settings: &mut Map<String, Value>, dry_run: bool) -> Result<MergeReport> {
    let m = &data.manifest;
    if m.device_id == local_device {
        return Err(AppError::validation("이 기기에서 만든 묶음은 합치지 않음", None));
    }
    let local_schema = sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(version) FROM _sqlx_migrations WHERE success = 1")
        .fetch_one(db)
        .await?
        .unwrap_or(0);
    if m.schema_version > local_schema {
        return Err(AppError::validation(
            format!(
                "더 새 앱에서 만든 묶음 (DB 버전 {} > 이 기기 {}) — 이 기기의 앱을 업데이트하세요",
                m.schema_version, local_schema
            ),
            None,
        ));
    }

    let mut tx = db.begin().await?;
    sqlx::query("PRAGMA defer_foreign_keys = ON").execute(&mut *tx).await?;
    let schema = Schema::load(&mut tx).await?;
    let peer: Option<(i64, i64)> = sqlx::query_as("SELECT last_bundle_created_at, last_merge_at FROM sync_peers WHERE device_id = ?")
        .bind(&m.device_id)
        .fetch_optional(&mut *tx)
        .await?;
    if let Some((seen, _)) = peer {
        if m.created_at <= seen {
            return Err(AppError::validation("이미 합친 묶음이거나 그보다 오래된 묶음", None));
        }
    }
    let mut ctx = Ctx {
        schema: &schema,
        out: outgoing(&schema),
        device: m.device_id.clone(),
        now: now_ms(),
        remote_seen: peer.map_or(i64::MIN, |p| p.0),
        // 이 기기 행이 "그 뒤로 고친 것"인지의 기준 (둘 다 이 기기 시계):
        //   그 기기에서 마지막으로 합친 시각, 그리고 그 기기가 이미 합친 이 기기 묶음의 시각(묶음의 acks).
        // 둘 다 없으면(처음 만남) 이 기기의 모든 행을 "고친 것"으로 본다 (내용이 다르면 둘 다 남김)
        local_base: peer.map_or(i64::MIN, |p| p.1).max(m.acks.get(local_device).copied().unwrap_or(i64::MIN)),
        id_map: HashMap::new(),
        restored: HashSet::new(),
        held: m.held.get(local_device).map(|v| v.iter().cloned().collect()).unwrap_or_default(),
        acks: m.acks.clone(),
        report: MergeReport { dry_run, ..Default::default() },
    };

    apply_tombstones(&mut tx, &mut ctx, &data.tombstones).await?;
    let names: Vec<String> = data.tables.keys().filter(|t| schema.columns.contains_key(*t)).cloned().collect();
    ctx.report.skipped += data
        .tables
        .iter()
        .filter(|(t, _)| !schema.columns.contains_key(*t))
        .map(|(_, r)| r.len() as u32)
        .sum::<u32>();
    let order = table_order(&schema, &names);
    for t in &order {
        apply_rows(&mut tx, &mut ctx, t, &data.tables[t]).await?;
    }
    let mut touched = order.clone();
    touched.push("conversations".into());
    touched.push("messages".into());
    touched.dedup();
    fix_orphans(&mut tx, &mut ctx, &touched).await?;

    if !data.settings.is_empty() {
        ctx.report.settings_applied = true;
        if !dry_run {
            settings_scope::merge_shared(settings, &data.settings);
        }
    }
    sqlx::query(
        "INSERT INTO sync_peers (device_id, last_bundle_created_at, last_merge_at, merges) VALUES (?, ?, ?, 1)
         ON CONFLICT(device_id) DO UPDATE SET last_bundle_created_at = excluded.last_bundle_created_at, last_merge_at = excluded.last_merge_at, merges = merges + 1",
    )
    .bind(&m.device_id)
    .bind(m.created_at)
    .bind(ctx.now)
    .execute(&mut *tx)
    .await?;

    let report = ctx.report;
    if dry_run {
        tx.rollback().await?;
    } else {
        tx.commit().await?;
    }
    Ok(report)
}
