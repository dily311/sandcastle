//! 고치기 제안 → 차이 보기 → 승인 (CLAUDE.md 6-D-3, ADR-012). 플러그인이 쓰고, 에이전트(5-B)도 같은 틀을 쓴다.
//!
//! - 제안은 원본을 바꾸지 않는다. 고칠 수 있는 칸은 종류마다 정해 둔 것뿐(`Kind::fields`)이고, 값의 모양·크기를 검사한다.
//! - 적용: 지금 값이 제안할 때 값(before)과 같아야 한다. 그 사이 바뀌었으면 적용하지 않는다(덮어쓰지 않음).
//! - 되돌리기: 지금 값이 적용한 값(after)과 같을 때만 before로 돌린다. 새로 만든 항목은 지운다.
//! - 쓰기는 원래 테이블의 UPDATE·INSERT라 변경 추적(updated_at)·지운 기록·동기화가 평소대로 동작한다.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{Row, SqlitePool};

use crate::{
    db::schema::quote,
    error::{AppError, Result},
};

const MAX_TEXT: usize = 100_000;
const MAX_LIST_ITEMS: usize = 200;
const MAX_LIST_ITEM: usize = 200;
const MAX_REASON: usize = 1000;
/// 한 출처가 쌓아 둘 수 있는 기다리는 제안
pub const MAX_PENDING: i64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type, strum_macros::EnumString, strum_macros::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ProposalKind {
    Character,
    Persona,
    /// 로어북 자체 (이름·설명)
    Lorebook,
    LorebookEntry,
    /// 대화의 메시지 글
    Message,
}

#[derive(Clone, Copy, PartialEq)]
enum Ty {
    Text,
    /// 글자 배열 (DB에는 JSON)
    List,
    Flag,
}

struct Field {
    api: &'static str,
    column: &'static str,
    ty: Ty,
    label: &'static str,
}

const fn f(api: &'static str, column: &'static str, ty: Ty, label: &'static str) -> Field {
    Field { api, column, ty, label }
}

const CHARACTER: &[Field] = &[
    f("name", "name", Ty::Text, "이름"),
    f("description", "description", Ty::Text, "설명"),
    f("firstMessage", "first_message", Ty::Text, "첫 메시지"),
    f("exampleDialogue", "example_dialogue", Ty::Text, "예시 대화"),
    f("creatorNotes", "creator_notes", Ty::Text, "제작자 메모"),
    f("systemPromptOverride", "system_prompt_override", Ty::Text, "시스템 프롬프트"),
    f("tags", "tags", Ty::List, "태그"),
];
const PERSONA: &[Field] = &[
    f("name", "name", Ty::Text, "이름"),
    f("description", "description", Ty::Text, "설명"),
    f("tags", "tags", Ty::List, "태그"),
];
const LOREBOOK: &[Field] = &[f("name", "name", Ty::Text, "이름"), f("description", "description", Ty::Text, "설명")];
const ENTRY: &[Field] = &[
    f("keys", "keys", Ty::List, "키워드"),
    f("secondaryKeys", "secondary_keys", Ty::List, "보조 키워드"),
    f("content", "content", Ty::Text, "내용"),
    f("comment", "comment", Ty::Text, "메모"),
    f("enabled", "enabled", Ty::Flag, "켜짐"),
];
const MESSAGE: &[Field] = &[f("content", "content", Ty::Text, "내용")];

impl ProposalKind {
    fn table(self) -> &'static str {
        match self {
            Self::Character => "characters",
            Self::Persona => "personas",
            Self::Lorebook => "worldinfo_books",
            Self::LorebookEntry => "worldinfo_entries",
            Self::Message => "messages",
        }
    }
    fn fields(self) -> &'static [Field] {
        match self {
            Self::Character => CHARACTER,
            Self::Persona => PERSONA,
            Self::Lorebook => LOREBOOK,
            Self::LorebookEntry => ENTRY,
            Self::Message => MESSAGE,
        }
    }
    /// 이 종류를 고치는 데 필요한 플러그인 권한
    pub fn write_permission(self) -> &'static str {
        match self {
            Self::Character => "data.write:character",
            Self::Persona => "data.write:persona",
            Self::Lorebook | Self::LorebookEntry => "data.write:lorebook",
            Self::Message => "data.write:chat",
        }
    }
    /// 새로 만들기를 받는 종류 (부모 테이블)
    fn parent_table(self) -> Option<&'static str> {
        match self {
            Self::LorebookEntry => Some("worldinfo_books"),
            _ => None,
        }
    }
    fn label_sql(self) -> &'static str {
        match self {
            Self::Character | Self::Persona | Self::Lorebook => "name",
            Self::LorebookEntry => "COALESCE(NULLIF(comment, ''), substr(content, 1, 40))",
            Self::Message => "substr(content, 1, 40)",
        }
    }
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProposalField {
    pub field: String,
    pub label: String,
    pub before: Value,
    pub after: Value,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ChangeProposal {
    pub id: String,
    pub source: String,
    pub kind: ProposalKind,
    /// update / create
    pub op: String,
    pub target_id: Option<String>,
    pub parent_id: Option<String>,
    pub target_label: String,
    pub fields: Vec<ProposalField>,
    pub reason: String,
    /// pending / applied / rejected / reverted
    pub status: String,
    pub created_at: i64,
    pub decided_at: Option<i64>,
    /// 기다리는 제안: 제안한 뒤 원본이 바뀌어 적용할 수 없음
    pub outdated: bool,
    /// 적용한 제안: 그 뒤 원본이 그대로라 되돌릴 수 있음
    pub can_revert: bool,
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn bad(msg: impl Into<String>) -> AppError {
    AppError::validation(msg, None)
}

fn field(kind: ProposalKind, api: &str) -> Option<&'static Field> {
    kind.fields().iter().find(|f| f.api == api)
}

/// 제안의 값을 검사해 표준 모양으로 (Text → 글자, List → 글자 배열, Flag → bool)
fn normalize(f: &Field, v: &Value) -> Result<Value> {
    match f.ty {
        Ty::Text => {
            let s = v.as_str().ok_or_else(|| bad(format!("{}: 글자여야 함", f.api)))?;
            if s.chars().count() > MAX_TEXT {
                return Err(bad(format!("{}: 너무 김 ({MAX_TEXT}자까지)", f.api)));
            }
            Ok(Value::String(s.to_string()))
        }
        Ty::List => {
            let a = v.as_array().ok_or_else(|| bad(format!("{}: 글자 배열이어야 함", f.api)))?;
            if a.len() > MAX_LIST_ITEMS {
                return Err(bad(format!("{}: {MAX_LIST_ITEMS}개까지", f.api)));
            }
            let mut out = vec![];
            for x in a {
                let s = x.as_str().ok_or_else(|| bad(format!("{}: 글자 배열이어야 함", f.api)))?;
                if s.chars().count() > MAX_LIST_ITEM {
                    return Err(bad(format!("{}: 항목은 {MAX_LIST_ITEM}자까지", f.api)));
                }
                out.push(Value::String(s.to_string()));
            }
            Ok(Value::Array(out))
        }
        Ty::Flag => v.as_bool().map(Value::Bool).ok_or_else(|| bad(format!("{}: true/false여야 함", f.api))),
    }
}

/// DB 값 → 표준 모양
fn from_db(f: &Field, row: &sqlx::sqlite::SqliteRow) -> Value {
    match f.ty {
        Ty::Text => Value::String(row.try_get::<Option<String>, _>(f.column).ok().flatten().unwrap_or_default()),
        Ty::List => row
            .try_get::<Option<String>, _>(f.column)
            .ok()
            .flatten()
            .and_then(|r| serde_json::from_str::<Value>(&r).ok())
            .filter(Value::is_array)
            .unwrap_or_else(|| Value::Array(vec![])),
        Ty::Flag => Value::Bool(row.try_get::<Option<i64>, _>(f.column).ok().flatten().unwrap_or(1) != 0),
    }
}

/// 표준 모양 → DB에 넣을 값
enum DbVal {
    Text(String),
    Int(i64),
}

fn to_db(f: &Field, v: &Value) -> DbVal {
    match f.ty {
        Ty::Text => DbVal::Text(v.as_str().unwrap_or("").to_string()),
        Ty::List => DbVal::Text(serde_json::to_string(v).unwrap_or_else(|_| "[]".into())),
        Ty::Flag => DbVal::Int(v.as_bool().unwrap_or(true) as i64),
    }
}

/// 지금 값 (그 종류의 고칠 수 있는 칸 전부). 행이 없으면 None
async fn current<'e, E>(db: E, kind: ProposalKind, id: &str) -> Result<Option<(Map<String, Value>, String)>>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let cols: Vec<String> = kind.fields().iter().map(|f| quote(f.column)).collect();
    let sql = format!(
        "SELECT {}, {} AS __label FROM {} WHERE id = ?",
        cols.join(", "),
        kind.label_sql(),
        quote(kind.table())
    );
    let Some(row) = sqlx::query(sqlx::AssertSqlSafe(sql)).bind(id).fetch_optional(db).await? else {
        return Ok(None);
    };
    let values = kind.fields().iter().map(|f| (f.api.to_string(), from_db(f, &row))).collect();
    let label = row.try_get::<Option<String>, _>("__label").ok().flatten().unwrap_or_default();
    Ok(Some((values, label)))
}

fn pick(all: &Map<String, Value>, keys: &Map<String, Value>) -> Map<String, Value> {
    keys.keys().filter_map(|k| all.get(k).map(|v| (k.clone(), v.clone()))).collect()
}

async fn exists(db: &SqlitePool, table: &str, id: &str) -> Result<bool> {
    let sql = format!("SELECT 1 FROM {} WHERE id = ?", quote(table));
    Ok(sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(db)
        .await?
        .is_some())
}

pub struct NewProposal<'a> {
    pub source: &'a str,
    pub kind: ProposalKind,
    /// update / create
    pub op: &'a str,
    pub target_id: Option<&'a str>,
    pub parent_id: Option<&'a str>,
    pub changes: &'a Map<String, Value>,
    pub reason: &'a str,
}

/// 제안 만들기. 원본은 바꾸지 않는다
pub async fn propose(db: &SqlitePool, p: NewProposal<'_>) -> Result<ChangeProposal> {
    if p.changes.is_empty() {
        return Err(bad("바꿀 칸이 없음"));
    }
    let mut after = Map::new();
    for (k, v) in p.changes {
        let f = field(p.kind, k).ok_or_else(|| bad(format!("고칠 수 없는 칸: {k}")))?;
        after.insert(k.clone(), normalize(f, v)?);
    }
    let reason: String = p.reason.trim().chars().take(MAX_REASON).collect();

    let (target_id, parent_id, before, label) = match p.op {
        "update" => {
            let id = p.target_id.filter(|s| !s.is_empty()).ok_or_else(|| bad("고칠 항목 id가 필요함"))?;
            let (now, label) = current(db, p.kind, id)
                .await?
                .ok_or_else(|| AppError::not_found(format!("고칠 항목이 없음: {id}")))?;
            // 지금과 같은 칸은 뺀다
            after.retain(|k, v| now.get(k) != Some(v));
            if after.is_empty() {
                return Err(bad("지금과 달라지는 칸이 없음"));
            }
            (Some(id.to_string()), None, pick(&now, &after), label)
        }
        "create" => {
            let parent_table = p.kind.parent_table().ok_or_else(|| bad("이 종류는 새로 만들기를 제안할 수 없음"))?;
            let parent = p.parent_id.filter(|s| !s.is_empty()).ok_or_else(|| bad("넣을 곳(parentId)이 필요함"))?;
            if !exists(db, parent_table, parent).await? {
                return Err(AppError::not_found(format!("넣을 곳이 없음: {parent}")));
            }
            let label = after
                .get("comment")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .or_else(|| after.get("content").and_then(Value::as_str))
                .unwrap_or("")
                .chars()
                .take(40)
                .collect();
            (None, Some(parent.to_string()), Map::new(), label)
        }
        other => return Err(bad(format!("모르는 op: {other}"))),
    };

    // 같은 제안이 이미 기다리고 있으면 그것을 돌려준다 (플러그인이 시작할 때마다 같은 제안을 다시 보내도 쌓이지 않게)
    let after_json = Value::Object(after.clone()).to_string();
    let same: Option<String> = sqlx::query_scalar(
        "SELECT id FROM change_proposals WHERE status = 'pending' AND source = ? AND kind = ? AND op = ? AND COALESCE(target_id, '') = ? AND COALESCE(parent_id, '') = ? AND after_json = ?",
    )
    .bind(p.source)
    .bind(p.kind.to_string())
    .bind(p.op)
    .bind(target_id.as_deref().unwrap_or(""))
    .bind(parent_id.as_deref().unwrap_or(""))
    .bind(&after_json)
    .fetch_optional(db)
    .await?;
    if let Some(id) = same {
        return get(db, &id).await;
    }

    let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM change_proposals WHERE source = ? AND status = 'pending'")
        .bind(p.source)
        .fetch_one(db)
        .await?;
    if pending >= MAX_PENDING {
        return Err(bad(format!("기다리는 제안이 너무 많음 ({MAX_PENDING}개까지) — 사용자가 고르기를 기다리세요")));
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO change_proposals (id, source, kind, op, target_id, parent_id, target_label, before_json, after_json, reason, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(p.source)
    .bind(p.kind.to_string())
    .bind(p.op)
    .bind(&target_id)
    .bind(&parent_id)
    .bind(&label)
    .bind(Value::Object(before).to_string())
    .bind(&after_json)
    .bind(&reason)
    .bind(now_ms())
    .execute(db)
    .await?;
    get(db, &id).await
}

type Stored = (
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
    i64,
    Option<i64>,
);
const SELECT: &str =
    "SELECT id, source, kind, op, target_id, parent_id, target_label, before_json, after_json, reason, status, created_at, decided_at FROM change_proposals";

fn obj(raw: &str) -> Map<String, Value> {
    serde_json::from_str::<Value>(raw).ok().and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

async fn to_dto(db: &SqlitePool, s: Stored) -> Result<ChangeProposal> {
    let kind: ProposalKind = s.2.parse().map_err(|_| AppError::internal(format!("모르는 제안 종류: {}", s.2)))?;
    let before = obj(&s.7);
    let after = obj(&s.8);
    let now = match &s.4 {
        Some(id) => current(db, kind, id).await?.map(|(v, _)| v),
        None => None,
    };
    let status = s.10.clone();
    let outdated = status == "pending" && s.3 == "update" && now.as_ref().is_none_or(|n| pick(n, &after) != before);
    let can_revert = status == "applied" && now.as_ref().is_some_and(|n| pick(n, &after) == after);
    let fields = after
        .iter()
        .map(|(k, v)| ProposalField {
            field: k.clone(),
            label: field(kind, k).map(|f| f.label).unwrap_or(k).to_string(),
            before: before.get(k).cloned().unwrap_or(Value::Null),
            after: v.clone(),
        })
        .collect();
    Ok(ChangeProposal {
        id: s.0,
        source: s.1,
        kind,
        op: s.3,
        target_id: s.4,
        parent_id: s.5,
        target_label: s.6,
        fields,
        reason: s.9,
        status,
        created_at: s.11,
        decided_at: s.12,
        outdated,
        can_revert,
    })
}

pub async fn get(db: &SqlitePool, id: &str) -> Result<ChangeProposal> {
    let s: Stored = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE id = ?")))
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("제안이 없음: {id}")))?;
    to_dto(db, s).await
}

/// 제안 목록 (새것부터). status를 주면 그것만
pub async fn list(db: &SqlitePool, status: Option<&str>) -> Result<Vec<ChangeProposal>> {
    let rows: Vec<Stored> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} WHERE (?1 IS NULL OR status = ?1) ORDER BY created_at DESC LIMIT 500"
    )))
    .bind(status)
    .fetch_all(db)
    .await?;
    let mut out = vec![];
    for r in rows {
        out.push(to_dto(db, r).await?);
    }
    Ok(out)
}

pub async fn pending_count(db: &SqlitePool) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM change_proposals WHERE status = 'pending'")
        .fetch_one(db)
        .await?)
}

async fn set_status(tx: &mut sqlx::SqliteConnection, id: &str, status: &str, target: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE change_proposals SET status = ?, decided_at = ?, target_id = COALESCE(?, target_id) WHERE id = ?")
        .bind(status)
        .bind(now_ms())
        .bind(target)
        .bind(id)
        .execute(tx)
        .await?;
    Ok(())
}

async fn write(tx: &mut sqlx::SqliteConnection, kind: ProposalKind, id: &str, values: &Map<String, Value>) -> Result<()> {
    let fields: Vec<&Field> = values.keys().filter_map(|k| field(kind, k)).collect();
    if fields.is_empty() {
        return Ok(());
    }
    let sets: Vec<String> = fields.iter().map(|f| format!("{} = ?", quote(f.column))).collect();
    let sql = format!("UPDATE {} SET {} WHERE id = ?", quote(kind.table()), sets.join(", "));
    let mut q = sqlx::query(sqlx::AssertSqlSafe(sql));
    for f in &fields {
        q = match to_db(f, &values[f.api]) {
            DbVal::Text(s) => q.bind(s),
            DbVal::Int(i) => q.bind(i),
        };
    }
    q.bind(id).execute(tx).await?;
    Ok(())
}

/// 적용. 그 사이 원본이 바뀌었으면 적용하지 않는다
pub async fn apply(db: &SqlitePool, id: &str) -> Result<ChangeProposal> {
    let p = get(db, id).await?;
    if p.status != "pending" {
        return Err(bad(format!("이미 고른 제안 ({})", p.status)));
    }
    let after: Map<String, Value> = p.fields.iter().map(|f| (f.field.clone(), f.after.clone())).collect();
    let mut tx = db.begin().await?;
    match p.op.as_str() {
        "update" => {
            let target = p.target_id.clone().unwrap_or_default();
            let Some((now, _)) = current(&mut *tx, p.kind, &target).await? else {
                return Err(AppError::not_found("고칠 항목이 지워졌음 — 거절하세요"));
            };
            let before: Map<String, Value> = p.fields.iter().map(|f| (f.field.clone(), f.before.clone())).collect();
            if pick(&now, &after) != before {
                return Err(bad("제안한 뒤 원본이 바뀌어 적용하지 않음 — 거절하고 다시 제안받으세요"));
            }
            write(&mut tx, p.kind, &target, &after).await?;
            set_status(&mut tx, id, "applied", None).await?;
        }
        "create" => {
            let parent = p.parent_id.clone().unwrap_or_default();
            let parent_table = p.kind.parent_table().unwrap_or("");
            let sql = format!("SELECT 1 FROM {} WHERE id = ?", quote(parent_table));
            if sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(sql))
                .bind(&parent)
                .fetch_optional(&mut *tx)
                .await?
                .is_none()
            {
                return Err(AppError::not_found("넣을 곳이 지워졌음 — 거절하세요"));
            }
            let new_id = uuid::Uuid::new_v4().to_string();
            // 지금은 로어북 항목만 새로 만들 수 있다 (parent_table이 있는 종류)
            sqlx::query(
                "INSERT INTO worldinfo_entries (id, book_id, keys, secondary_keys, content, comment, enabled, priority, constant) VALUES (?, ?, '[]', '[]', '', '', 1, 0, 0)",
            )
            .bind(&new_id)
            .bind(&parent)
            .execute(&mut *tx)
            .await?;
            write(&mut tx, p.kind, &new_id, &after).await?;
            set_status(&mut tx, id, "applied", Some(&new_id)).await?;
        }
        other => return Err(bad(format!("모르는 op: {other}"))),
    }
    tx.commit().await?;
    get(db, id).await
}

pub async fn reject(db: &SqlitePool, id: &str) -> Result<ChangeProposal> {
    let p = get(db, id).await?;
    if p.status != "pending" {
        return Err(bad(format!("이미 고른 제안 ({})", p.status)));
    }
    let mut conn = db.acquire().await?;
    set_status(&mut conn, id, "rejected", None).await?;
    drop(conn);
    get(db, id).await
}

/// 적용한 것을 되돌림. 그 뒤 원본이 또 바뀌었으면 되돌리지 않는다
pub async fn revert(db: &SqlitePool, id: &str) -> Result<ChangeProposal> {
    let p = get(db, id).await?;
    if p.status != "applied" {
        return Err(bad("적용한 제안만 되돌릴 수 있음"));
    }
    if !p.can_revert {
        return Err(bad("적용한 뒤 원본이 또 바뀌어 되돌리지 않음"));
    }
    let target = p.target_id.clone().unwrap_or_default();
    let mut tx = db.begin().await?;
    if p.op == "create" {
        let sql = format!("DELETE FROM {} WHERE id = ?", quote(p.kind.table()));
        sqlx::query(sqlx::AssertSqlSafe(sql)).bind(&target).execute(&mut *tx).await?;
    } else {
        let before: Map<String, Value> = p.fields.iter().map(|f| (f.field.clone(), f.before.clone())).collect();
        write(&mut tx, p.kind, &target, &before).await?;
    }
    set_status(&mut tx, id, "reverted", None).await?;
    tx.commit().await?;
    get(db, id).await
}

#[cfg(test)]
#[path = "proposals/tests.rs"]
mod tests;
