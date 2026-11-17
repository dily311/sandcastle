//! 대화 화면 커맨드: 보내기/다시 생성/멈추기, 폴더, 대화 관리(이름·고정·이동·복제·분기·설정)
use tauri::{ipc::Channel, AppHandle, State};

use crate::{
    db::{chat_ui, conversation},
    error::{AppError, ErrorCode, Result},
    models::{
        ai_service::{PromptMessage, StreamPayload},
        conversation::{ChatTurn, Conversation, ConversationFolder, ConversationOptions},
        settings::AppSetting,
    },
    service::chat::chat_task::{self, ChatJob},
    state::AppState,
    sys::settings::SettingsState,
};

/// ChatManager(anyhow) 오류에서 원래 AppError(코드·재시도 여부)를 되살린다.
pub(crate) fn into_app_error(e: anyhow::Error) -> AppError {
    e.downcast::<AppError>()
        .unwrap_or_else(|e| AppError::ai_provider(ErrorCode::AiProviderError, e.to_string(), false))
}

fn found(ok: bool, what: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(AppError::not_found(format!("{what}을(를) 찾을 수 없음")))
    }
}

fn required(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::validation("이름이 비었음", None));
    }
    Ok(name.to_string())
}

// ---------- 보내기 ----------

/// 입력을 저장하고 응답을 만든다. `text`가 비었으면 이어서 응답만 (이어쓰기).
/// 생성은 작업(ChatStream)으로 돌아서 다른 대화로 옮겨 가도 계속되고, 앱이 꺼지면 다시 시도할 수 있다.
/// 응답 조각은 `on_event`로 흐르고, 저장된 메시지는 반환값으로 온다.
#[tauri::command]
#[specta::specta]
pub async fn chat_send(app: AppHandle, conversation_id: String, text: String, attachments: Vec<String>, on_event: Channel<StreamPayload>) -> Result<ChatTurn> {
    let saved = chat_task::run(&app, &conversation_id, ChatJob::Send { text, attachments }, on_event).await?;
    chat_task::into_turn(saved)
}

/// 마지막 AI 메시지 다시 만들기. `swipe`면 스와이프로 쌓고, 아니면 스와이프를 버리고 바꾼다.
#[tauri::command]
#[specta::specta]
pub async fn chat_regenerate(app: AppHandle, conversation_id: String, swipe: bool, on_event: Channel<StreamPayload>) -> Result<ChatTurn> {
    let saved = chat_task::run(&app, &conversation_id, ChatJob::Regenerate { swipe }, on_event).await?;
    chat_task::into_turn(saved)
}

/// 진행 중인 응답 멈추기 (받은 데까지는 저장됨). 멈출 게 있었으면 true
#[tauri::command]
#[specta::specta]
pub async fn chat_cancel(app: AppHandle, conversation_id: String) -> Result<bool> {
    chat_task::cancel(&app, &conversation_id).await
}

/// 이 AI 메시지를 만들 때 실제로 보낸 프롬프트 (없으면 null: 가져온 메시지, 취소된 응답 등)
#[tauri::command]
#[specta::specta]
pub async fn get_message_prompt(state: State<'_, AppState>, message_id: String) -> Result<Option<PromptMessage>> {
    let v = chat_ui::get_prompt(&state.db, &message_id).await?;
    Ok(v.and_then(|v| serde_json::from_value(v).ok()))
}

// ---------- 폴더 ----------

#[tauri::command]
#[specta::specta]
pub async fn list_conversation_folders(state: State<'_, AppState>, character_id: String) -> Result<Vec<ConversationFolder>> {
    Ok(chat_ui::list_folders(&state.db, &character_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_conversation_folder(state: State<'_, AppState>, character_id: String, name: String) -> Result<ConversationFolder> {
    let name = required(&name)?;
    Ok(chat_ui::create_folder(&state.db, &character_id, &name).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn rename_conversation_folder(state: State<'_, AppState>, id: String, name: String) -> Result<()> {
    let name = required(&name)?;
    found(chat_ui::rename_folder(&state.db, &id, &name).await?, "폴더")
}

/// 폴더만 지운다. 안의 대화는 폴더 밖으로 나온다.
#[tauri::command]
#[specta::specta]
pub async fn delete_conversation_folder(state: State<'_, AppState>, id: String) -> Result<()> {
    found(chat_ui::delete_folder(&state.db, &id).await?, "폴더")
}

// ---------- 대화 관리 ----------

#[tauri::command]
#[specta::specta]
pub async fn rename_conversation(state: State<'_, AppState>, id: String, title: String) -> Result<()> {
    let title = required(&title)?;
    found(chat_ui::rename_conversation(&state.db, &id, &title).await?, "대화")
}

#[tauri::command]
#[specta::specta]
pub async fn set_conversation_pinned(state: State<'_, AppState>, id: String, pinned: bool) -> Result<()> {
    found(chat_ui::set_pinned(&state.db, &id, pinned).await?, "대화")
}

/// 폴더로 옮기기. `folder_id`가 null이면 폴더 밖으로
#[tauri::command]
#[specta::specta]
pub async fn move_conversation(state: State<'_, AppState>, id: String, folder_id: Option<String>) -> Result<()> {
    found(chat_ui::move_to_folder(&state.db, &id, folder_id.as_deref()).await?, "대화")
}

/// 대화별 페르소나·프리셋·API (프리셋/API가 null이면 앱 설정의 선택값을 씀)
#[tauri::command]
#[specta::specta]
pub async fn set_conversation_options(state: State<'_, AppState>, id: String, options: ConversationOptions) -> Result<Conversation> {
    found(chat_ui::set_options(&state.db, &id, &options).await?, "대화")?;
    Ok(conversation::get_conversation(&state.db, &id).await?)
}

/// 대화 통째로 복제
#[tauri::command]
#[specta::specta]
pub async fn duplicate_conversation(state: State<'_, AppState>, id: String) -> Result<Conversation> {
    let src = conversation::get_conversation(&state.db, &id).await?;
    let new_id = chat_ui::copy_conversation(&state.db, &id, &format!("{} (복제)", src.title), None).await?;
    Ok(conversation::get_conversation(&state.db, &new_id).await?)
}

/// 분기 나누기: 이 메시지까지 복사한 새 대화
#[tauri::command]
#[specta::specta]
pub async fn branch_conversation(state: State<'_, AppState>, id: String, message_id: String) -> Result<Conversation> {
    let src = conversation::get_conversation(&state.db, &id).await?;
    let msg = chat_ui::get_message(&state.db, &message_id).await?;
    let title = format!("{} · #{} 분기", src.title, msg.seq);
    let new_id = chat_ui::copy_conversation(&state.db, &id, &title, Some(&message_id)).await?;
    // 어디서 갈라졌는지 기억 (부모 대화로 돌아가기·분기 경로 표시용)
    let branch = serde_json::json!({ "branch": { "parentId": id, "messageId": message_id, "seq": msg.seq } });
    chat_ui::patch_conversation_meta(&state.db, &new_id, &branch).await?;
    Ok(conversation::get_conversation(&state.db, &new_id).await?)
}

/// 화면에 보일 메시지 (role, 원문)
#[derive(serde::Deserialize, specta::Type)]
pub struct DisplayItem {
    pub role: String,
    pub content: String,
}

/// 메시지들을 화면용으로: 표시용 정규식 + {{char}}/{{user}} 이름. 순서대로, 마지막이 가장 최근
#[tauri::command]
#[specta::specta]
pub async fn display_messages(state: State<'_, AppState>, conversation_id: String, items: Vec<DisplayItem>) -> Result<Vec<String>> {
    use crate::service::chat::regex_processor::{RegexMacros, RegexProcessor};
    let conv = conversation::get_conversation(&state.db, &conversation_id).await?;
    let char_name = conversation::get_character(&state.db, &conv.character_id)
        .await
        .map(|c| c.name)
        .unwrap_or_default();
    let persona_id = match conv.persona_id.as_str() {
        "" => conversation::get_system_default_persona(&state.db).await.unwrap_or_else(|_| "default".into()),
        id => id.to_string(),
    };
    let user_name = conversation::get_persona(&state.db, &persona_id).await.map(|p| p.name).unwrap_or_default();
    let regex = RegexProcessor::load(
        &state.db,
        Some(&conv.character_id),
        conv.prompt_preset_id.as_deref(),
        RegexMacros { char_name, user_name },
    )
    .await?;
    let n = items.len();
    Ok(items
        .iter()
        .enumerate()
        .map(|(i, m)| regex.for_display(&m.content, &m.role, n - 1 - i))
        .collect())
}

/// 컨텍스트 사용량 (오른쪽 패널). 마지막으로 보낸 프롬프트 기준
#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ContextUsage {
    /// 마지막 프롬프트의 토큰 (보낸 적 없으면 None)
    pub used: Option<u32>,
    /// 그중 시스템(규칙·캐릭터·월드인포·노트 등)
    pub system: u32,
    /// 그중 대화 기록
    pub history: u32,
    /// 대화 기록에 들어간 메시지 수
    pub history_count: u32,
    /// 전체 컨텍스트 한도 (응답 몫 포함)
    pub limit: u32,
    /// 응답 몫으로 비워 두는 토큰
    pub reserved: u32,
    pub preset_id: Option<String>,
    pub preset_name: Option<String>,
    /// 커뮤니티 플러그인이 더한 프롬프트 블록 (6-D-5, 시스템 토큰에 들어 있음)
    pub plugins: Vec<crate::service::community::prompt_hook::PluginPromptUse>,
}

#[tauri::command]
#[specta::specta]
pub async fn context_usage(state: State<'_, AppState>, settings: State<'_, SettingsState<AppSetting>>, conversation_id: String) -> Result<ContextUsage> {
    use crate::models::ai_service::PromptRole;
    use crate::service::{
        chat::chat_manager::current_conv_request,
        prompt::prompt_manager::{chat_options_from, context_limit_from, PromptManager, DEFAULT_CONTEXT_LIMIT as CONTEXT_LIMIT},
    };

    let conv = conversation::get_conversation(&state.db, &conversation_id).await?;
    let char_default = conversation::get_character(&state.db, &conv.character_id)
        .await
        .ok()
        .and_then(|c| c.default_prompt_preset_id);
    let fallback = current_conv_request(&settings).map(|r| r.preset_id).unwrap_or_default();
    let preset_id = conv
        .prompt_preset_id
        .clone()
        .filter(|s| !s.is_empty())
        .or(char_default.filter(|s| !s.is_empty()))
        .or(Some(fallback).filter(|s| !s.is_empty()));
    let preset = PromptManager::load_preset(&state.db, preset_id.as_deref()).await?;
    let limit = preset.as_ref().and_then(|p| context_limit_from(&p.preset)).unwrap_or(CONTEXT_LIMIT);
    let reserved = preset.as_ref().and_then(|p| chat_options_from(&p.preset).max_tokens).unwrap_or(0);

    // 마지막 AI 메시지에 저장된 프롬프트
    let recent = conversation::get_messages(&state.db, &conversation_id, 30, None).await?;
    let mut prompt: Option<PromptMessage> = None;
    let mut plugins = vec![];
    for m in recent.iter().rev().filter(|m| m.role == "assistant") {
        if let Some(v) = chat_ui::get_prompt(&state.db, &m.id).await? {
            if let Ok(p) = serde_json::from_value::<PromptMessage>(v) {
                prompt = Some(p);
                plugins = m
                    .meta
                    .as_ref()
                    .and_then(|meta| meta.pointer("/info/pluginPrompt"))
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                break;
            }
        }
    }
    let (mut system, mut history, mut history_count) = (0u32, 0u32, 0u32);
    if let Some(p) = &prompt {
        for m in &p.messages {
            let t = m.token.max(0) as u32;
            if m.role == PromptRole::System {
                system += t;
            } else {
                history += t;
                history_count += 1;
            }
        }
    }
    Ok(ContextUsage {
        used: prompt.map(|_| system + history),
        system,
        history,
        history_count,
        limit,
        reserved,
        preset_name: preset.as_ref().map(|p| p.preset.name.clone()),
        preset_id,
        plugins,
    })
}

/// 대화 하나를 파일로 내보내기. format: "jsonl"(SillyTavern 채팅) | "txt"(읽기용 글)
#[tauri::command]
#[specta::specta]
pub async fn export_conversation(state: State<'_, AppState>, id: String, format: String, path: String) -> Result<u32> {
    let conv = conversation::get_conversation(&state.db, &id).await?;
    let messages = conversation::get_messages(&state.db, &id, i64::MAX, None).await?;
    let char_name = conversation::get_character(&state.db, &conv.character_id)
        .await
        .map(|c| c.name)
        .unwrap_or_else(|_| "Character".into());
    let user_name = conversation::get_persona(&state.db, &conv.persona_id)
        .await
        .map(|p| p.name)
        .unwrap_or_else(|_| "User".into());
    let iso = |ms: i64| {
        chrono::DateTime::from_timestamp_millis(ms)
            .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .unwrap_or_default()
    };

    let out = match format.as_str() {
        "jsonl" => {
            let mut lines = vec![serde_json::json!({
                "user_name": user_name,
                "character_name": char_name,
                "create_date": iso(conv.created_at),
                "chat_metadata": { "note_prompt": conv.meta.as_ref().and_then(|m| m.pointer("/authorNote/text")).cloned().unwrap_or_default() },
            })
            .to_string()];
            for m in &messages {
                let meta = m.meta.clone().unwrap_or_default();
                let is_user = m.role == "user";
                let mut row = serde_json::json!({
                    "name": m.name.clone().unwrap_or_else(|| if is_user { user_name.clone() } else { char_name.clone() }),
                    "is_user": is_user,
                    "is_system": meta.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false),
                    "send_date": iso(m.created_at),
                    "mes": m.content,
                    "extra": {},
                });
                if let Some(sw) = meta.get("swipes").and_then(|v| v.as_array()).filter(|a| !a.is_empty()) {
                    row["swipes"] = serde_json::Value::Array(sw.iter().map(|s| s.get("content").cloned().unwrap_or_default()).collect());
                    row["swipe_id"] = meta.get("swipeId").cloned().unwrap_or(serde_json::json!(0));
                }
                lines.push(row.to_string());
            }
            lines.join("\n") + "\n"
        }
        "txt" => {
            let mut s = format!("{}\n{} · {}\n\n", conv.title, char_name, iso(conv.created_at));
            for m in &messages {
                let name = m
                    .name
                    .clone()
                    .unwrap_or_else(|| if m.role == "user" { user_name.clone() } else { char_name.clone() });
                s.push_str(&format!("{name}:\n{}\n\n", m.content.trim()));
            }
            s
        }
        _ => return Err(AppError::validation(format!("모르는 형식: {format}"), None)),
    };
    std::fs::write(&path, out).map_err(|e| AppError::io(format!("파일을 쓸 수 없습니다: {e}")))?;
    Ok(messages.len() as u32)
}

/// 대화 meta에 키를 합친다 (작가 노트, 메모 등). 값이 null이면 그 키를 지움
#[tauri::command]
#[specta::specta]
pub async fn patch_conversation_meta(state: State<'_, AppState>, id: String, patch: serde_json::Value) -> Result<Conversation> {
    if !patch.is_object() {
        return Err(AppError::validation("patch는 객체여야 합니다", None));
    }
    chat_ui::patch_conversation_meta(&state.db, &id, &patch).await?;
    Ok(conversation::get_conversation(&state.db, &id).await?)
}

/// 메시지 여러 개 한 번에 지우기. 지운 개수를 돌려준다
#[tauri::command]
#[specta::specta]
pub async fn delete_messages(state: State<'_, AppState>, ids: Vec<String>) -> Result<u32> {
    let mut tx = state.db.begin().await?;
    let mut n = 0;
    for id in &ids {
        n += sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?
            .rows_affected() as u32;
    }
    tx.commit().await?;
    Ok(n)
}
