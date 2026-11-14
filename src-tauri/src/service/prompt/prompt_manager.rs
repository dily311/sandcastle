//! 프롬프트 조립기.
//!
//! 입력: ChatContext(규칙·캐릭터·페르소나·월드인포·대화기록) + 사용자 메시지 + 프리셋
//! 출력: 공급자와 무관한 PromptMessage (역할이 붙은 메시지 목록 + 생성 옵션)
//!
//! ## 조립 순서
//! 1. 앱 고정 규칙(`system_rules`) — 항상 맨 앞. 프리셋/게임이 바꿀 수 없다 (ADR-003)
//! 2. 프리셋의 relative 블록을 sort_order 순으로 배치
//!    - 마커 블록(identifier)은 컨텍스트 값으로 채운다: charDescription, personaDescription,
//!      dialogueExamples, worldInfo(Before/After), chatHistory
//!    - 그 외 블록은 content를 매크로 치환해 그대로 넣는다
//! 3. chatHistory 위치에 대화 기록 + 이번 사용자 메시지, 그 사이에 in_chat 블록을 depth만큼 앞에 끼움
//!
//! ## 토큰 예산
//! `context_limit - max_tokens(응답 몫)`을 넘지 않도록
//! 1. 고정 부분(규칙, 블록, 설명, 사용자 메시지)을 먼저 잡고 — 이것만으로 넘치면 오류
//! 2. 남은 예산의 `world_info_ratio`만큼 월드인포 (상시 항목 → 우선순위 순)
//! 3. 나머지를 최근 대화부터 채운다
use serde_json::Value;

use crate::{
    db::{self, conn::DbPool},
    error::{AppError, ErrorCode, Result},
    models::{
        ai_service::{
            ChatContext, ChatOptions, GenerationType, PromptBlocks, PromptMessage,
            PromptMessageItem, PromptPresets, PromptRequest, PromptRole,
        },
        conversation::WorldInfoEntry,
    },
    service::prompt::token::{count_message_tokens, count_tokens},
};

/// 월드인포 키워드를 찾을 최근 대화 수
const WORLD_INFO_SCAN_DEPTH: usize = 10;

/// DB에서 불러온 프리셋 + 블록
#[derive(Debug, Clone)]
pub struct LoadedPreset {
    pub preset: PromptPresets,
    pub blocks: Vec<PromptBlocks>,
}

pub struct PromptManager;

impl PromptManager {
    /// 프리셋 로드. id가 없거나 DB에 없으면 None (기본 배치 사용)
    pub async fn load_preset(db: &DbPool, preset_id: Option<&str>) -> Result<Option<LoadedPreset>> {
        let Some(id) = preset_id.filter(|s| !s.is_empty()) else {
            return Ok(None);
        };
        let preset = match db::ai_service::get_prompt_preset(db, id).await {
            Ok(p) => p,
            Err(sqlx::Error::RowNotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let blocks = db::ai_service::get_prompt_blocks_by_preset(db, id).await?;
        Ok(Some(LoadedPreset { preset, blocks }))
    }

    /// 프리셋을 DB에서 불러와 조립
    pub async fn build_with_db(&self, db: &DbPool, request: PromptRequest) -> Result<PromptMessage> {
        let preset = Self::load_preset(db, request.preset_id.as_deref()).await?;
        self.build(request, preset.as_ref())
    }

    /// 순수 조립 (DB 접근 없음)
    pub fn build(&self, request: PromptRequest, preset: Option<&LoadedPreset>) -> Result<PromptMessage> {
        let ctx = &request.context;
        let macros = Macros::from_context(ctx);
        let options = preset.map(|p| chat_options_from(&p.preset)).unwrap_or_default();

        // ---- 배치 결정
        let (mut layout, in_chat) = match preset {
            Some(p) if !p.blocks.is_empty() => layout_from_preset(p, request.generation_type, &macros),
            _ => (default_layout(&macros), vec![]),
        };
        if !layout.iter().any(|s| matches!(s, Slot::Section { section: Section::ChatHistory, .. })) {
            layout.push(Slot::Section {
                section: Section::ChatHistory,
                role: PromptRole::User,
                label: None,
            });
        }

        // ---- 고정 내용
        let rules = macros.apply(ctx.system_rules.join("\n").trim());
        let sections = SectionValues {
            char_description: ctx.character_info.as_deref().map(|s| macros.apply(s)),
            persona_description: Some(macros.apply(&ctx.persona_info)),
            dialogue_examples: ctx.example_dialogue.as_deref().map(|s| macros.apply(s)),
        };
        let user_message = request.user_message.trim().to_string();

        // ---- 예산
        let budget = if request.context_limit == 0 {
            usize::MAX
        } else {
            (request.context_limit as usize).saturating_sub(options.max_tokens.unwrap_or(0) as usize)
        };

        let mut fixed = token_if(&rules) + token_if(&user_message);
        for slot in &layout {
            fixed += match slot {
                Slot::Text { content, .. } => token_if(content),
                Slot::Section { section, label, .. } => sections
                    .get(*section)
                    .map(|v| token_if(&with_label(label.as_deref(), v)))
                    .unwrap_or(0),
            };
        }
        fixed += in_chat.iter().map(|b| token_if(&b.content)).sum::<usize>();

        if fixed > budget {
            return Err(AppError::Validation {
                code: ErrorCode::AiContextOverflow,
                message: format!(
                    "고정 프롬프트({fixed} 토큰)가 사용 가능한 컨텍스트({budget} 토큰)를 넘습니다. 컨텍스트 크기를 늘리거나 응답 길이를 줄여 주세요"
                ),
                details: Some(serde_json::json!({ "fixedTokens": fixed, "budget": budget })),
            });
        }
        let remaining = budget - fixed;

        // ---- 월드인포
        let ratio = request.world_info_ratio.clamp(0.0, 1.0) as f64;
        let wi_budget = if remaining == usize::MAX { usize::MAX } else { (remaining as f64 * ratio) as usize };
        let world_info = select_world_info(ctx, &user_message, wi_budget, &macros);
        let wi_label = layout.iter().find_map(|s| match s {
            Slot::Section { section: Section::WorldInfo, label, .. } => Some(label.clone()),
            _ => None,
        });
        let world_info_text = (!world_info.is_empty())
            .then(|| with_label(wi_label.flatten().as_deref(), &world_info.join("\n")));
        let wi_used = world_info_text.as_deref().map(token_if).unwrap_or(0);

        // ---- 대화 기록 (최근부터)
        let history_budget = remaining.saturating_sub(wi_used);
        let mut used = 0usize;
        let mut history: Vec<(PromptRole, String)> = vec![];
        for item in ctx.history.iter().rev() {
            if item.content.trim().is_empty() {
                continue;
            }
            let t = count_message_tokens(&item.content);
            if used.saturating_add(t) > history_budget {
                break;
            }
            used += t;
            history.push((PromptRole::from_history(&item.role), item.content.clone()));
        }
        history.reverse();
        if !user_message.is_empty() {
            history.push((PromptRole::User, user_message));
        }

        // ---- 조립
        let mut out: Vec<(PromptRole, String)> = vec![];
        if !rules.is_empty() {
            out.push((PromptRole::System, rules));
        }
        let has_wi_slot = layout
            .iter()
            .any(|s| matches!(s, Slot::Section { section: Section::WorldInfo, .. }));
        if !has_wi_slot {
            if let Some(wi) = &world_info_text {
                out.push((PromptRole::System, wi.clone()));
            }
        }

        let mut wi_placed = false;
        let mut history_placed = false;
        for slot in layout {
            match slot {
                Slot::Text { role, content } => {
                    if !content.trim().is_empty() {
                        out.push((role, content));
                    }
                }
                Slot::Section { section: Section::WorldInfo, role, .. } => {
                    if !wi_placed {
                        if let Some(wi) = &world_info_text {
                            out.push((role, wi.clone()));
                        }
                        wi_placed = true;
                    }
                }
                Slot::Section { section: Section::ChatHistory, .. } => {
                    if !history_placed {
                        out.extend(interleave_in_chat(std::mem::take(&mut history), &in_chat));
                        history_placed = true;
                    }
                }
                Slot::Section { section, role, label } => {
                    if let Some(v) = sections.get(section) {
                        out.push((role, with_label(label.as_deref(), v)));
                    }
                }
            }
        }

        Ok(PromptMessage {
            chat_options: options,
            messages: out
                .into_iter()
                .map(|(role, content)| PromptMessageItem {
                    token: count_message_tokens(&content) as i64,
                    role,
                    content,
                })
                .collect(),
        })
    }
}

// =======================================
// 배치
// =======================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    CharDescription,
    PersonaDescription,
    DialogueExamples,
    WorldInfo,
    ChatHistory,
}

impl Section {
    fn from_identifier(id: &str) -> Option<Self> {
        match id.to_ascii_lowercase().replace('_', "").as_str() {
            "chardescription" => Some(Self::CharDescription),
            "personadescription" => Some(Self::PersonaDescription),
            "dialogueexamples" | "mesexamples" => Some(Self::DialogueExamples),
            "worldinfo" | "worldinfobefore" | "worldinfoafter" => Some(Self::WorldInfo),
            "chathistory" => Some(Self::ChatHistory),
            _ => None,
        }
    }
}

enum Slot {
    Text { role: PromptRole, content: String },
    Section { section: Section, role: PromptRole, label: Option<String> },
}

/// 대화 기록 사이에 끼우는 블록
struct InChat {
    depth: usize,
    role: PromptRole,
    content: String,
}

struct SectionValues {
    char_description: Option<String>,
    persona_description: Option<String>,
    dialogue_examples: Option<String>,
}

impl SectionValues {
    /// 고정 섹션의 내용 (비어 있으면 None). WorldInfo/ChatHistory는 예산 단계에서 따로 채움
    fn get(&self, section: Section) -> Option<&str> {
        let v = match section {
            Section::CharDescription => self.char_description.as_deref(),
            Section::PersonaDescription => self.persona_description.as_deref(),
            Section::DialogueExamples => self.dialogue_examples.as_deref(),
            Section::WorldInfo | Section::ChatHistory => None,
        };
        v.map(str::trim).filter(|s| !s.is_empty())
    }
}

/// 프리셋이 없을 때의 기본 배치
fn default_layout(macros: &Macros) -> Vec<Slot> {
    let section = |section, label: &str| Slot::Section {
        section,
        role: PromptRole::System,
        label: Some(macros.apply(label)),
    };
    vec![
        section(Section::WorldInfo, "[World Info]"),
        section(Section::CharDescription, "[Character: {{char}}]"),
        section(Section::PersonaDescription, "[User: {{user}}]"),
        section(Section::DialogueExamples, "[Example Dialogue]"),
        Slot::Section {
            section: Section::ChatHistory,
            role: PromptRole::User,
            label: None,
        },
    ]
}

fn layout_from_preset(
    preset: &LoadedPreset,
    generation_type: GenerationType,
    macros: &Macros,
) -> (Vec<Slot>, Vec<InChat>) {
    let mut blocks: Vec<&PromptBlocks> = preset
        .blocks
        .iter()
        .filter(|b| b.enabled && trigger_matches(&b.triggers_json, generation_type))
        .collect();
    blocks.sort_by_key(|b| b.sort_order);

    let mut layout = vec![];
    let mut in_chat = vec![];
    for b in blocks {
        let role = PromptRole::from_history(&b.role);
        if b.position == "in_chat" {
            let content = macros.apply(&b.content);
            if !content.trim().is_empty() {
                in_chat.push(InChat {
                    depth: b.depth.max(0) as usize,
                    role,
                    content,
                });
            }
            continue;
        }
        match Section::from_identifier(&b.identifier) {
            Some(section) => layout.push(Slot::Section {
                section,
                role,
                label: None,
            }),
            None => layout.push(Slot::Text {
                role,
                content: macros.apply(&b.content),
            }),
        }
    }
    (layout, in_chat)
}

/// triggers_json이 비었거나 배열이 아니면 모든 생성에서 사용
fn trigger_matches(triggers: &Value, generation_type: GenerationType) -> bool {
    match triggers.as_array() {
        Some(arr) if !arr.is_empty() => arr
            .iter()
            .any(|t| t.as_str() == Some(generation_type.as_str())),
        _ => true,
    }
}

/// in_chat 블록을 "끝에서 depth번째 메시지 앞"에 끼운다. depth 0 = 맨 끝.
/// 같은 위치의 블록은 프리셋 순서를 유지한다.
fn interleave_in_chat(chat: Vec<(PromptRole, String)>, in_chat: &[InChat]) -> Vec<(PromptRole, String)> {
    let n = chat.len();
    let at = |pos: usize| {
        in_chat
            .iter()
            .filter(move |b| n - b.depth.min(n) == pos)
            .map(|b| (b.role, b.content.clone()))
    };
    let mut out = Vec::with_capacity(n + in_chat.len());
    for (i, msg) in chat.into_iter().enumerate() {
        out.extend(at(i));
        out.push(msg);
    }
    out.extend(at(n));
    out
}

// =======================================
// 월드인포
// =======================================

fn select_world_info(ctx: &ChatContext, user_message: &str, budget: usize, macros: &Macros) -> Vec<String> {
    if budget == 0 || ctx.world_info.is_empty() {
        return vec![];
    }
    let mut scan = user_message.to_lowercase();
    for item in ctx.history.iter().rev().take(WORLD_INFO_SCAN_DEPTH) {
        scan.push('\n');
        scan.push_str(&item.content.to_lowercase());
    }

    let mut seen = std::collections::HashSet::new();
    let mut active: Vec<(usize, &WorldInfoEntry)> = ctx
        .world_info
        .iter()
        .enumerate()
        .filter(|(_, e)| e.enabled && !e.content.trim().is_empty())
        .filter(|(_, e)| seen.insert(e.id.clone()))
        .filter(|(_, e)| e.constant || world_info_matches(e, &scan))
        .collect();
    // 상시 항목 먼저, 그다음 우선순위 높은 순, 같으면 원래 순서
    active.sort_by(|(ia, a), (ib, b)| {
        b.constant
            .cmp(&a.constant)
            .then(b.priority.cmp(&a.priority))
            .then(ia.cmp(ib))
    });

    let mut used = crate::service::prompt::token::MESSAGE_OVERHEAD;
    let mut out = vec![];
    for (_, e) in active {
        let content = macros.apply(e.content.trim());
        let t = count_tokens(&content) + 1;
        if used.saturating_add(t) > budget {
            continue; // 큰 항목 하나 때문에 작은 항목까지 빠지지 않도록 건너뛰기만 함
        }
        used += t;
        out.push(content);
    }
    out
}

fn world_info_matches(entry: &WorldInfoEntry, scan: &str) -> bool {
    let hit = |keys: &[String]| {
        keys.iter()
            .map(|k| k.trim().to_lowercase())
            .any(|k| !k.is_empty() && scan.contains(&k))
    };
    hit(&entry.keys) && (entry.secondary_keys.is_empty() || hit(&entry.secondary_keys))
}

// =======================================
// 생성 옵션
// =======================================

/// 프리셋 설정 → ChatOptions. settings_json이 config_json보다 우선.
/// 네이티브 키와 SillyTavern 키를 모두 읽는다.
fn chat_options_from(preset: &PromptPresets) -> ChatOptions {
    let mut o = ChatOptions::default();
    for src in [Some(&preset.config_json), preset.settings_json.as_ref()].into_iter().flatten() {
        if let Some(v) = num(src, &["temperature", "temp_openai"]) {
            o.temperature = Some(v as f32);
        }
        if let Some(v) = num(src, &["max_tokens", "maxTokens", "openai_max_tokens", "max_response_length"]) {
            if v > 0.0 {
                o.max_tokens = Some(v as u32);
            }
        }
        if let Some(v) = num(src, &["top_p", "topP"]) {
            o.top_p = Some(v as f32);
        }
        if let Some(v) = num(src, &["top_k", "topK"]) {
            o.top_k = (v > 0.0).then_some(v as u32);
        }
        if let Some(v) = ["stream", "stream_openai", "streaming"]
            .iter()
            .find_map(|k| src.get(*k).and_then(Value::as_bool))
        {
            o.stream = v;
        }
    }
    o
}

fn num(v: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|k| v.get(*k).and_then(Value::as_f64))
}

// =======================================
// 매크로
// =======================================

/// `{{char}}`, `{{user}}` 등 치환. 모르는 매크로는 그대로 둔다.
struct Macros {
    values: Vec<(&'static str, String)>,
}

impl Macros {
    fn from_context(ctx: &ChatContext) -> Self {
        let base = Self {
            values: vec![
                ("char", ctx.char_name.clone().unwrap_or_else(|| "Assistant".into())),
                ("user", ctx.user_name.clone().unwrap_or_else(|| "User".into())),
                ("newline", "\n".into()),
                ("trim", String::new()),
                ("noop", String::new()),
            ],
        };
        // 설명 안의 {{char}}/{{user}}도 풀어 둔 뒤 {{description}} 등으로 쓸 수 있게
        let description = base.apply(ctx.character_info.as_deref().unwrap_or_default());
        let persona = base.apply(&ctx.persona_info);
        let examples = base.apply(ctx.example_dialogue.as_deref().unwrap_or_default());
        let mut values = base.values;
        values.push(("description", description));
        values.push(("persona", persona));
        values.push(("mesexamples", examples));
        Self { values }
    }

    fn apply(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(start) = rest.find("{{") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            match after.find("}}") {
                Some(end) => {
                    let key = after[..end].trim().to_ascii_lowercase();
                    match self.values.iter().find(|(k, _)| *k == key) {
                        Some((_, v)) => out.push_str(v),
                        None => out.push_str(&rest[start..start + 2 + end + 2]),
                    }
                    rest = &after[end + 2..];
                }
                None => {
                    out.push_str(&rest[start..]);
                    rest = "";
                }
            }
        }
        out.push_str(rest);
        out
    }
}

// =======================================
// 도우미
// =======================================

fn token_if(text: &str) -> usize {
    if text.trim().is_empty() {
        0
    } else {
        count_message_tokens(text)
    }
}

fn with_label(label: Option<&str>, value: &str) -> String {
    match label {
        Some(l) if !l.is_empty() => format!("{l}\n{value}"),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests;
