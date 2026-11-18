//! 플러그인 설정 (manifest의 `settings`, CLAUDE.md 6-D-8, ADR-012).
//! 앱이 설정 화면을 그려 주고 값은 플러그인 저장소의 예약 키(`__settings:<칸>`)에 칸마다 하나씩 둔다 — 한 칸을 고치는 것이
//! 다른 칸을 읽고 다시 쓰지 않으므로, 빠르게 잇달아 고쳐도 서로 덮어쓰지 않는다.
//! 플러그인은 `sandcastle.settings.get()`으로 읽기만 한다(쓰는 것은 사용자뿐). 예약 키(`__`로 시작)는 플러그인의 `storage` API로 못 건드린다.
use serde_json::{Map, Value};
use sqlx::SqlitePool;

use super::{manifest::PluginSettingField, storage};
use crate::error::{AppError, Result};

const KEY_PREFIX: &str = "__settings:";
const MAX_TEXT: usize = 2000;

fn bad(msg: impl Into<String>) -> AppError {
    AppError::validation(msg, None)
}

/// 선언한 기본값 (없으면 종류마다: 글 "", 숫자 0, 켜짐 false, 고르기 첫 항목)
pub fn default_of(f: &PluginSettingField) -> Value {
    if let Some(d) = &f.default {
        return d.clone();
    }
    match f.kind.as_str() {
        "number" => Value::from(0),
        "toggle" => Value::Bool(false),
        "select" => f.options.first().cloned().map(Value::String).unwrap_or(Value::Null),
        _ => Value::String(String::new()),
    }
}

/// 값이 그 칸의 종류에 맞는지 (맞지 않으면 거절)
pub fn check(f: &PluginSettingField, v: &Value) -> Result<Value> {
    match f.kind.as_str() {
        "text" => {
            let s = v.as_str().ok_or_else(|| bad(format!("{}: 글이어야 함", f.key)))?;
            if s.chars().count() > MAX_TEXT {
                return Err(bad(format!("{}: {MAX_TEXT}자까지", f.key)));
            }
            Ok(Value::String(s.to_string()))
        }
        "number" => {
            let n = v.as_f64().filter(|n| n.is_finite()).ok_or_else(|| bad(format!("{}: 숫자여야 함", f.key)))?;
            Ok(serde_json::Number::from_f64(n).map(Value::Number).unwrap_or(Value::Null))
        }
        "toggle" => v.as_bool().map(Value::Bool).ok_or_else(|| bad(format!("{}: true/false여야 함", f.key))),
        "select" => {
            let s = v
                .as_str()
                .filter(|s| f.options.iter().any(|o| o == s))
                .ok_or_else(|| bad(format!("{}: 고를 수 있는 값이 아님", f.key)))?;
            Ok(Value::String(s.to_string()))
        }
        other => Err(bad(format!("{}: 모르는 종류 {other}", f.key))),
    }
}

fn key_of(field: &str) -> String {
    format!("{KEY_PREFIX}{field}")
}

/// 선언한 칸 전부의 값 (저장한 값이 있으면 그것, 없거나 지금 종류에 안 맞으면 기본값)
pub async fn get(db: &SqlitePool, plugin: &str, decl: &[PluginSettingField]) -> Result<Value> {
    let mut out = Map::new();
    for f in decl {
        let saved = storage::get_raw(db, plugin, &key_of(&f.key)).await?;
        let v = if saved.is_null() { None } else { check(f, &saved).ok() };
        out.insert(f.key.clone(), v.unwrap_or_else(|| default_of(f)));
    }
    Ok(Value::Object(out))
}

/// 한 칸 저장. null이면 기본값으로 되돌린다
pub async fn set(db: &SqlitePool, plugin: &str, decl: &[PluginSettingField], key: &str, value: &Value) -> Result<()> {
    let f = decl.iter().find(|f| f.key == key).ok_or_else(|| bad(format!("선언하지 않은 설정: {key}")))?;
    if value.is_null() {
        storage::delete_raw(db, plugin, &key_of(key)).await
    } else {
        storage::set_raw(db, plugin, &key_of(key), &check(f, value)?).await
    }
}
