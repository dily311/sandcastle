//! 보내기 전 프롬프트 훅 (CLAUDE.md 6-D-5, ADR-012).
//!
//! 프롬프트는 백엔드가 짓지만 플러그인 코드는 화면(iframe)에서 돈다. 그래서 보내기 직전:
//! 백엔드 → 화면에 요청(이벤트) → 호스트가 `prompt.modify` 플러그인을 차례로 부름 → `answer_plugin_prompt`로 답 → 여기서 받아 넣는다.
//! - 그런 플러그인이 하나도 돌고 있지 않으면 묻지 않는다(기다림 없음).
//! - 기다리는 시간은 정해 두고, 넘으면 플러그인 없이 보낸다(그 플러그인 기록에 남김). 화면이 없을 때(창이 닫힘)도 같다.
//! - 플러그인은 블록을 **더하기만** 한다: 시스템 규칙 끝(rules) 또는 대화 기록의 뒤에서 depth번째(history, 작가 노트와 같은 자리).
//!   있는 프롬프트를 고치거나 빼지는 못한다.
//! - 블록 수·글 길이·토큰에 상한이 있고, 넘는 것은 버리고 기록한다. 넣은 것은 응답 meta.info.pluginPrompt에 남아 컨텍스트 탭에 보인다.
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use super::runtime;
use crate::{
    models::ai_service::{ChatContext, HistoryItem},
    service::prompt::token::count_tokens,
};

pub const PERMISSION: &str = "prompt.modify";
pub const REQUEST_EVENT: &str = "user-plugin-prompt";
const MAX_BLOCKS: usize = 5;
const MAX_BLOCK_CHARS: usize = 4000;
/// 플러그인 하나가 더할 수 있는 토큰
pub const MAX_TOKENS_PER_PLUGIN: usize = 1000;
/// 모든 플러그인을 합친 토큰
pub const MAX_TOKENS_TOTAL: usize = 2000;
/// 화면의 호스트가 플러그인 하나에 주는 시간 (ms) — 화면과 같게
pub const PER_PLUGIN_MS: u64 = 2000;
const MAX_WAIT_MS: u64 = 8000;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptHookRequest {
    pub request_id: String,
    pub conversation_id: String,
    pub character_id: Option<String>,
    /// 이번 입력 (비었으면 이어 쓰기·다시 만들기)
    pub user_input: String,
    /// 부를 플러그인 (설치 순서)
    pub plugins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PromptBlockInput {
    pub text: String,
    /// rules(시스템 규칙 끝) / history(대화 기록 안, depth)
    pub position: String,
    pub depth: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginPromptAnswer {
    pub plugin_id: String,
    pub blocks: Vec<PromptBlockInput>,
}

/// 응답 meta.info.pluginPrompt에 남는 것
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginPromptUse {
    pub plugin_id: String,
    pub name: String,
    pub blocks: u32,
    pub tokens: u32,
}

static PENDING: LazyLock<Mutex<HashMap<String, oneshot::Sender<Vec<PluginPromptAnswer>>>>> = LazyLock::new(Default::default);

fn pending() -> std::sync::MutexGuard<'static, HashMap<String, oneshot::Sender<Vec<PluginPromptAnswer>>>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// 화면의 답. 기다리는 요청이 없으면(시간이 지남) false
pub fn answer(request_id: &str, answers: Vec<PluginPromptAnswer>) -> bool {
    match pending().remove(request_id) {
        Some(tx) => tx.send(answers).is_ok(),
        None => false,
    }
}

/// 이번에 부를 플러그인 (도는 중이고 prompt.modify 권한)
pub fn hooked_plugins() -> Vec<String> {
    runtime::running_ids()
        .into_iter()
        .filter(|id| runtime::has_permission(id, PERMISSION))
        .collect()
}

/// 화면에 묻고 답을 기다린다. emit은 이벤트 보내기 (테스트에서는 바로 답함)
pub async fn ask(
    conversation_id: &str,
    character_id: Option<&str>,
    user_input: &str,
    emit: impl FnOnce(&PromptHookRequest) -> bool,
) -> Vec<PluginPromptAnswer> {
    let plugins = hooked_plugins();
    if plugins.is_empty() {
        return vec![];
    }
    let request = PromptHookRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_string(),
        character_id: character_id.map(str::to_string),
        user_input: user_input.to_string(),
        plugins: plugins.clone(),
    };
    let (tx, rx) = oneshot::channel();
    pending().insert(request.request_id.clone(), tx);
    if !emit(&request) {
        pending().remove(&request.request_id);
        return vec![];
    }
    let wait = Duration::from_millis((PER_PLUGIN_MS * plugins.len() as u64 + 500).min(MAX_WAIT_MS));
    match tokio::time::timeout(wait, rx).await {
        Ok(Ok(a)) => a,
        _ => {
            pending().remove(&request.request_id);
            for id in &plugins {
                runtime::log(id, "warn", "프롬프트 훅: 화면에서 답이 오지 않아 플러그인 없이 보냄");
            }
            vec![]
        }
    }
}

/// 답을 검사해 컨텍스트에 넣고, 넣은 것을 돌려준다 (상한을 넘는 블록은 버리고 기록)
pub fn apply(ctx: &mut ChatContext, answers: Vec<PluginPromptAnswer>) -> Vec<PluginPromptUse> {
    let order = hooked_plugins();
    let mut total = 0usize;
    let mut used = vec![];
    // 설치 순서대로 (화면이 섞어 보내도)
    let mut answers = answers;
    answers.sort_by_key(|a| order.iter().position(|id| *id == a.plugin_id).unwrap_or(usize::MAX));
    let mut seen = std::collections::HashSet::new();
    for a in answers {
        if !order.contains(&a.plugin_id) || !seen.insert(a.plugin_id.clone()) {
            continue; // 부르지 않았거나 권한이 없는 플러그인, 두 번 온 답
        }
        let name = runtime::name_of(&a.plugin_id).unwrap_or_else(|| a.plugin_id.clone());
        let mut mine = 0usize;
        let mut count = 0u32;
        for (i, b) in a.blocks.iter().enumerate() {
            let text = b.text.trim();
            if text.is_empty() {
                continue;
            }
            if i >= MAX_BLOCKS {
                runtime::log(&a.plugin_id, "warn", &format!("프롬프트 블록은 {MAX_BLOCKS}개까지 — 나머지는 버림"));
                break;
            }
            if text.chars().count() > MAX_BLOCK_CHARS {
                runtime::log(&a.plugin_id, "warn", &format!("프롬프트 블록이 {MAX_BLOCK_CHARS}자를 넘어 버림"));
                continue;
            }
            let content = format!("[플러그인: {name}]\n{text}");
            let t = count_tokens(&content);
            if mine + t > MAX_TOKENS_PER_PLUGIN || total + t > MAX_TOKENS_TOTAL {
                runtime::log(&a.plugin_id, "warn", "프롬프트 토큰 상한을 넘어 블록을 버림");
                continue;
            }
            match b.position.as_str() {
                "history" => {
                    let depth = b.depth.unwrap_or(0) as usize;
                    let at = ctx.history.len().saturating_sub(depth);
                    ctx.history.insert(
                        at,
                        HistoryItem {
                            images: vec![],
                            role: "system".into(),
                            content,
                        },
                    );
                }
                _ => ctx.system_rules.push(content),
            }
            mine += t;
            total += t;
            count += 1;
        }
        if count > 0 {
            used.push(PluginPromptUse {
                plugin_id: a.plugin_id.clone(),
                name,
                blocks: count,
                tokens: mine as u32,
            });
        }
    }
    used
}
