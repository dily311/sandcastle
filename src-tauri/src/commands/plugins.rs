//! 코어 플러그인 커맨드: 설정, 번역, 음성 읽기, 이미지 생성. 기능이 꺼져 있으면 모두 거절한다.
use std::sync::Arc;

use crate::service::data::settings_scope::same_path;
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
    task::{CreateTaskRequest, TaskManager, TaskOwner, TaskType},
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

/// 앱 프롬프트 기본값 (설정 화면이 보여주고 "기본값으로" 되돌릴 때)
#[tauri::command]
#[specta::specta]
pub fn default_app_prompts() -> crate::models::settings::AppPrompts {
    crate::service::app_prompts::defaults()
}

#[tauri::command]
#[specta::specta]
pub fn update_plugin_setting(app: AppHandle, state: State<'_, SettingsState<PluginSetting>>, setting: PluginSetting) -> Result<PluginSetting> {
    {
        let mut cur = state.0.lock().map_err(|_| AppError::internal("settings lock poisoned"))?;
        *cur = setting.clone();
        save(&app, &*cur)?;
    }
    crate::service::app_prompts::set(&setting.prompts);
    crate::service::prompt::prompt_manager::set_phone_clock(
        setting.phone.custom_datetime_enabled,
        setting.phone.custom_date.clone(),
        setting.phone.custom_time.clone(),
    );
    let _ = app.emit(PluginSetting::EVENT_NAME, &setting);
    Ok(setting)
}

// ---------------------------------------------------------
// 번역
// ---------------------------------------------------------

/// 글 번역. to를 비우면 설정의 답변 언어. (입력창의 "보내기 전에 번역"은 to에 입력 언어를 넣어 부른다)
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn translate_text(
    app: AppHandle,
    ai: State<'_, Arc<AiService>>,
    tasks: State<'_, Arc<TaskManager>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    settings: State<'_, SettingsState<AppSetting>>,
    text: String,
    to: Option<String>,
    auto: Option<bool>,
) -> Result<String> {
    require(&app, id::TRANSLATE)?;
    let p = read(&plugin)?.translate;
    let to = to.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| p.target_lang.clone());
    let db = &app.state::<AppState>().db;
    cached_translate(
        db,
        &ai,
        &tasks,
        &p,
        &fallback_profile(&read(&settings)?, None),
        &text,
        &to,
        auto.unwrap_or(false),
    )
    .await
}

/// 공용 번역 캐시를 먼저 보고, 없을 때만 AI로 번역해 캐시에 넣는다 (같은 문장에 다시 요금을 내지 않게).
/// AI를 부를 때만 작업으로 남긴다 (캐시에서 꺼낸 건 요금이 없으니 기록하지 않음)
#[allow(clippy::too_many_arguments)]
async fn cached_translate(
    db: &crate::db::conn::DbPool,
    ai: &AiService,
    tasks: &TaskManager,
    p: &crate::models::settings::TranslateSetting,
    fallback: &str,
    text: &str,
    to: &str,
    auto: bool,
) -> Result<String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }
    if let Some(hit) = translation::get_translation(db, text, to).await? {
        return Ok(hit);
    }
    let job = CreateTaskRequest::inline(TaskType::Translate, TaskOwner::System, id::TRANSLATE, format!("번역 → {to}"), auto);
    let out = tasks.run_inline(job, |_| tools::translate(ai, p, fallback, text, to)).await?;
    translation::put_translation(db, text, to, &out).await?;
    Ok(out)
}

/// 이미 번역해 둔 것만 꺼낸다 (AI를 부르지 않음). 게임 대사 줄·지난 대사 창이 다시 열릴 때 쓴다.
/// texts와 같은 순서로, 캐시에 없으면 null
#[tauri::command]
#[specta::specta]
pub async fn get_cached_translations(
    app: AppHandle,
    state: State<'_, AppState>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    texts: Vec<String>,
    to: Option<String>,
) -> Result<Vec<Option<String>>> {
    if !crate::sys::plugins::is_enabled(&app, id::TRANSLATE) {
        return Ok(vec![None; texts.len()]);
    }
    let to = to
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| read(&plugin).map(|p| p.translate.target_lang).unwrap_or_default());
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
#[allow(clippy::too_many_arguments)]
pub async fn translate_message(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    tasks: State<'_, Arc<TaskManager>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    settings: State<'_, SettingsState<AppSetting>>,
    message_id: String,
    auto: Option<bool>,
) -> Result<Message> {
    require(&app, id::TRANSLATE)?;
    let p = read(&plugin)?.translate;
    let msg = chat_ui::get_message(&state.db, &message_id).await?;
    let conv = conversation::get_conversation(&state.db, &msg.conversation_id).await?;
    let fallback = fallback_profile(&read(&settings)?, conv.api_profile_id.as_deref());
    let text = cached_translate(&state.db, &ai, &tasks, &p, &fallback, &msg.content, &p.target_lang, auto.unwrap_or(false)).await?;
    let mut meta = msg.meta.clone().filter(Value::is_object).unwrap_or_else(|| json!({}));
    meta["translation"] = json!({ "lang": p.target_lang, "text": text, "source": msg.content });
    conversation::update_message_meta(&state.db, &message_id, meta).await?;
    Ok(chat_ui::get_message(&state.db, &message_id).await?)
}

// ---------------------------------------------------------
// 음성
// ---------------------------------------------------------

/// 글을 음성 파일(mp3)로 만들고 경로를 돌려준다 (같은 글·설정이면 캐시). cached_only면 캐시만 (읽기 화면)
#[allow(clippy::too_many_arguments)]
#[tauri::command]
#[specta::specta]
pub async fn speak_text(
    app: AppHandle,
    ai: State<'_, Arc<AiService>>,
    tasks: State<'_, Arc<TaskManager>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    settings: State<'_, SettingsState<AppSetting>>,
    text: String,
    auto: Option<bool>,
    cached_only: Option<bool>,
) -> Result<String> {
    require(&app, id::TTS)?;
    let p = read(&plugin)?.tts;
    let cache = app.path().app_cache_dir().map_err(|e| AppError::io(e.to_string()))?;
    let fallback = fallback_profile(&read(&settings)?, None);
    // 이미 만든 음성이면 작업 없이 바로 (요금 없음)
    if let Some(path) = tools::cached_speech(&p, &fallback, &text, &cache) {
        return Ok(path.to_string_lossy().into_owned());
    }
    // 읽기 화면: 이미 만든 음성만 (새로 만들지 않음 — 요금 없음, 5-E-6-2)
    if cached_only.unwrap_or(false) {
        return Err(AppError::validation("아직 만든 음성이 없음 (채팅 모드에서 읽기를 누르면 만들어짐)", None));
    }
    let job = CreateTaskRequest::inline(TaskType::Tts, TaskOwner::System, id::TTS, "음성 읽기", auto.unwrap_or(false));
    let path = tasks.run_inline(job, |_| tools::speak(&ai, &p, &fallback, &text, &cache)).await?;
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
    tasks: State<'_, Arc<TaskManager>>,
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
    let out_dir = generated_dir(&app)?;
    // 프롬프트 작성(채팅 AI)과 그리기(이미지 API)를 한 작업으로: 사용량은 두 줄로 남는다
    let job = CreateTaskRequest::inline(TaskType::ImageGen, TaskOwner::Image, id::IMAGE_GEN, "메시지 그림", false);
    let image = tasks
        .run_inline(job, |_| async {
            let prompt = tools::image_prompt(&ai, &p, &chat_profile, &msg.content, &context).await?;
            tools::generate_image(&ai, &p, &chat_profile, &prompt, &out_dir, "message", Some((&message_id, &msg.conversation_id))).await
        })
        .await?;

    let mut meta = msg.meta.clone().filter(Value::is_object).unwrap_or_else(|| json!({}));
    let mut list = meta["generatedImages"].as_array().cloned().unwrap_or_default();
    list.push(json!({ "path": image.path, "prompt": image.prompt, "createdAt": image.created_at }));
    meta["generatedImages"] = Value::Array(list);
    conversation::update_message_meta(&state.db, &message_id, meta).await?;
    crate::service::notify::task_done(&app, "그림 생성", "메시지 그림이 완성됨");
    Ok(chat_ui::get_message(&state.db, &message_id).await?)
}

/// 이미지 자동 생성: 응답에 붙은 그림 프롬프트(meta.autoImagePrompt)로 바로 그린다 (채팅 AI 호출 없음).
/// 응답이 온 뒤 화면이 자동으로 부르므로 실행 기록에는 "자동"으로 남는다
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn generate_auto_image(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    tasks: State<'_, Arc<TaskManager>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    settings: State<'_, SettingsState<AppSetting>>,
    message_id: String,
) -> Result<Message> {
    require(&app, id::IMAGE_AUTO)?;
    let p = read(&plugin)?.image_gen;
    let msg = chat_ui::get_message(&state.db, &message_id).await?;
    let prompt = msg
        .meta
        .as_ref()
        .and_then(|m| m["autoImagePrompt"].as_str())
        .map(str::to_string)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| AppError::validation("이 메시지에는 그림 프롬프트가 없습니다", None))?;
    let conv = conversation::get_conversation(&state.db, &msg.conversation_id).await?;
    let chat_profile = fallback_profile(&read(&settings)?, conv.api_profile_id.as_deref());
    let out_dir = generated_dir(&app)?;
    let job = CreateTaskRequest::inline(TaskType::ImageGen, TaskOwner::Image, id::IMAGE_AUTO, "자동 그림", true);
    let image = tasks
        .run_inline(job, |_| {
            tools::generate_image(&ai, &p, &chat_profile, &prompt, &out_dir, "message", Some((&message_id, &msg.conversation_id)))
        })
        .await?;
    let mut meta = msg.meta.clone().filter(Value::is_object).unwrap_or_else(|| json!({}));
    let mut list = meta["generatedImages"].as_array().cloned().unwrap_or_default();
    list.push(json!({ "path": image.path, "prompt": image.prompt, "createdAt": image.created_at, "auto": true }));
    meta["generatedImages"] = Value::Array(list);
    conversation::update_message_meta(&state.db, &message_id, meta).await?;
    crate::service::notify::task_done(&app, "자동 그림", "자동 그림이 완성됨");
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

/// 갤러리에 그림 올리기 (복사, AI 생성 정보는 파일에서 읽어 정리). 이미지 생성 플러그인이 꺼져 있어도 된다 (AI 호출 없음)
#[tauri::command]
#[specta::specta]
pub fn import_gallery_images(app: AppHandle, paths: Vec<String>) -> Result<Vec<tools::GeneratedImage>> {
    tools::import_images(&generated_dir(&app)?, &paths)
}

#[tauri::command]
#[specta::specta]
pub fn delete_generated_image(app: AppHandle, theme: State<'_, SettingsState<crate::models::settings::ThemeSetting>>, path: String) -> Result<()> {
    tools::delete_image(&generated_dir(&app)?, &path)?;
    forget_backgrounds(&app, &theme, std::slice::from_ref(&path))
}

/// 지운 그림을 테마 배경 목록·앱 배경에서도 뺀다 (배경 태그를 단 갤러리 그림)
fn forget_backgrounds(app: &AppHandle, theme: &SettingsState<crate::models::settings::ThemeSetting>, paths: &[String]) -> Result<()> {
    let cur = read(theme)?;
    let gone = |b: &String| paths.iter().any(|p| same_path(p, b));
    let list: Vec<String> = cur.backgrounds.iter().filter(|b| !gone(b)).cloned().collect();
    let selected_gone = cur.background_image.as_ref().is_some_and(gone);
    if list.len() == cur.backgrounds.len() && !selected_gone {
        return Ok(());
    }
    let mut patch = json!({ "backgrounds": list });
    if selected_gone {
        patch["backgroundImage"] = Value::Null;
    }
    let settings = crate::sys::settings::apply_patch(app, &theme.0, patch)?;
    let _ = app.emit(<crate::models::settings::ThemeSetting as SettingsDomain>::EVENT_NAME, &settings);
    Ok(())
}

/// 앱의 전체 그림: 갤러리(만든·올린 그림) + 배경 목록 + 캐릭터 아바타·커버·CG 에셋 + 페르소나 아바타.
/// 다른 곳의 그림은 readonly (갤러리에서 지우거나 태그를 달지 않고, 원래 화면에서 관리)
#[tauri::command]
#[specta::specta]
pub async fn list_all_images(
    app: AppHandle,
    state: State<'_, AppState>,
    theme: State<'_, SettingsState<crate::models::settings::ThemeSetting>>,
) -> Result<Vec<tools::GeneratedImage>> {
    use tools::GeneratedImage as G;
    let gen_dir = generated_dir(&app)?;
    let data_dir = app_data_dir(&app)?;
    let mut backgrounds = read(&theme)?.backgrounds;
    let mut out = tools::list_images(&gen_dir);
    // 갤러리 그림 중 배경 목록에 든 것은 배경 태그를 보인다.
    // 거꾸로 배경 태그(그림 옆 .json)가 있는데 목록에서 빠졌으면 다시 넣는다 — 예전 동기화가 다른 기기의 목록으로 덮어써서 빠진 것
    let mut missing = vec![];
    for img in &mut out {
        let listed = backgrounds.iter().any(|b| same_path(b, &img.path));
        let tagged = img.tags.iter().any(|t| t == "background");
        if listed && !tagged {
            img.tags.push("background".into());
        } else if tagged && !listed {
            missing.push(img.path.clone());
        }
    }
    if !missing.is_empty() {
        backgrounds.extend(missing);
        let settings = crate::sys::settings::apply_patch(&app, &theme.0, json!({ "backgrounds": backgrounds }))?;
        let _ = app.emit(<crate::models::settings::ThemeSetting as SettingsDomain>::EVENT_NAME, &settings);
    }
    let file_name = |p: &str| {
        std::path::Path::new(p)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    // 배경 목록 중 갤러리 밖(backgrounds/에 올린 것)
    for b in backgrounds.iter().filter(|b| !std::path::Path::new(b).starts_with(&gen_dir)) {
        if std::path::Path::new(b).exists() {
            let mut g = G::external(b.clone(), "background", file_name(b), vec!["background".into()]);
            g.readonly = false; // 배경 목록에서 빼기·지우기는 됨
            out.push(g);
        }
    }
    let exists = |p: &str| std::path::Path::new(p).exists();
    let chars: Vec<(String, String, Option<String>, Option<String>)> = sqlx::query_as("SELECT id, name, avatar, cover FROM characters")
        .fetch_all(&state.db)
        .await
        .map_err(AppError::from)?;
    for (_, name, avatar, cover) in &chars {
        if let Some(a) = avatar.as_deref().filter(|a| exists(a)) {
            out.push(G::external(a.into(), "avatar", format!("{name} 아바타"), vec![]));
        }
        if let Some(c) = cover.as_deref().filter(|c| exists(c)) {
            out.push(G::external(c.into(), "cover", format!("{name} 커버"), vec![]));
        }
    }
    let names: std::collections::HashMap<&str, &str> = chars.iter().map(|(id, n, _, _)| (id.as_str(), n.as_str())).collect();
    let assets: Vec<(String, String, String)> = sqlx::query_as("SELECT character_id, key, file_path FROM character_images")
        .fetch_all(&state.db)
        .await
        .map_err(AppError::from)?;
    for (cid, key, rel) in assets {
        let abs = data_dir.join(&rel).to_string_lossy().into_owned();
        if exists(&abs) {
            let mut g = G::external(abs, "asset", format!("{} · {key}", names.get(cid.as_str()).unwrap_or(&"?")), vec![]);
            g.readonly = false; // CG 에셋은 갤러리에서도 지울 수 있다 (태그는 달지 않음)
            out.push(g);
        }
    }
    let personas: Vec<(String, Option<String>)> = sqlx::query_as("SELECT name, avatar FROM personas")
        .fetch_all(&state.db)
        .await
        .map_err(AppError::from)?;
    for (name, avatar) in personas {
        if let Some(a) = avatar.filter(|a| exists(a)) {
            out.push(G::external(a, "persona", format!("{name} 아바타"), vec![]));
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

/// 갤러리 그림의 태그 바꾸기 ("preset" / "upload" / "background" 등). 배경 태그는 테마 배경 목록에도 넣고 뺀다
#[tauri::command]
#[specta::specta]
pub fn set_image_tags(
    app: AppHandle,
    theme: State<'_, SettingsState<crate::models::settings::ThemeSetting>>,
    path: String,
    tags: Vec<String>,
) -> Result<tools::GeneratedImage> {
    let info = tools::set_image_tags(&generated_dir(&app)?, &path, tags)?;
    let wants_bg = info.tags.iter().any(|t| t == "background");
    let mut list = read(&theme)?.backgrounds;
    let has = list.iter().any(|b| same_path(b, &path));
    if wants_bg != has {
        if wants_bg {
            list.push(path.clone());
        } else {
            list.retain(|b| !same_path(b, &path));
        }
        let settings = crate::sys::settings::apply_patch(&app, &theme.0, json!({ "backgrounds": list }))?;
        let _ = app.emit(<crate::models::settings::ThemeSetting as SettingsDomain>::EVENT_NAME, &settings);
    }
    Ok(info)
}

/// 갤러리 그림의 보일 이름 바꾸기
#[tauri::command]
#[specta::specta]
pub fn rename_gallery_image(app: AppHandle, path: String, name: String) -> Result<tools::GeneratedImage> {
    tools::rename_image(&generated_dir(&app)?, &path, &name)
}

/// 여러 그림 한 번에 지우기 (갤러리 그림만, 다른 곳의 그림은 건너뜀). 지운 수를 돌려준다
#[tauri::command]
#[specta::specta]
pub async fn delete_gallery_images(
    app: AppHandle,
    state: State<'_, AppState>,
    theme: State<'_, SettingsState<crate::models::settings::ThemeSetting>>,
    paths: Vec<String>,
) -> Result<u32> {
    let dir = generated_dir(&app)?;
    let data_dir = app_data_dir(&app)?;
    // 캐릭터 CG 에셋은 행과 파일(썸네일 포함)을 함께 지운다
    let assets: Vec<(String, String, Option<String>)> = sqlx::query_as("SELECT id, file_path, thumb_path FROM character_images")
        .fetch_all(&state.db)
        .await
        .map_err(AppError::from)?;
    let mut n = 0;
    for p in paths.iter().take(500) {
        if let Some((id, rel, thumb)) = assets.iter().find(|(_, rel, _)| data_dir.join(rel) == std::path::Path::new(p)) {
            crate::db::character_image::delete_character_image(&state.db, id).await?;
            crate::service::chat::chat_images::remove_image_files(&data_dir, &[rel, thumb.as_deref().unwrap_or_default()]);
            n += 1;
        } else if tools::delete_image(&dir, p).is_ok() {
            n += 1;
        }
    }
    forget_backgrounds(&app, &theme, &paths)?;
    Ok(n)
}

/// 프롬프트로 직접 그리기 (이미지 생성 페이지). refine이면 채팅 AI가 프롬프트를 다듬는다.
/// source: "manual"(생성 페이지) / "game"(게임 대사)
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn generate_image_from_prompt(
    app: AppHandle,
    ai: State<'_, Arc<AiService>>,
    tasks: State<'_, Arc<TaskManager>>,
    plugin: State<'_, SettingsState<PluginSetting>>,
    settings: State<'_, SettingsState<AppSetting>>,
    prompt: String,
    refine: bool,
    source: Option<String>,
    preset: Option<crate::models::settings::ImageGenSetting>,
    album: Option<String>,
) -> Result<tools::GeneratedImage> {
    require(&app, id::IMAGE_GEN)?;
    if prompt.trim().is_empty() {
        return Err(AppError::validation("그릴 내용을 입력하세요", None));
    }
    // 만들기 화면에서 고친 프리셋이 있으면 그것으로 (이번 한 번만, 설정은 그대로)
    let p = match preset {
        Some(pr) => pr,
        None => read(&plugin)?.image_gen,
    };
    let chat_profile = fallback_profile(&read(&settings)?, None);
    let source = source.filter(|s| s == "game" || s == "phone").unwrap_or_else(|| "manual".into());
    let out_dir = generated_dir(&app)?;
    let label = match source.as_str() {
        "game" => "게임 장면 그림",
        "phone" => "폰 카메라",
        _ => "그림 만들기",
    };
    let job = CreateTaskRequest::inline(TaskType::ImageGen, TaskOwner::Image, id::IMAGE_GEN, label, false);
    let out = tasks
        .run_inline(job, |_| async {
            let final_prompt = if refine {
                tools::image_prompt(
                    &ai,
                    &crate::models::settings::ImageGenSetting { ai_prompt: true, ..p.clone() },
                    &chat_profile,
                    &prompt,
                    "",
                )
                .await?
            } else {
                prompt.trim().to_string()
            };
            let mut img = tools::generate_image(&ai, &p, &chat_profile, &final_prompt, &out_dir, &source, None).await?;
            // 폰 카메라: 그 폰 앨범으로 (정보 파일에도 남김)
            if let Some(album) = album.filter(|a| !a.trim().is_empty()) {
                img.album = Some(album);
                std::fs::write(std::path::Path::new(&img.path).with_extension("json"), serde_json::to_vec(&img)?)?;
            }
            Ok(img)
        })
        .await?;
    crate::service::notify::task_done(&app, label, "그림이 완성됨");
    Ok(out)
}
