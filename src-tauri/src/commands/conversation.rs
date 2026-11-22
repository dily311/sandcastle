use crate::commands::data::move_to_trash;
use crate::db::conversation::{self};
use crate::error::{AppError, Result};
use crate::models::conversation::*;
use crate::models::regex::{RegexPlacement, RegexTarget};
use crate::models::settings::AppSetting;
use crate::service::chat::regex_processor::RegexProcessor;
use crate::service::data::trash::{TrashItem, TrashKind};
use crate::state::AppState;
use crate::sys::settings::SettingsState;
use serde_json::json;
use tauri::State;

// =======================================
// 시스템 기본 페르소나
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_conv_settings(state: State<'_, AppState>) -> Result<ConvSettings> {
    let default = conversation::get_system_default_persona(&state.db).await?;
    Ok(ConvSettings {
        default_persona_id: default.into(),
    })
}

#[tauri::command]
#[specta::specta]
pub async fn set_system_default_persona(state: State<'_, AppState>, id: String) -> Result<()> {
    conversation::set_system_default_persona(&state.db, &id).await?;
    Ok(())
}

// =======================================
// Conversations
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_all_conversations(state: State<'_, AppState>) -> Result<Vec<Conversation>> {
    let rows = conversation::get_all_conversations(&state.db).await?;
    Ok(rows)
}

#[tauri::command]
#[specta::specta]
pub async fn get_conversation(state: State<'_, AppState>, id: String) -> Result<Conversation> {
    let row = conversation::get_conversation(&state.db, &id).await?;
    Ok(row)
}

#[tauri::command]
#[specta::specta]
pub async fn create_conversation(state: State<'_, AppState>, mut input: ConversationInput) -> Result<Conversation> {
    // 모드: 비우면 chat, 그 외에는 chat / phone만 허용
    input.mode = if input.mode.trim().is_empty() {
        ConversationMode::Chat.to_string()
    } else {
        input
            .mode
            .trim()
            .to_ascii_lowercase()
            .parse::<ConversationMode>()
            .map_err(|_| AppError::validation(format!("알 수 없는 대화 모드입니다: {}", input.mode), None))?
            .to_string()
    };
    // character_id가 실제로 존재하는지 확인: 없는 캐릭터로 대화가 생기는 걸 막음
    if !conversation::char_exists(&state.db, &input.character_id).await? {
        return Err(AppError::validation(
            "존재하지 않는 캐릭터입니다",
            Some(json!({"data": &input.character_id}).into()),
        ));
    }

    let conversation = conversation::create_conversation(&state.db, input).await?;
    Ok(conversation)
}

#[tauri::command]
#[specta::specta]
pub async fn touch_conversation(state: State<'_, AppState>, id: String) -> Result<i64> {
    let result = conversation::touch_conversation(&state.db, &id).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_conversation(app: tauri::AppHandle, state: State<'_, AppState>, id: String) -> Result<TrashItem> {
    move_to_trash(&app, &state, TrashKind::Conversation, &id).await
}

// =======================================
// Character
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_all_characters(state: State<'_, AppState>) -> Result<Vec<Character>> {
    Ok(conversation::get_all_characters(&state.db).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_list_characters(state: State<'_, AppState>) -> Result<Vec<CharacterOption>> {
    let rows = conversation::get_list_characters(&state.db).await?;
    Ok(rows)
}

#[tauri::command]
#[specta::specta]
pub async fn create_character(state: State<'_, AppState>, input: CharacterInput) -> Result<Character> {
    let books = input.world_info_ids.clone();
    let mut tx = state.db.begin().await?;
    let character = conversation::create_character(&mut *tx, input).await?;
    conversation::replace_character_worldinfo(&mut tx, &character.id, &books).await?;
    tx.commit().await?;
    Ok(character)
}

/// 캐릭터 저장. 월드인포 연결도 `world_info_ids`로 통째로 바꾼다
#[tauri::command]
#[specta::specta]
pub async fn update_character(state: State<'_, AppState>, id: String, input: CharacterInput) -> Result<Character> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("이름이 비어 있습니다", None));
    }
    let mut tx = state.db.begin().await?;
    conversation::update_character(&mut *tx, &id, &input).await?;
    conversation::replace_character_worldinfo(&mut tx, &id, &input.world_info_ids).await?;
    tx.commit().await?;
    conversation::get_character(&state.db, &id)
        .await
        .map_err(|_| AppError::not_found(format!("캐릭터를 찾을 수 없습니다: {id}")))
}

/// 캐릭터 아바타: 고른 이미지를 앱 데이터 characters/avatars로 복사. `path`가 없으면 지운다
#[tauri::command]
#[specta::specta]
pub async fn set_character_avatar(app: tauri::AppHandle, state: State<'_, AppState>, id: String, path: Option<String>) -> Result<Character> {
    let old = conversation::get_character(&state.db, &id)
        .await
        .map_err(|_| AppError::not_found(format!("캐릭터를 찾을 수 없습니다: {id}")))?;
    let dir = crate::commands::character_image::app_data_dir(&app)?.join("characters").join("avatars");
    let avatar = match path {
        Some(src) => Some(copy_avatar(&dir, &src, "character")?),
        None => None,
    };
    conversation::set_character_avatar(&state.db, &id, avatar.as_deref()).await?;
    remove_unused_avatar(&state, &dir, old.avatar.as_deref()).await;
    Ok(conversation::get_character(&state.db, &id).await?)
}

/// 캐릭터 커버(배너) 이미지 바꾸기. path가 None이면 지운다. 앱이 복사해 둔 이전 커버 파일은 지운다
#[tauri::command]
#[specta::specta]
pub async fn set_character_cover(app: tauri::AppHandle, state: State<'_, AppState>, id: String, path: Option<String>) -> Result<Character> {
    let old = conversation::get_character(&state.db, &id)
        .await
        .map_err(|_| AppError::not_found(format!("캐릭터를 찾을 수 없습니다: {id}")))?;
    let dir = crate::commands::character_image::app_data_dir(&app)?.join("characters").join("covers");
    let cover = match path {
        Some(src) => Some(copy_avatar(&dir, &src, "cover")?),
        None => None,
    };
    conversation::set_character_cover(&state.db, &id, cover.as_deref()).await?;
    if let Some(old) = old.cover.filter(|o| Some(o) != cover.as_ref()) {
        let p = std::path::PathBuf::from(old);
        if p.parent() == Some(dir.as_path()) {
            let _ = std::fs::remove_file(p);
        }
    }
    Ok(conversation::get_character(&state.db, &id).await?)
}

/// 커버에서 보일 세로 위치 (0 위 ~ 100 아래)
#[tauri::command]
#[specta::specta]
pub async fn set_character_cover_position(state: State<'_, AppState>, id: String, y: f64) -> Result<()> {
    conversation::set_character_cover_y(&state.db, &id, y).await?;
    Ok(())
}

/// 이미지 파일을 아바타 폴더로 복사하고 새 경로를 돌려준다
pub(crate) fn copy_avatar(dir: &std::path::Path, src: &str, prefix: &str) -> Result<String> {
    let src = std::path::PathBuf::from(src);
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif"))
        .ok_or_else(|| AppError::validation("png, jpg, webp, gif 이미지만 쓸 수 있습니다", None))?;
    std::fs::create_dir_all(dir).map_err(|e| AppError::io(e.to_string()))?;
    let dest = dir.join(format!("{prefix}-{}.{ext}", uuid::Uuid::new_v4()));
    std::fs::copy(&src, &dest).map_err(|e| AppError::io(format!("이미지를 복사할 수 없습니다: {e}")))?;
    Ok(dest.to_string_lossy().to_string())
}

/// 앱 아바타 폴더 안의 파일이고, 다른 캐릭터·페르소나가 쓰지 않으면 지운다
async fn remove_unused_avatar(state: &AppState, dir: &std::path::Path, avatar: Option<&str>) {
    let Some(a) = avatar else { return };
    if !std::path::Path::new(a).starts_with(dir) {
        return;
    }
    let used: i64 = sqlx::query_scalar("SELECT (SELECT COUNT(*) FROM characters WHERE avatar = ?1) + (SELECT COUNT(*) FROM personas WHERE avatar = ?1)")
        .bind(a)
        .fetch_one(&state.db)
        .await
        .unwrap_or(1);
    if used == 0 {
        let _ = std::fs::remove_file(a);
    }
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_favorite_character(state: State<'_, AppState>, id: String, is_favorite: bool) -> Result<()> {
    conversation::update_character_is_favorite(&state.db, &id, is_favorite).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_character(app: tauri::AppHandle, state: State<'_, AppState>, id: String) -> Result<TrashItem> {
    // 휴지통으로: 이미지 행(CASCADE)과 채팅 이미지 폴더도 같이 옮겨 두고, 되살리면 돌아온다 (2-A-3)
    move_to_trash(&app, &state, TrashKind::Character, &id).await
}

// =======================================
// Persona
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn get_all_personas(state: State<'_, AppState>) -> Result<Vec<Persona>> {
    Ok(conversation::get_all_personas(&state.db).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_persona(state: State<'_, AppState>, input: PersonaInput) -> Result<Persona> {
    let books = input.world_info_ids.clone();
    let mut tx = state.db.begin().await?;
    let persona = conversation::create_persona(&mut *tx, input).await?;
    conversation::replace_persona_worldinfo(&mut tx, &persona.id, &books).await?;
    tx.commit().await?;
    Ok(persona)
}

#[tauri::command]
#[specta::specta]
pub async fn update_persona(state: State<'_, AppState>, id: String, input: PersonaInput) -> Result<()> {
    let exists = conversation::persona_exists(&state.db, &id).await?;
    if !exists {
        return Err(AppError::not_found(format!("페르소나를 찾을 수 없습니다: {id}")));
    }
    let books = input.world_info_ids.clone();
    let mut tx = state.db.begin().await?;
    conversation::update_persona(&mut *tx, &id, input).await?;
    conversation::replace_persona_worldinfo(&mut tx, &id, &books).await?;
    tx.commit().await?;
    Ok(())
}

/// 페르소나 아바타: 고른 이미지를 앱 데이터(백업에 들어가는 characters/avatars)로 복사해 둔다.
/// `path`가 없으면 아바타를 지운다.
#[tauri::command]
#[specta::specta]
pub async fn set_persona_avatar(app: tauri::AppHandle, state: State<'_, AppState>, id: String, path: Option<String>) -> Result<Persona> {
    let old = conversation::get_persona(&state.db, &id)
        .await
        .map_err(|_| AppError::not_found(format!("페르소나를 찾을 수 없습니다: {id}")))?;
    let base = crate::commands::character_image::app_data_dir(&app)?;
    let dir = base.join("characters").join("avatars");

    let avatar = match path {
        Some(src) => Some(copy_avatar(&dir, &src, "persona")?),
        None => None,
    };
    conversation::set_persona_avatar(&state.db, &id, avatar.as_deref()).await?;
    remove_owned_avatar(&dir, old.avatar.as_deref());
    Ok(conversation::get_persona(&state.db, &id).await?)
}

/// 앱이 복사해 둔 아바타 파일만 지운다 (사용자가 가진 원본 경로는 건드리지 않음)
fn remove_owned_avatar(dir: &std::path::Path, avatar: Option<&str>) {
    let Some(p) = avatar.map(std::path::Path::new) else { return };
    if p.starts_with(dir) && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("persona-")) {
        let _ = std::fs::remove_file(p);
    }
}

#[tauri::command]
#[specta::specta]
pub async fn delete_persona(app: tauri::AppHandle, state: State<'_, AppState>, id: String) -> Result<TrashItem> {
    if conversation::get_system_default_persona(&state.db).await.ok().as_deref() == Some(id.as_str()) {
        return Err(AppError::validation(
            "기본 페르소나는 지울 수 없습니다. 다른 페르소나를 기본으로 정한 뒤 지우세요",
            None,
        ));
    }
    // 휴지통으로: 앱이 복사해 둔 아바타 파일도 같이 옮긴다 (2-A-3)
    move_to_trash(&app, &state, TrashKind::Persona, &id).await
}

// 삭제 전 영향 범위를 프론트에 알려주기 위한 커맨드 (선택)
#[tauri::command]
#[specta::specta]
pub async fn get_persona_usage(state: State<'_, AppState>, id: String) -> Result<PersonaUsage> {
    let usage = conversation::get_persona_usage(&state.db, &id).await?;
    Ok(usage)
}

// =======================================
// Message
// =======================================

#[tauri::command]
#[specta::specta]
pub async fn get_messages(state: State<'_, AppState>, conversation_id: String, limit: i64, before: Option<i64>) -> Result<Vec<Message>> {
    let rows = conversation::get_messages(&state.db, &conversation_id, limit, before).await?;
    Ok(rows)
}

#[tauri::command]
#[specta::specta]
pub async fn append_message(state: State<'_, AppState>, settings: State<'_, SettingsState<AppSetting>>, mut input: MessageInput) -> Result<Message> {
    if input.content.is_empty() && input.role == "user" {
        return Err(AppError::validation("빈 메시지는 전송할 수 없습니다", None));
    }
    // 사용자 입력은 저장 전에 정규식(user_input / stored) 적용
    if input.role == "user" {
        let preset_id = crate::commands::regex::selected_preset(&settings);
        let regex = RegexProcessor::for_conversation(&state.db, &input.conversation_id, preset_id.as_deref()).await?;
        input.content = regex.apply(&input.content, RegexPlacement::UserInput, &[RegexTarget::Stored], Some(0));
    }
    let conversation_id = input.conversation_id.clone();
    let mes = conversation::append_message(&state.db, input).await?;
    // 메시지 업데이트 시간 업데이트
    conversation::touch_conversation(&state.db, &conversation_id).await?;
    Ok(mes)
}

/// 메시지가 바뀐 뒤 메모리 색인을 맞춘다 (플러그인이 꺼져 있거나 색인 파일이 없으면 아무것도 하지 않음, 백그라운드)
async fn memory_changed(app: &tauri::AppHandle, state: &AppState, message_ids: &[String]) {
    let convs = crate::service::memory::maintain::conversations_of(&state.db, message_ids).await;
    crate::service::memory::maintain::after_message_change(app, &state.db, convs);
}

#[tauri::command]
#[specta::specta]
pub async fn update_message(app: tauri::AppHandle, state: State<'_, AppState>, id: String, content: String, meta: Option<serde_json::Value>) -> Result<()> {
    let result = conversation::update_message(&state.db, &id, &content, &meta).await?;
    if !result {
        return Err(AppError::not_found(format!("메시지를 찾을 수 없습니다: {id}")));
    }
    memory_changed(&app, &state, &[id]).await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_message_swipe_id(app: tauri::AppHandle, state: State<'_, AppState>, id: String, swipe_id: i64) -> Result<()> {
    let mut meta = conversation::get_message_meta(&state.db, &id).await?.unwrap_or_default();
    if let Some(obj) = meta.as_object_mut() {
        obj.insert("swipeId".to_string(), serde_json::json!(swipe_id));
    }
    // 업데이트
    conversation::update_message_meta(&state.db, &id, meta).await?;
    memory_changed(&app, &state, &[id]).await;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_message_content(app: tauri::AppHandle, state: State<'_, AppState>, id: String, content: String) -> Result<()> {
    let result = conversation::update_message_content(&state.db, &id, &content).await?;
    if !result {
        return Err(AppError::not_found(format!("메시지를 찾을 수 없습니다: {id}")));
    }
    memory_changed(&app, &state, &[id]).await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_message_meta(app: tauri::AppHandle, state: State<'_, AppState>, id: String, meta: serde_json::Value) -> Result<()> {
    let result = conversation::update_message_meta(&state.db, &id, meta).await?;
    if !result {
        return Err(AppError::not_found(format!("메시지를 찾을 수 없습니다: {id}")));
    }
    memory_changed(&app, &state, &[id]).await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_message(app: tauri::AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    // 지우기 전에 어느 대화인지 알아 둔다
    let convs = crate::service::memory::maintain::conversations_of(&state.db, std::slice::from_ref(&id)).await;
    let result = conversation::delete_message(&state.db, &id).await?;
    if !result {
        return Err(AppError::not_found(format!("메시지를 찾을 수 없습니다: {id}")));
    }
    crate::service::memory::maintain::after_message_change(&app, &state.db, convs);
    Ok(())
}
// =======================================
// WorldInfo
// =======================================

#[tauri::command]
#[specta::specta]
pub async fn get_all_worldinfo_books(state: State<'_, AppState>) -> Result<Vec<WorldInfoBook>> {
    Ok(conversation::get_all_worldinfo_books(&state.db).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_worldinfo_book(state: State<'_, AppState>, input: WorldInfoBookInput) -> Result<WorldInfoBook> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("월드인포 이름은 비어있을 수 없습니다", None));
    }

    let result = conversation::create_worldinfo_book(&state.db, input).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_worldinfo_book(app: tauri::AppHandle, state: State<'_, AppState>, id: String) -> Result<TrashItem> {
    move_to_trash(&app, &state, TrashKind::WorldinfoBook, &id).await
}

#[tauri::command]
#[specta::specta]
pub async fn get_worldinfo_entries(state: State<'_, AppState>, book_id: String) -> Result<Vec<WorldInfoEntry>> {
    let result = conversation::get_all_worldinfo_entries(&state.db, &book_id).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn upsert_worldinfo_entry(state: State<'_, AppState>, input: WorldInfoEntryInput) -> Result<WorldInfoEntry> {
    if input.keys.is_empty() && !input.constant {
        return Err(AppError::validation("상시 항목이 아니면 트리거 키워드가 최소 1개 필요합니다", None));
    }

    let result = conversation::upsert_worldinfo_entry(&state.db, input).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_worldinfo_entry(state: State<'_, AppState>, id: String) -> Result<()> {
    let result = conversation::delete_worldinfo_entry(&state.db, &id).await?;
    if !result {
        return Err(AppError::not_found(format!("월드인포 항목을 찾을 수 없습니다: {id}")));
    }
    Ok(())
}

// =======================================
// 로어북 화면
// =======================================

/// 책 이름·설명 바꾸기
#[tauri::command]
#[specta::specta]
pub async fn update_worldinfo_book(state: State<'_, AppState>, id: String, input: WorldInfoBookInput) -> Result<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("로어북 이름은 비어있을 수 없습니다", None));
    }
    if !conversation::update_worldinfo_book(&state.db, &id, &input).await? {
        return Err(AppError::not_found(format!("로어북을 찾을 수 없습니다: {id}")));
    }
    Ok(())
}

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfoBookCount {
    pub book_id: String,
    pub entries: u32,
    pub enabled: u32,
}

/// 책마다 항목 수 (목록 화면)
#[tauri::command]
#[specta::specta]
pub async fn worldinfo_entry_counts(state: State<'_, AppState>) -> Result<Vec<WorldInfoBookCount>> {
    Ok(conversation::worldinfo_entry_counts(&state.db)
        .await?
        .into_iter()
        .map(|(book_id, n, on)| WorldInfoBookCount {
            book_id,
            entries: n.max(0) as u32,
            enabled: on.max(0) as u32,
        })
        .collect())
}

/// 대화에 직접 붙인 로어북 바꾸기 (캐릭터·페르소나의 책은 그대로)
#[tauri::command]
#[specta::specta]
pub async fn set_conversation_worldinfo(state: State<'_, AppState>, id: String, book_ids: Vec<String>) -> Result<Conversation> {
    if !conversation::set_conversation_world_info(&state.db, &id, &book_ids).await? {
        return Err(AppError::not_found(format!("대화를 찾을 수 없습니다: {id}")));
    }
    Ok(conversation::get_conversation(&state.db, &id).await?)
}

/// 책 하나를 내보내기. format: "json"(SillyTavern 월드인포) | "txt"(읽기용 글). 내보낸 항목 수를 돌려준다
#[tauri::command]
#[specta::specta]
pub async fn export_worldinfo_book(state: State<'_, AppState>, id: String, format: String, path: String) -> Result<u32> {
    let entries = conversation::get_all_worldinfo_entries(&state.db, &id).await?;
    let out = match format.as_str() {
        "json" => sillytavern_book_json(&entries).to_string(),
        "txt" => {
            let name = conversation::get_all_worldinfo_books(&state.db)
                .await?
                .into_iter()
                .find(|b| b.id == id)
                .map(|b| b.name)
                .unwrap_or_default();
            let mut s = format!(
                "{name}

"
            );
            for e in &entries {
                let title = if e.comment.trim().is_empty() { e.keys.join(", ") } else { e.comment.clone() };
                let keys = if e.constant { "항상 넣음".to_string() } else { e.keys.join(", ") };
                s.push_str(&format!(
                    "## {title}{}
키워드: {keys}

{}

",
                    if e.enabled { "" } else { " (꺼짐)" },
                    e.content.trim()
                ));
            }
            s
        }
        _ => return Err(AppError::validation(format!("모르는 형식: {format}"), None)),
    };
    std::fs::write(&path, out).map_err(|e| AppError::io(format!("파일을 쓸 수 없습니다: {e}")))?;
    Ok(entries.len() as u32)
}

/// SillyTavern 월드인포 형식 { entries: { "0": {...}, ... } }
pub(crate) fn sillytavern_book_json(entries: &[WorldInfoEntry]) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            (
                i.to_string(),
                json!({
                    "uid": i,
                    "key": e.keys,
                    "keysecondary": e.secondary_keys,
                    "selective": !e.secondary_keys.is_empty(),
                    "comment": e.comment,
                    "content": e.content,
                    "constant": e.constant,
                    "disable": !e.enabled,
                    "order": e.priority,
                    "position": 0,
                    // 샌드캐슬 분류 (SillyTavern은 모르는 칸을 그대로 둔다 — 다시 가져오면 되살아남, 5-F-1)
                    "extensions": { "sandcastle_category": e.category },
                }),
            )
        })
        .collect();
    json!({ "entries": map })
}

#[cfg(test)]
mod lore_tests {
    use super::*;

    #[test]
    fn exported_book_reimports_with_titles() {
        let entry = WorldInfoEntry {
            id: "e".into(),
            book_id: "b".into(),
            keys: vec!["dragon".into()],
            secondary_keys: vec!["cave".into()],
            content: "Dragons sleep in caves.".into(),
            enabled: false,
            priority: 42,
            constant: false,
            comment: "용".into(),
            category: "캐릭터".into(),
        };
        let v = sillytavern_book_json(&[entry]);
        assert_eq!(v["entries"]["0"]["comment"], "용");
        let mut w = vec![];
        let back = crate::service::import::sillytavern::worldinfo::map_entries(&v, &mut w);
        assert_eq!(back[0].category, "캐릭터", "SillyTavern 왕복에도 분류가 남음");
        assert_eq!(v["entries"]["0"]["disable"], true);
        assert_eq!(v["entries"]["0"]["selective"], true);
        assert_eq!(v["entries"]["0"]["order"], 42);
    }
}
