use crate::{
    db,
    error::AppError,
    models::{
        ai_service::{ChatContext, GenerationType, HistoryItem, PromptRequest},
        settings::AppSetting,
        widget::{
            MascotCharacterConfig, MascotChatRequest, MascotPeerChatRequest, MascotPeerChatTurn, MusicMetaInput, Playlist, PlaylistInput, PlaylistItem,
            WidgetMascotSetting, WidgetMusicSetting,
        },
    },
    service::{
        ai::{AiRequest, AiService},
        chat::phone as phone_sides,
        prompt::PromptManager,
    },
    state::AppState,
    sys::{
        library::LibraryRoot,
        music::{self},
        settings::{merge_json, SettingsState},
    },
    widget::{manager::WidgetManager, InteractiveRegion, MouseTrackerState, WidgetInteractionState},
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_autostart::AutoLaunchManager;
use tauri_plugin_store::StoreExt;
use tokio_util::sync::CancellationToken;

// =======================================
// 위젯 윈도우 열기
// =======================================
#[tauri::command]
#[specta::specta]
pub async fn open_widget(app: AppHandle, state: State<'_, MouseTrackerState>) -> Result<(), AppError> {
    // 하위 기능이 모두 꺼져 있으면 빈 창을 띄우지 않는다
    let wanted = app
        .try_state::<crate::sys::settings::SettingsState<crate::models::settings::FeatureSetting>>()
        .and_then(|s| s.0.lock().ok().map(|f| f.widget_window_wanted()))
        .unwrap_or(true);
    if wanted {
        WidgetManager::open_widget(&app, &state)?;
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn close_widget(app: AppHandle, state: State<'_, MouseTrackerState>) -> Result<(), AppError> {
    WidgetManager::close_widget(&app, &state)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_widget(app: AppHandle, state: State<'_, MouseTrackerState>) -> Result<(), AppError> {
    WidgetManager::toggle_widget(&app, &state)?;
    Ok(())
}

// ignore 값을 받아서 해당 앱 윈도우를 무시하게 하거나 클릭될 수 있게 하는 함수
#[tauri::command]
#[specta::specta]
pub async fn toggle_ignore_cursor(window: tauri::Window, ignore: bool) -> Result<(), AppError> {
    window.set_ignore_cursor_events(ignore)?;
    Ok(())
}

// =======================================
// 자동시작 설정
// =======================================

#[tauri::command]
#[specta::specta]
pub async fn is_autostart_enabled(state: State<'_, AutoLaunchManager>) -> Result<bool, AppError> {
    state.is_enabled().map_err(|e| AppError::window(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn set_autostart(state: State<'_, AutoLaunchManager>, enabled: bool) -> Result<(), AppError> {
    if enabled {
        state.enable().map_err(|e| AppError::io(e.to_string()))?;
    } else {
        state.disable().map_err(|e| AppError::io(e.to_string()))?;
    }

    Ok(())
}

// =======================================
// 클릭스루
// =======================================
#[tauri::command]
#[specta::specta]
pub fn update_interactive_regions(upserts: Vec<InteractiveRegion>, removed: Vec<String>, state: State<'_, WidgetInteractionState>) -> Result<(), AppError> {
    let mut regions = state.regions.lock().unwrap();

    for region in upserts {
        regions.insert(region.id.clone(), region);
    }
    for id in removed {
        regions.remove(&id);
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn remove_interactive_regions(ids: Vec<String>, state: State<WidgetInteractionState>) -> Result<(), AppError> {
    let mut regions = state.regions.lock().unwrap();

    for id in ids {
        regions.remove(&id);
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_widget_click_through(window: WebviewWindow, enabled: bool) -> Result<(), AppError> {
    window.set_ignore_cursor_events(enabled)?;
    Ok(())
}

// =======================================
// 음악 플레이어
// =======================================
#[tauri::command]
#[specta::specta]
pub fn get_widget_music_settings(app: AppHandle) -> Result<WidgetMusicSetting, AppError> {
    let store = app.store("widget.json").map_err(|e| AppError::io(e.to_string()))?;
    let result: WidgetMusicSetting = store.get("music").and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();

    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn save_widget_music_settings(app: AppHandle, patch: serde_json::Value) -> Result<(), AppError> {
    let store = app.store("widget.json").map_err(|e| AppError::io(e.to_string()))?;
    let setting: WidgetMusicSetting = store.get("music").and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    let mut data: serde_json::Value = serde_json::to_value(&setting)?;

    // 이전 데이터와 통합
    merge_json(&mut data, &patch);

    store.set("music", serde_json::to_value(&data).unwrap());
    // 설정 화면과 위젯 창이 서로의 변경을 따라가게 (5-E-21)
    let _ = tauri::Emitter::emit(&app, "widget_music::changed", &data);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_select_folder_music_data(app: AppHandle, state: State<'_, AppState>, folder_path: String) -> Result<LibraryRoot, AppError> {
    let cache_dir = app.path().app_cache_dir()?;
    // 에셋 프로토콜 범위는 앱 폴더로 좁혀 두었으므로(tauri.conf.json), 사용자가 고른 음악 폴더만 실행 중에 연다
    app.asset_protocol_scope()
        .allow_directory(&folder_path, true)
        .map_err(|e| AppError::io(format!("음악 폴더를 열 수 없습니다: {e}")))?;
    let result = music::folder_indexing(&state.db, folder_path.clone(), &cache_dir).await?;
    Ok(result)
}

// ---------------------------------------------------------
//  플레이 리스트
// ---------------------------------------------------------
#[tauri::command]
#[specta::specta]
pub async fn get_all_playlist(state: State<'_, AppState>) -> Result<Vec<Playlist>, AppError> {
    let result = db::widget::get_all_playlist(&state.db).await?;
    Ok(result)
}
#[tauri::command]
#[specta::specta]
pub async fn create_playlist(state: State<'_, AppState>, input: PlaylistInput) -> Result<Playlist, AppError> {
    let result = db::widget::create_playlist(&state.db, input).await?;
    Ok(result)
}
#[tauri::command]
#[specta::specta]
pub async fn update_playlist(state: State<'_, AppState>, id: String, input: PlaylistInput) -> Result<(), AppError> {
    db::widget::update_playlist(&state.db, &id, input).await?;
    Ok(())
}
#[tauri::command]
#[specta::specta]
pub async fn delete_playlist(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    db::widget::delete_playlist(&state.db, &id).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_playlist_entry(state: State<'_, AppState>, playlist_id: String) -> Result<Vec<PlaylistItem>, AppError> {
    let result = db::widget::get_all_playlist_entry(&state.db, &playlist_id).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn append_playlist_entry(state: State<'_, AppState>, playlist_id: String, music_src: String, sort_order: Option<u32>) -> Result<(), AppError> {
    db::widget::append_playlist_entry(&state.db, &playlist_id, &music_src, sort_order).await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn remove_playlist_entry(state: State<'_, AppState>, playlist_id: String, music_src: String) -> Result<(), AppError> {
    db::widget::remove_playlist_entry(&state.db, &playlist_id, &music_src).await?;
    Ok(())
}

// ---------------------------------------------------------
//  음악 데이터 업데이트
// ---------------------------------------------------------
#[tauri::command]
#[specta::specta]
pub async fn update_music_meta(state: State<'_, AppState>, input: MusicMetaInput) -> Result<(), AppError> {
    db::widget::upsert_music_meta(&state.db, input).await?;
    Ok(())
}
#[tauri::command]
#[specta::specta]
pub async fn update_playlist_music_order(state: State<'_, AppState>, playlist_id: String, input: Vec<PlaylistItem>) -> Result<(), AppError> {
    for item in input {
        db::widget::update_playlist_music_order(&state.db, &playlist_id, &item.meta.path, item.sort_order).await?;
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_music_album_art_pos(state: State<'_, AppState>, path: String, pos: serde_json::Value) -> Result<(), AppError> {
    db::widget::upsert_music_album_art_pos(&state.db, &path, pos).await?;
    Ok(())
}

// ---------------------------------------------------------
//  파일 접근 또는 변경
// ---------------------------------------------------------
#[tauri::command]
#[specta::specta]
pub async fn set_album_art(app: AppHandle, file_path: String, img_path: String) -> Result<String, AppError> {
    let cache_dir = app.path().app_cache_dir()?;
    music::set_album_art(file_path, img_path, &cache_dir).await
}

#[tauri::command]
#[specta::specta]
pub async fn filter_existing_paths(paths: Vec<String>) -> Result<Vec<String>, AppError> {
    let existing = paths
        .into_iter()
        .filter(|p| Path::new(p).is_file()) // exists() 대신 is_file()로 폴더는 제외
        .collect();
    Ok(existing)
}

// =======================================
// 시메지 (마스코트)
// =======================================

const MASCOT_IMAGE_MAX: u64 = 8 * 1024 * 1024;

fn mascot_images_root(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(app.path().app_data_dir()?.join("widget").join("mascot"))
}

fn load_mascot_setting(app: &AppHandle) -> Result<WidgetMascotSetting, AppError> {
    let store = app.store("widget.json").map_err(|e| AppError::io(e.to_string()))?;
    let setting: WidgetMascotSetting = store.get("mascot").and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    Ok(setting)
}

#[tauri::command]
#[specta::specta]
pub fn get_widget_mascot_settings(app: AppHandle) -> Result<WidgetMascotSetting, AppError> {
    load_mascot_setting(&app)
}

#[tauri::command]
#[specta::specta]
pub fn save_widget_mascot_settings(app: AppHandle, setting: WidgetMascotSetting) -> Result<WidgetMascotSetting, AppError> {
    let store = app.store("widget.json").map_err(|e| AppError::io(e.to_string()))?;
    let val = serde_json::to_value(&setting)?;
    store.set("mascot", val);
    let _ = tauri::Emitter::emit(&app, "widget_mascot::changed", &setting);
    Ok(setting)
}

#[tauri::command]
#[specta::specta]
pub async fn upload_widget_mascot_image(app: AppHandle, mascot_id: String, slot: String, source_path: String) -> Result<String, AppError> {
    let src = Path::new(&source_path);
    let ext = src.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    if !["png", "jpg", "jpeg", "webp", "gif", "avif"].contains(&ext.as_str()) {
        return Err(AppError::validation("이미지 파일(png·jpg·webp·gif·avif)만 올릴 수 있습니다", None));
    }
    let meta = std::fs::metadata(src)?;
    if meta.len() > MASCOT_IMAGE_MAX {
        return Err(AppError::validation("이미지 파일이 너무 큽니다 (8MB까지)", None));
    }

    let safe_id: String = mascot_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect();
    let safe_slot: String = slot
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(32)
        .collect();
    if safe_id.is_empty() || safe_slot.is_empty() {
        return Err(AppError::validation("잘못된 시메지 ID 또는 슬롯입니다", None));
    }

    let dir = mascot_images_root(&app)?.join(&safe_id);
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(format!("{safe_slot}_{}.{ext}", uuid::Uuid::new_v4()));
    std::fs::copy(src, &dest)?;
    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_widget_mascot_image(app: AppHandle, path: String) -> Result<(), AppError> {
    let root = mascot_images_root(&app)?;
    let target = PathBuf::from(&path);
    if target.starts_with(&root) && target.is_file() {
        let _ = std::fs::remove_file(&target);
    }
    Ok(())
}

/// 시메지 설정과 앱 기본값에서 API 프로필·모델·프리셋을 결정한다
fn resolve_mascot_ai(mascot: &MascotCharacterConfig, app_setting: &AppSetting) -> Result<(String, Option<String>, Option<String>), AppError> {
    let profile_id = mascot
        .api_profile_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| app_setting.selected_api_profile.clone().filter(|s| !s.trim().is_empty()))
        .ok_or_else(|| AppError::validation("시메지에 연결된 API가 없습니다. 설정에서 API를 선택해 주세요.", None))?;

    let model = if mascot.api_profile_id.as_deref().map(str::trim).is_some_and(|s| !s.is_empty()) {
        mascot.model.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
    } else {
        mascot
            .model
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| app_setting.selected_model.clone().filter(|s| !s.trim().is_empty()))
    };

    let preset_id = mascot
        .preset_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            let p = app_setting.selected_preset.trim();
            (!p.is_empty()).then(|| p.to_string())
        });

    Ok((profile_id, model, preset_id))
}

async fn load_mascot_char_info(db: &db::conn::DbPool, mascot: &MascotCharacterConfig) -> (String, String, Option<String>, Option<String>) {
    if let Some(cid) = mascot.character_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if let Ok(ch) = db::conversation::get_character(db, cid).await {
            return (
                ch.name,
                ch.description,
                ch.example_dialogue,
                ch.system_prompt_override.filter(|s| !s.trim().is_empty()),
            );
        }
    }
    (mascot.name.clone(), String::new(), None, None)
}

async fn load_mascot_persona_info(db: &db::conn::DbPool, mascot: &MascotCharacterConfig, app_setting: &AppSetting) -> (String, String) {
    let pid = mascot
        .persona_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(app_setting.selected_persona_id.as_str());
    if !pid.is_empty() {
        if let Ok(p) = db::conversation::get_persona(db, pid).await {
            return (p.name, p.description);
        }
    }
    ("User".into(), String::new())
}

/// 시메지 AI 대사 생성 (유저와의 짧은 대화, 선물 반응 등)
#[tauri::command]
#[specta::specta]
pub async fn mascot_generate_reply(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    settings: State<'_, SettingsState<AppSetting>>,
    request: MascotChatRequest,
) -> Result<String, AppError> {
    let mascot_setting = load_mascot_setting(&app)?;
    let mascot = mascot_setting
        .characters
        .iter()
        .find(|c| c.id == request.mascot_id)
        .ok_or_else(|| AppError::not_found("시메지를 찾을 수 없습니다"))?
        .clone();

    let app_setting = settings
        .0
        .lock()
        .map_err(|_| AppError::internal("settings lock poisoned"))?
        .clone();

    let (profile_id, model, preset_id) = resolve_mascot_ai(&mascot, &app_setting)?;
    let (char_name, char_desc, example_dialogue, main_prompt) = load_mascot_char_info(&state.db, &mascot).await;
    let (user_name, persona_desc) = load_mascot_persona_info(&state.db, &mascot, &app_setting).await;

    let event_hint = match request.event_kind.as_deref() {
        Some("gift") => "The user just gave you a gift on the desktop.",
        Some("long_absence") => "The user has not visited for over 7 days and just came back.",
        Some("greeting") => "You just appeared on the user's desktop to greet them.",
        Some("notify") => "A desktop notification just arrived for the user.",
        _ => "You are a cute desktop mascot (shimeji) chatting briefly with the user.",
    };

    let system_rule = format!(
        "[Desktop Mascot Mode]\n{event_hint}\n\
         - Reply as {char_name} in 1 to 2 very short, natural sentences (fits inside a small speech bubble, under 80 characters if possible).\n\
         - Stay in character. Output ONLY the spoken line without quotation marks, narration, or stage directions.\n\
         - Respond in the language of the user or character profile."
    );

    let history = request
        .history
        .into_iter()
        .rev()
        .take(10)
        .rev()
        .map(|h| HistoryItem {
            role: if h.role == "assistant" { "assistant".into() } else { "user".into() },
            content: h.content,
            images: vec![],
            source: None,
        })
        .collect();

    let ctx = ChatContext {
        system_rules: vec![system_rule],
        char_name: Some(char_name),
        user_name: Some(user_name),
        character_info: (!char_desc.trim().is_empty()).then_some(char_desc),
        persona_info: persona_desc,
        example_dialogue,
        main_prompt,
        history,
        ..Default::default()
    };

    let mut prompt = PromptManager
        .build_with_db(
            &state.db,
            PromptRequest {
                context: ctx,
                user_message: request.user_message,
                preset_id,
                context_limit: crate::service::prompt::prompt_manager::DEFAULT_CONTEXT_LIMIT,
                world_info_ratio: 0.2,
                generation_type: GenerationType::Normal,
            },
        )
        .await?;
    prompt.chat_options.stream = false;
    prompt.chat_options.max_tokens = Some(prompt.chat_options.max_tokens.unwrap_or(200).min(240));

    let cancel = CancellationToken::new();
    let resp = ai
        .generate(
            AiRequest {
                profile_id,
                prompt,
                model,
            },
            &cancel,
            |_| {},
        )
        .await?;

    Ok(resp.content.trim().trim_matches(['"', '「', '」']).to_string())
}

/// 두 시메지 간 짧은 대화 생성 (AI)
#[tauri::command]
#[specta::specta]
pub async fn mascot_generate_peer_chat(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    settings: State<'_, SettingsState<AppSetting>>,
    request: MascotPeerChatRequest,
) -> Result<Vec<MascotPeerChatTurn>, AppError> {
    let mascot_setting = load_mascot_setting(&app)?;
    let mascot_a = mascot_setting
        .characters
        .iter()
        .find(|c| c.id == request.mascot_a_id)
        .ok_or_else(|| AppError::not_found("첫 번째 시메지를 찾을 수 없습니다"))?
        .clone();
    let mascot_b = mascot_setting
        .characters
        .iter()
        .find(|c| c.id == request.mascot_b_id)
        .ok_or_else(|| AppError::not_found("두 번째 시메지를 찾을 수 없습니다"))?
        .clone();

    let app_setting = settings
        .0
        .lock()
        .map_err(|_| AppError::internal("settings lock poisoned"))?
        .clone();

    // A의 API 우선, 없으면 B의 API, 없으면 앱 기본 API
    let (profile_id, model, preset_id) = resolve_mascot_ai(&mascot_a, &app_setting).or_else(|_| resolve_mascot_ai(&mascot_b, &app_setting))?;

    let (name_a, desc_a, _, _) = load_mascot_char_info(&state.db, &mascot_a).await;
    let (name_b, desc_b, _, _) = load_mascot_char_info(&state.db, &mascot_b).await;
    let (user_name, _) = load_mascot_persona_info(&state.db, &mascot_a, &app_setting).await;

    let display_a = if name_a.trim().is_empty() { mascot_a.name.clone() } else { name_a };
    let display_b = if name_b.trim().is_empty() { mascot_b.name.clone() } else { name_b };
    let topic_line = request
        .topic
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|t| format!("Topic: {t}\n"))
        .unwrap_or_default();

    let system_rule = format!(
        "[Desktop Mascots Banter]\n\
         Two desktop mascots ({display_a} and {display_b}) met on {user_name}'s screen and have a short, cute conversation.\n\
         {topic_line}\
         - Write 2 to 4 short lines alternating between {display_a} and {display_b}.\n\
         - Each line MUST be formatted strictly as `{display_a}: message` or `{display_b}: message`.\n\
         - Keep each message under 60 characters so it fits a small speech bubble.\n\
         - No narration, action descriptions, or extra commentary. Write in Korean unless the character profiles are in another language.\n\n\
         [{display_a}]\n{desc_a}\n\n\
         [{display_b}]\n{desc_b}"
    );

    let ctx = ChatContext {
        system_rules: vec![system_rule],
        char_name: Some(display_a.clone()),
        user_name: Some(user_name),
        ..Default::default()
    };

    let mut prompt = PromptManager
        .build_with_db(
            &state.db,
            PromptRequest {
                context: ctx,
                user_message: format!("{display_a}와 {display_b}가 화면에서 마주쳐 짧은 대화를 나눈다."),
                preset_id,
                context_limit: crate::service::prompt::prompt_manager::DEFAULT_CONTEXT_LIMIT,
                world_info_ratio: 0.1,
                generation_type: GenerationType::Quiet,
            },
        )
        .await?;
    prompt.chat_options.stream = false;
    prompt.chat_options.max_tokens = Some(prompt.chat_options.max_tokens.unwrap_or(260).min(320));

    let cancel = CancellationToken::new();
    let resp = ai
        .generate(
            AiRequest {
                profile_id,
                prompt,
                model,
            },
            &cancel,
            |_| {},
        )
        .await?;

    let parsed = phone_sides::parse_peek(&resp.content, &display_a, &display_b);
    let turns = parsed
        .into_iter()
        .take(4)
        .map(|(is_a, text)| {
            if is_a {
                MascotPeerChatTurn {
                    mascot_id: mascot_a.id.clone(),
                    name: mascot_a.name.clone(),
                    text: text.trim_matches(['"', '「', '」']).to_string(),
                }
            } else {
                MascotPeerChatTurn {
                    mascot_id: mascot_b.id.clone(),
                    name: mascot_b.name.clone(),
                    text: text.trim_matches(['"', '「', '」']).to_string(),
                }
            }
        })
        .filter(|t| !t.text.trim().is_empty())
        .collect();

    Ok(turns)
}
