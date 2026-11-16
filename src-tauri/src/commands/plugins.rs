//! 코어 플러그인 커맨드: 설정, 번역, 음성 읽기, 이미지 생성. 기능이 꺼져 있으면 모두 거절한다.
use std::sync::Arc;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{
    commands::character_image::app_data_dir,
    db::{chat_ui, conversation, translation},
    error::{AppError, Result},
    models::{
        conversation::Message,
        settings::{AppSetting, PluginSetting, SettingsDomain},
    },
    service::{ai::AiService, plugins as tools},
    state::AppState,
    sys::{
        plugins::{id, require},
        settings::{save, SettingsState},
    },
};

/// 만든 그림이 모이는 폴더 (갤러리)
fn generated_dir(app: &AppHandle) -> Result<std::path::PathBuf> {
    Ok(app_data_dir(app)?.join("generated"))
}

fn read<T: Clone>(state: &SettingsState<T>) -> Result<T> {
    Ok(state.0.lock().map_err(|_| AppError::internal("settings lock poisoned"))?.clone())
}

/// 대화에 고른 API, 없으면 앱에서 고른 API
fn fallback_profile(app: &AppSetting, conv_profile: Option<&str>) -> String {
    conv_profile
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| app.selected_api_profile.clone())
        .unwrap_or_default()
}

// ---------------------------------------------------------
// 설정
// ---------------------------------------------------------

#[tauri::command]
#[specta::specta]
pub fn get_plugin_setting(state: State<'_, SettingsState<PluginSetting>>) -> Result<PluginSetting> {
    read(&state)
}

#[tauri::command]
#[specta::specta]
pub fn update_plugin_setting(app: AppHandle, state: State<'_, SettingsState<PluginSetting>>, setting: PluginSetting) -> Result<PluginSetting> {
    {
        let mut cur = state.0.lock().map_err(|_| AppError::internal("settings lock poisoned"))?;
        *cur = setting.clone();
        save(&app, &*cur)?;
    }
    let _ = app.emit(PluginSetting::EVENT_NAME, &setting);
    Ok(setting)
}

// ---------------------------------------------------------
// 번역
// ---------------------------------------------------------

/// 글 번역. to를 비우면 설정의 답변 언어. (입력창의 "보내기 전에 번역"은 to에 입력 언어를 넣어 부른다)
#[tauri::command]
#[specta::specta]
pub async fn translate_text(
    app: AppHandle,
    ai: State<'_, Arc<AiService>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    settings: State<'_, SettingsState<AppSetting>>,
    text: String,
    to: Option<String>,
) -> Result<String> {
    require(&app, id::TRANSLATE)?;
    let p = read(&plugin)?.translate;
    let to = to.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| p.target_lang.clone());
    let db = &app.state::<AppState>().db;
    cached_translate(db, &ai, &p, &fallback_profile(&read(&settings)?, None), &text, &to).await
}

/// 공용 번역 캐시를 먼저 보고, 없을 때만 AI로 번역해 캐시에 넣는다 (같은 문장에 다시 요금을 내지 않게)
async fn cached_translate(db: &crate::db::conn::DbPool, ai: &AiService, p: &crate::models::settings::TranslateSetting, fallback: &str, text: &str, to: &str) -> Result<String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }
    if let Some(hit) = translation::get_translation(db, text, to).await? {
        return Ok(hit);
    }
    let out = tools::translate(ai, p, fallback, text, to).await?;
    translation::put_translation(db, text, to, &out).await?;
    Ok(out)
}

/// 이미 번역해 둔 것만 꺼낸다 (AI를 부르지 않음). 게임 대사 줄·지난 대사 창이 다시 열릴 때 쓴다.
/// texts와 같은 순서로, 캐시에 없으면 null
#[tauri::command]
#[specta::specta]
pub async fn get_cached_translations(app: AppHandle, state: State<'_, AppState>, plugin: State<'_, SettingsState<PluginSetting>>, texts: Vec<String>, to: Option<String>) -> Result<Vec<Option<String>>> {
    if !crate::sys::plugins::is_enabled(&app, id::TRANSLATE) {
        return Ok(vec![None; texts.len()]);
    }
    let to = to.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| read(&plugin).map(|p| p.translate.target_lang).unwrap_or_default());
    let mut out = Vec::with_capacity(texts.len());
    for t in texts.iter().take(500) {
        out.push(translation::get_translation(&state.db, t, &to).await?);
    }
    out.resize(texts.len(), None);
    Ok(out)
}

/// 메시지 번역. 결과는 meta.translation = {lang, text, source}에 저장해 다시 열어도 보인다 (원문이 바뀌면 다시 번역)
#[tauri::command]
#[specta::specta]
pub async fn translate_message(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    settings: State<'_, SettingsState<AppSetting>>,
    message_id: String,
) -> Result<Message> {
    require(&app, id::TRANSLATE)?;
    let p = read(&plugin)?.translate;
    let msg = chat_ui::get_message(&state.db, &message_id).await?;
    let conv = conversation::get_conversation(&state.db, &msg.conversation_id).await?;
    let text = cached_translate(&state.db, &ai, &p, &fallback_profile(&read(&settings)?, conv.api_profile_id.as_deref()), &msg.content, &p.target_lang).await?;
    let mut meta = msg.meta.clone().filter(Value::is_object).unwrap_or_else(|| json!({}));
    meta["translation"] = json!({ "lang": p.target_lang, "text": text, "source": msg.content });
    conversation::update_message_meta(&state.db, &message_id, meta).await?;
    Ok(chat_ui::get_message(&state.db, &message_id).await?)
}

// ---------------------------------------------------------
// 음성
// ---------------------------------------------------------

/// 글을 음성 파일(mp3)로 만들고 경로를 돌려준다 (같은 글·설정이면 캐시)
#[tauri::command]
#[specta::specta]
pub async fn speak_text(
    app: AppHandle,
    ai: State<'_, Arc<AiService>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    settings: State<'_, SettingsState<AppSetting>>,
    text: String,
) -> Result<String> {
    require(&app, id::TTS)?;
    let p = read(&plugin)?.tts;
    let cache = app.path().app_cache_dir().map_err(|e| AppError::io(e.to_string()))?;
    let path = tools::speak(&ai, &p, &fallback_profile(&read(&settings)?, None), &text, &cache).await?;
    Ok(path.to_string_lossy().into_owned())
}

// ---------------------------------------------------------
// 이미지 생성
// ---------------------------------------------------------

/// 메시지 장면을 그림으로. meta.generatedImages에 {path, prompt}를 쌓는다
#[tauri::command]
#[specta::specta]
pub async fn generate_message_image(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    settings: State<'_, SettingsState<AppSetting>>,
    message_id: String,
) -> Result<Message> {
    require(&app, id::IMAGE_GEN)?;
    let p = read(&plugin)?.image_gen;
    let msg = chat_ui::get_message(&state.db, &message_id).await?;
    let conv = conversation::get_conversation(&state.db, &msg.conversation_id).await?;
    let chat_profile = fallback_profile(&read(&settings)?, conv.api_profile_id.as_deref());
    // 외모를 알 수 있게 캐릭터 설명을 함께 준다
    let context = match conversation::get_character(&state.db, &conv.character_id).await {
        Ok(c) => format!("{}: {}", c.name, c.description.chars().take(600).collect::<String>()),
        Err(_) => String::new(),
    };
    let prompt = tools::image_prompt(&ai, &p, &chat_profile, &msg.content, &context).await?;
    let image = tools::generate_image(&ai, &p, &chat_profile, &prompt, &generated_dir(&app)?, "message", Some((&message_id, &msg.conversation_id))).await?;

    let mut meta = msg.meta.clone().filter(Value::is_object).unwrap_or_else(|| json!({}));
    let mut list = meta["generatedImages"].as_array().cloned().unwrap_or_default();
    list.push(json!({ "path": image.path, "prompt": image.prompt, "createdAt": image.created_at }));
    meta["generatedImages"] = Value::Array(list);
    conversation::update_message_meta(&state.db, &message_id, meta).await?;
    Ok(chat_ui::get_message(&state.db, &message_id).await?)
}

/// 음성 API 프로필의 목소리 목록 (ElevenLabs는 계정의 목소리)
#[tauri::command]
#[specta::specta]
pub async fn list_tts_voices(app: AppHandle, ai: State<'_, Arc<AiService>>, profile_id: String) -> Result<Vec<tools::Voice>> {
    require(&app, id::TTS)?;
    tools::voices(&ai, &profile_id).await
}

/// 이미지 갤러리: 만든 그림 전부 (최신 순)
#[tauri::command]
#[specta::specta]
pub fn list_generated_images(app: AppHandle) -> Result<Vec<tools::GeneratedImage>> {
    Ok(tools::list_images(&generated_dir(&app)?))
}

#[tauri::command]
#[specta::specta]
pub fn delete_generated_image(app: AppHandle, path: String) -> Result<()> {
    tools::delete_image(&generated_dir(&app)?, &path)
}

/// 프롬프트로 직접 그리기 (이미지 생성 페이지). refine이면 채팅 AI가 프롬프트를 다듬는다.
/// source: "manual"(생성 페이지) / "game"(게임 대사)
#[tauri::command]
#[specta::specta]
pub async fn generate_image_from_prompt(
    app: AppHandle,
    ai: State<'_, Arc<AiService>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    settings: State<'_, SettingsState<AppSetting>>,
    prompt: String,
    refine: bool,
    source: Option<String>,
) -> Result<tools::GeneratedImage> {
    require(&app, id::IMAGE_GEN)?;
    if prompt.trim().is_empty() {
        return Err(AppError::validation("그릴 내용을 입력하세요", None));
    }
    let p = read(&plugin)?.image_gen;
    let chat_profile = fallback_profile(&read(&settings)?, None);
    let final_prompt = if refine {
        tools::image_prompt(&ai, &crate::models::settings::ImageGenSetting { ai_prompt: true, ..p.clone() }, &chat_profile, &prompt, "").await?
    } else {
        let style = p.style_prompt.trim();
        if style.is_empty() { prompt.trim().to_string() } else { format!("{}, {style}", prompt.trim()) }
    };
    let source = source.filter(|s| s == "game").unwrap_or_else(|| "manual".into());
    tools::generate_image(&ai, &p, &chat_profile, &final_prompt, &generated_dir(&app)?, &source, None).await
}
