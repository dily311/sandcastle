//! 플러그인의 AI 호출 정책 (`ai` 권한, CLAUDE.md 6-D-7, ADR-012).
//!
//! 실제 호출은 커맨드(`call_user_plugin`)가 TaskManager 작업(`TaskType::PluginAi`, feature `plugin:<id>`) 안에서 한다 —
//! 공급자는 AiService를 거치므로 사용량·예산 한도가 그대로 적용된다. 여기는 그 앞의 **정책**만 둔다:
//! - 인자 검사와 상한 (입력 글자, 출력 토큰, 온도)
//! - 실행 주체: 화면의 호스트가 정해 넘긴다(플러그인은 못 정함). `auto`면 플러그인별 스위치(`ai_auto`, 기본 꺼짐)가 켜져 있어야 한다
//! - 플러그인마다 한 번에 하나, 1분에 몇 번까지
//! - 실패해도 자동으로 다시 시도하지 않는다 (호출한 플러그인이 알아서)
use std::{
    collections::{HashMap, VecDeque},
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

use serde_json::Value;
use sqlx::SqlitePool;

use crate::error::{AppError, Result};

pub const PERMISSION: &str = "ai";
/// system + prompt 합쳐서
pub const MAX_INPUT_CHARS: usize = 20_000;
pub const DEFAULT_MAX_TOKENS: u32 = 500;
pub const MAX_OUTPUT_TOKENS: u32 = 2_000;
pub const MAX_PER_MINUTE: usize = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct AiCall {
    pub system: Option<String>,
    pub prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

fn bad(msg: impl Into<String>) -> AppError {
    AppError::validation(msg, None)
}

/// 플러그인이 보낸 인자 → 검사한 호출. 상한을 넘는 입력은 거절, 출력 토큰·온도는 범위로 맞춘다
pub fn parse(args: &Value) -> Result<AiCall> {
    let prompt = args
        .get("prompt")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| bad("prompt가 필요함 (글)"))?
        .to_string();
    let system = match args.get("system") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.trim().to_string()).filter(|s| !s.is_empty()),
        Some(_) => return Err(bad("system은 글이어야 함")),
    };
    let total = prompt.chars().count() + system.as_deref().map_or(0, |s| s.chars().count());
    if total > MAX_INPUT_CHARS {
        return Err(bad(format!("AI에 보낼 글이 너무 김 ({MAX_INPUT_CHARS}자까지, 지금 {total}자)")));
    }
    let max_tokens = match args.get("maxTokens") {
        None | Some(Value::Null) => DEFAULT_MAX_TOKENS,
        Some(v) => v.as_u64().ok_or_else(|| bad("maxTokens는 양의 정수"))?.clamp(1, MAX_OUTPUT_TOKENS as u64) as u32,
    };
    let temperature = match args.get("temperature") {
        None | Some(Value::Null) => 0.7,
        Some(v) => v.as_f64().ok_or_else(|| bad("temperature는 숫자"))?.clamp(0.0, 2.0) as f32,
    };
    Ok(AiCall {
        system,
        prompt,
        max_tokens,
        temperature,
    })
}

#[derive(Default)]
struct State {
    running: bool,
    calls: VecDeque<Instant>,
}

static STATE: LazyLock<Mutex<HashMap<String, State>>> = LazyLock::new(Default::default);

fn state() -> std::sync::MutexGuard<'static, HashMap<String, State>> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// 호출 하나를 하는 동안 쥐고 있는다 (놓으면 다음 호출을 받을 수 있다)
pub struct Guard {
    plugin: String,
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(s) = state().get_mut(&self.plugin) {
            s.running = false;
        }
    }
}

/// 호출을 받아도 되는지: 자동 실행 스위치, 한 번에 하나, 1분에 `MAX_PER_MINUTE`번
pub async fn admit(db: &SqlitePool, plugin: &str, auto: bool) -> Result<Guard> {
    if auto {
        let on: Option<i64> = sqlx::query_scalar("SELECT ai_auto FROM user_plugins WHERE id = ?")
            .bind(plugin)
            .fetch_optional(db)
            .await?;
        if on.unwrap_or(0) == 0 {
            return Err(bad(
                "이 플러그인은 사용자가 누른 명령 밖에서 AI를 부를 수 없음 — 설정 > 커뮤니티 플러그인에서 '자동 AI 호출'을 켜면 가능 (요금이 들 수 있음)",
            ));
        }
    }
    let now = Instant::now();
    let mut all = state();
    let s = all.entry(plugin.to_string()).or_default();
    if s.running {
        return Err(bad("이전 AI 호출이 끝나지 않았음 — 하나씩 부르세요"));
    }
    while s.calls.front().is_some_and(|t| now.duration_since(*t) > Duration::from_secs(60)) {
        s.calls.pop_front();
    }
    if s.calls.len() >= MAX_PER_MINUTE {
        return Err(bad(format!("AI 호출이 너무 잦음 (1분에 {MAX_PER_MINUTE}번까지)")));
    }
    s.calls.push_back(now);
    s.running = true;
    Ok(Guard { plugin: plugin.to_string() })
}

pub async fn set_auto(db: &SqlitePool, plugin: &str, on: bool) -> Result<()> {
    let n = sqlx::query("UPDATE user_plugins SET ai_auto = ? WHERE id = ?")
        .bind(on as i64)
        .bind(plugin)
        .execute(db)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(AppError::not_found(format!("설치된 플러그인이 아님: {plugin}")));
    }
    Ok(())
}

#[cfg(test)]
pub fn reset_for_test() {
    state().clear();
}

/// 작업(TaskType::PluginAi, feature `plugin:<id>`) 안에서 공급자를 부른다. 사용량·예산 한도는 AiService가 처리하고,
/// 한도에 걸리면 공급자를 부르지 않고 BUDGET_EXCEEDED로 실패한다(자동은 항상 막힘, 직접은 "이번만 계속").
pub async fn generate(
    ai: &crate::service::ai::AiService,
    tasks: &crate::task::TaskManager,
    plugin_id: &str,
    plugin_name: &str,
    profile_id: String,
    auto: bool,
    call: AiCall,
) -> Result<Value> {
    use crate::{
        models::ai_service::{ChatOptions, PromptMessage, PromptMessageItem, PromptRole},
        service::ai::AiRequest,
        task::{CreateTaskRequest, TaskOwner, TaskType},
    };
    let item = |role, content: String| PromptMessageItem {
        images: vec![],
        role,
        token: 0,
        content,
    };
    let mut messages = vec![];
    if let Some(s) = call.system {
        messages.push(item(PromptRole::System, s));
    }
    messages.push(item(PromptRole::User, call.prompt));
    let prompt = PromptMessage {
        chat_options: ChatOptions {
            max_tokens: Some(call.max_tokens),
            stream: false,
            temperature: Some(call.temperature),
            ..ChatOptions::default()
        },
        messages,
    };
    let job = CreateTaskRequest::inline(
        TaskType::PluginAi,
        TaskOwner::System,
        &format!("plugin:{plugin_id}"),
        format!("플러그인 · {plugin_name}"),
        auto,
    );
    let resp = tasks
        .run_inline(job, |_| async { ai.generate_once(AiRequest { profile_id, prompt }).await })
        .await?;
    Ok(serde_json::json!({ "text": resp.content.trim(), "model": resp.model }))
}
