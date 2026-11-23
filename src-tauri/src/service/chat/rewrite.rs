//! 자동 재생성 (CLAUDE.md 3-B-4, ADR-017).
//!
//! linter가 "금지어"나 "단어 반복 오류"를 찾았고 사용자가 그 규칙의 자동 재생성을 켜 두었으면, 응답이 저장된 뒤 코어 재생성(지시 추가)으로
//! **새 스와이프**를 하나 더 만든다. 원본은 스와이프로 남는다. 이것은 재시도가 아니라 품질 때문에 다시 쓰는 것이다 — 공급자 오류·취소·예산
//! 한도로 끝난 응답에는 이어가지 않고, 실패해도 같은 요청을 다시 보내지 않는다.
//!
//! 사슬은 한 메시지에 묶인다(`AutoCtx`): 실행할 때 그 메시지가 아직 마지막 AI 메시지이고 같은 스와이프일 때만 돈다. 시도 횟수는 작업
//! payload에 있다(메시지 meta에 두면 다음 사용자 재생성에 새어 나간다). 사용자가 보내거나 멈추면 자동 작업이 먼저 멈춘다.
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager};

use super::lint::Finding;
use crate::{
    error::{AppError, Result},
    models::{conversation::Conversation, conversation::Message, settings::PluginSetting, settings::RewriteSetting},
    sys::{plugins, settings::SettingsState},
};

pub const FEATURE: &str = "rewrite";
/// 한 응답에 자동으로 더 만드는 최대 횟수 (한 번마다 응답 하나의 요금이 들므로 설정이 더 커도 이만큼까지)
pub const HARD_CAP: u32 = 3;
/// 사슬이 시작하기 전에 지금 작업이 끝나기를 기다리는 시간
const WAIT_STEP: Duration = Duration::from_millis(250);
const WAIT_STEPS: u32 = 80;

/// 자동 재생성 작업이 묶인 메시지와 몇 번째 시도인지 (작업 payload)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoCtx {
    pub message_id: String,
    /// 묶은 시점의 스와이프 (사용자가 넘기거나 고쳤으면 멈춤)
    pub swipe_id: i64,
    /// 이번이 몇 번째 자동 시도인가 (1부터)
    pub attempt: u32,
    pub max: u32,
    pub rules: Vec<String>,
}

/// turn이 다음 시도가 필요하다고 알리는 것 (응답 meta.info.rewritePending)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    /// 다음 시도 번호 (1부터)
    pub attempt: u32,
    pub max: u32,
    pub rules: Vec<String>,
    pub instruction: String,
}

pub fn swipe_id_of(meta: Option<&Value>) -> i64 {
    meta.and_then(|m| m.get("swipeId")).and_then(Value::as_i64).unwrap_or(0)
}

impl AutoCtx {
    /// 이 메시지가 사슬이 묶은 그 메시지·스와이프인가
    pub fn matches(&self, m: &Message) -> bool {
        m.id == self.message_id && swipe_id_of(m.meta.as_ref()) == self.swipe_id
    }
}

pub fn max_attempts(cfg: &RewriteSetting) -> u32 {
    cfg.max_attempts.clamp(1, HARD_CAP)
}

/// 자동 재생성의 방아쇠가 될 수 있는 결과: 고치지 않은 금지어·단어 반복 오류 중 사용자가 켠 규칙
pub fn eligible(f: &Finding, cfg: &RewriteSetting) -> bool {
    !f.fixed
        && match f.rule.as_str() {
            "rewriteBanned" => cfg.banned,
            "stuck" => cfg.stuck,
            _ => false,
        }
}

fn findings_of(lint: Option<&Value>) -> Vec<Finding> {
    lint.and_then(|l| l.get("findings"))
        .and_then(|f| serde_json::from_value(f.clone()).ok())
        .unwrap_or_default()
}

/// 지시 글: 문제가 된 말만 짧게 (결함이 있는 원문은 넣지 않는다 — 새로 생성하는 것이라 같은 결함을 베끼지 않게)
pub fn build_instruction(findings: &[&Finding]) -> String {
    let mut lines = vec![];
    for f in findings {
        let detail = f.detail.as_deref().unwrap_or("");
        match f.rule.as_str() {
            "rewriteBanned" => {
                let words: Vec<String> = detail.split('\u{1f}').filter(|w| !w.is_empty()).map(|w| format!("「{w}」")).collect();
                if !words.is_empty() {
                    lines.push(format!("Do not use these expressions; say it differently: {}", words.join(", ")));
                }
            }
            "stuck" => {
                let unit: String = detail.chars().take(20).collect();
                lines.push(format!(
                    "Do not repeat the same word or phrase over and over (the previous response broke down repeating 「{unit}」)"
                ));
            }
            _ => {}
        }
    }
    lines.join("\n")
}

/// 다음 자동 시도가 필요한가. `lint`는 방금 저장하는 응답의 meta.lint, `attempts_done`은 이 사슬에서 이미 한 자동 시도 수
pub fn decide(cfg: &RewriteSetting, lint: Option<&Value>, attempts_done: u32) -> Option<Pending> {
    let max = max_attempts(cfg);
    if attempts_done >= max {
        return None;
    }
    let all = findings_of(lint);
    let hits: Vec<&Finding> = all.iter().filter(|f| eligible(f, cfg)).collect();
    if hits.is_empty() {
        return None;
    }
    let instruction = build_instruction(&hits);
    if instruction.is_empty() {
        return None;
    }
    let mut rules: Vec<String> = hits.iter().map(|f| f.rule.clone()).collect();
    rules.dedup();
    Some(Pending {
        attempt: attempts_done + 1,
        max,
        rules,
        instruction,
    })
}

/// 켜져 있고 이 대화에서 쓸 수 있을 때만 설정을 돌려준다 (폰 대화는 제외, 대화별 끄기 `rewriteOff`)
pub fn enabled_for(app: &AppHandle, conv: &Conversation) -> Option<RewriteSetting> {
    if !plugins::is_enabled(app, plugins::id::REWRITE) || conv.mode == "phone" {
        return None;
    }
    if conv.meta.as_ref().and_then(|m| m.get("rewriteOff")).and_then(Value::as_bool) == Some(true) {
        return None;
    }
    app.try_state::<SettingsState<PluginSetting>>()?.0.lock().ok().map(|s| s.rewrite.clone())
}

pub fn pending_of(m: &Message) -> Option<Pending> {
    m.meta
        .as_ref()?
        .pointer("/info/rewritePending")
        .filter(|v| !v.is_null())
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// 화면에 "다시 쓰는 중"을 켜고 끄는 신호
pub fn emit_active(app: &AppHandle, conversation_id: &str, active: bool) {
    let _ = app.emit("chat:rewriting", json!({ "conversationId": conversation_id, "active": active }));
}

/// 다음 자동 시도를 예약한다 (지금 작업이 끝난 뒤, 그동안 사용자가 다른 작업을 시작했으면 포기)
pub fn schedule(app: &AppHandle, conversation_id: &str, assistant: &Message, p: Pending) {
    emit_active(app, conversation_id, true);
    let ctx = AutoCtx {
        message_id: assistant.id.clone(),
        swipe_id: swipe_id_of(assistant.meta.as_ref()),
        attempt: p.attempt,
        max: p.max,
        rules: p.rules,
    };
    let (app, conv) = (app.clone(), conversation_id.to_string());
    tauri::async_runtime::spawn(async move {
        use crate::service::chat::chat_task::{active_task_of, submit_auto, ChatJob};
        // 지금 이 작업 자신이 끝나기를 기다린다
        for _ in 0..WAIT_STEPS {
            match active_task_of(&app, &conv).await {
                Ok(None) => break,
                Ok(Some(_)) => tokio::time::sleep(WAIT_STEP).await,
                Err(_) => break,
            }
        }
        let busy = active_task_of(&app, &conv).await.ok().flatten().is_some();
        if busy || !plugins::is_enabled(&app, plugins::id::REWRITE) {
            emit_active(&app, &conv, false); // 사용자가 다른 일을 시작했음 — 자동은 물러난다
            return;
        }
        let label = format!("자동 재생성 ({}/{})", ctx.attempt, ctx.max);
        let job = ChatJob::Regenerate {
            swipe: true,
            extra: Some(p.instruction),
            auto: Some(ctx),
        };
        if let Err(e) = submit_auto(&app, &conv, job, &label).await {
            eprintln!("[rewrite] 자동 재생성을 시작하지 못함: {e}");
            emit_active(&app, &conv, false);
        }
    });
}

fn note(meta: &mut Value, status: &str, extra: Value) {
    meta["rewriteNote"] = json!({ "status": status, "detail": extra, "at": chrono::Utc::now().timestamp_millis() });
}

async fn write_meta(db: &SqlitePool, message_id: &str, meta: &Value) -> Result<()> {
    sqlx::query("UPDATE messages SET meta = ? WHERE id = ?")
        .bind(meta.to_string())
        .bind(message_id)
        .execute(db)
        .await?;
    Ok(())
}

async fn read_meta(db: &SqlitePool, message_id: &str) -> Result<Value> {
    let raw: Option<Option<String>> = sqlx::query_scalar("SELECT meta FROM messages WHERE id = ?")
        .bind(message_id)
        .fetch_optional(db)
        .await?;
    let Some(raw) = raw else {
        return Err(AppError::not_found("메시지가 없음"));
    };
    Ok(raw
        .as_deref()
        .and_then(|r| serde_json::from_str(r).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({})))
}

/// 자동 재생성이 실패로 끝났을 때: 원본은 그대로 두고 이유만 메시지에 남긴다 (사용자가 직접 멈춘 것은 남기지 않음)
pub async fn on_failed(app: &AppHandle, db: &SqlitePool, conversation_id: &str, ctx: &AutoCtx, reason: &str, user_cancelled: bool) {
    if !user_cancelled {
        if let Ok(mut meta) = read_meta(db, &ctx.message_id).await {
            note(&mut meta, "failed", json!({ "reason": reason, "attempt": ctx.attempt }));
            let _ = write_meta(db, &ctx.message_id, &meta).await;
        }
    }
    emit_active(app, conversation_id, false);
}

fn eligible_count(lint: Option<&Value>, cfg: &RewriteSetting) -> usize {
    findings_of(lint).iter().filter(|f| eligible(f, cfg)).count()
}

/// 사슬이 끝났는데 아직 걸린 것이 남아 있으면: 걸린 수가 가장 적은 스와이프를 고른다 (같으면 나중 것). 메모를 남긴다. 고른 스와이프 번호
pub async fn finish_exhausted(db: &SqlitePool, message_id: &str, cfg: &RewriteSetting, attempts: u32) -> Result<Option<usize>> {
    let mut meta = read_meta(db, message_id).await?;
    let swipes = meta.get("swipes").and_then(Value::as_array).cloned().unwrap_or_default();
    let current = swipe_id_of(Some(&meta)).max(0) as usize;
    let mut best = current;
    if !swipes.is_empty() {
        let counts: Vec<usize> = swipes.iter().map(|s| eligible_count(s.pointer("/meta/lint"), cfg)).collect();
        let min = counts.iter().copied().min().unwrap_or(0);
        best = counts.iter().rposition(|c| *c == min).unwrap_or(current);
    }
    if best != current {
        let s = swipes[best].clone();
        let content = s.get("content").and_then(Value::as_str).unwrap_or_default().to_string();
        meta["swipeId"] = json!(best);
        for key in ["info", "images", "lint"] {
            match s.pointer(&format!("/meta/{key}")) {
                Some(v) => meta[key] = v.clone(),
                None => {
                    if let Some(o) = meta.as_object_mut() {
                        o.remove(key);
                    }
                }
            }
        }
        note(&mut meta, "exhausted", json!({ "attempts": attempts }));
        sqlx::query("UPDATE messages SET content = ?, meta = ?, updated_at = ? WHERE id = ?")
            .bind(&content)
            .bind(meta.to_string())
            .bind(chrono::Utc::now().timestamp_millis())
            .bind(message_id)
            .execute(db)
            .await?;
    } else {
        note(&mut meta, "exhausted", json!({ "attempts": attempts }));
        write_meta(db, message_id, &meta).await?;
    }
    Ok(Some(best))
}

/// 사슬 끝 처리: 걸린 것이 없으면 아무것도 하지 않고, 남아 있으면 가장 나은 스와이프를 고른다
pub async fn finish_chain(app: &AppHandle, db: &SqlitePool, conversation_id: &str, assistant: &Message, attempts: u32) {
    let cfg = app
        .try_state::<SettingsState<PluginSetting>>()
        .and_then(|s| s.0.lock().ok().map(|s| s.rewrite.clone()));
    if let Some(cfg) = cfg {
        let lint = assistant.meta.as_ref().and_then(|m| m.get("lint"));
        if eligible_count(lint, &cfg) > 0 {
            if let Err(e) = finish_exhausted(db, &assistant.id, &cfg, attempts).await {
                eprintln!("[rewrite] 사슬을 마무리하지 못함: {e}");
            }
        }
    }
    emit_active(app, conversation_id, false);
}

#[cfg(test)]
mod tests;
