//! 코어 플러그인의 AI 도구: 번역 · 음성 읽기(TTS) · 이미지 생성.
//!
//! 셋 다 사용자가 연결한 API 프로필(주소 + keyring의 키)을 쓴다. 비우면 앱에서 고른 API.
//! - 번역: 채팅 AI에 번역 지시를 보낸다 (공급자와 무관하게 어댑터를 그대로 탄다)
//! - 음성: OpenAI 호환 `POST {base}/audio/speech` → mp3를 앱 캐시(tts/)에 저장, 같은 글·설정이면 다시 만들지 않음
//! - 이미지: OpenAI 호환 `POST {base}/images/generations` (b64_json) → png를 앱 데이터(generated/)에 저장
//! 메시지에 붙는 결과(번역문, 그린 그림)는 메시지 meta에 남겨 다시 열어도 보인다.
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use base64::Engine;
use serde_json::{json, Value};

use crate::{
    db::usage::{UsageEntry, UsageKind},
    error::{AppError, ErrorCode, Result},
    models::{
        ai_service::{ApiProfiles, ChatOptions, PromptMessage, PromptMessageItem, PromptRole},
        settings::{ImageGenSetting, TranslateSetting, TtsSetting},
    },
    service::ai::{AiRequest, AiService},
};

const OPENAI_BASE: &str = "https://api.openai.com/v1";
const MAX_TTS_CHARS: usize = 4000;

/// 프로필 고르기: 플러그인 설정 → 앱에서 고른 API
pub fn pick_profile(own: &Option<String>, fallback: &str) -> Result<String> {
    own.clone()
        .filter(|s| !s.is_empty())
        .or_else(|| Some(fallback.to_string()).filter(|s| !s.is_empty()))
        .ok_or_else(|| AppError::validation("API를 먼저 고르세요 (설정 > 코어 플러그인 또는 API 키)", None))
}

fn base_url(profile: &ApiProfiles) -> String {
    let b = profile.base_url.trim().trim_end_matches('/');
    if b.is_empty() {
        OPENAI_BASE.to_string()
    } else {
        b.to_string()
    }
}

fn message(role: PromptRole, content: String) -> PromptMessageItem {
    PromptMessageItem {
        images: vec![],
        role,
        token: 0,
        content,
        source: None,
    }
}

/// 채팅 AI에 지시 하나를 보내 답을 받는다 (번역, 이미지 프롬프트 쓰기)
async fn ask(ai: &AiService, profile_id: String, model: Option<String>, system: String, user: String, max_tokens: u32) -> Result<String> {
    let prompt = PromptMessage {
        chat_options: ChatOptions {
            max_tokens: Some(max_tokens),
            stream: false,
            temperature: Some(0.2),
            ..ChatOptions::default()
        },
        messages: vec![message(PromptRole::System, system), message(PromptRole::User, user)],
    };
    Ok(ai.generate_once(AiRequest { profile_id, prompt, model }).await?.content.trim().to_string())
}

// =========================================================
// 번역
// =========================================================

/// 기능 설정에 API를 따로 정했을 때만 그 기능의 모델을 쓴다 (모델은 API에 딸린 이름 — API가 앱·대화 기본이면 그 API의 기본 모델, ADR-020)
fn own_model(api_profile_id: &Option<String>, model: &str) -> Option<String> {
    api_profile_id.as_deref().filter(|s| !s.is_empty())?;
    Some(model.trim().to_string()).filter(|m| !m.is_empty())
}

pub fn translate_rules(to: &str) -> String {
    crate::service::app_prompts::translate(to)
}

pub async fn translate(ai: &AiService, setting: &TranslateSetting, fallback_profile: &str, text: &str, to: &str) -> Result<String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }
    let profile = pick_profile(&setting.api_profile_id, fallback_profile)?;
    let max = ((text.chars().count() * 3) as u32).clamp(256, 8192);
    ask(
        ai,
        profile,
        own_model(&setting.api_profile_id, &setting.model),
        translate_rules(to),
        text.to_string(),
        max,
    )
    .await
}

// =========================================================
// 음성
// =========================================================

/// 읽을 글만 남긴다: 코드 블록·태그·마크다운 기호를 빼고, dialogue_only면 *지문*도 뺀다
pub fn speakable(text: &str, dialogue_only: bool) -> String {
    let mut s = text.to_string();
    // ``` 코드 블록
    while let Some(start) = s.find("```") {
        let Some(end) = s[start + 3..].find("```") else { break };
        s.replace_range(start..start + 3 + end + 3, " ");
    }
    // <태그>
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    if dialogue_only {
        let mut kept = String::new();
        let mut in_action = false;
        for c in out.chars() {
            if c == '*' {
                in_action = !in_action;
                continue;
            }
            if !in_action {
                kept.push(c);
            }
        }
        out = kept;
    }
    out.replace(['*', '_', '#', '`'], "").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn cache_name(text: &str, setting: &TtsSetting, profile: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (text, &setting.model, &setting.voice, setting.speed.to_bits(), profile).hash(&mut h);
    format!("{:016x}.mp3", h.finish())
}

/// 이미 만든 음성 파일 (없으면 None). 호출 전에 확인해 요금 없는 경우는 작업을 만들지 않는다
pub fn cached_speech(setting: &TtsSetting, fallback_profile: &str, text: &str, cache_dir: &Path) -> Option<PathBuf> {
    let input: String = speakable(text, setting.dialogue_only).chars().take(MAX_TTS_CHARS).collect();
    let profile_id = pick_profile(&setting.api_profile_id, fallback_profile).ok()?;
    let path = cache_dir.join("tts").join(cache_name(&input, setting, &profile_id));
    (!input.is_empty() && path.exists()).then_some(path)
}

/// 글을 음성 파일(mp3)로. 이미 만든 것이 있으면 그 경로.
/// 프로필 종류가 elevenlabs면 ElevenLabs, 아니면 OpenAI 호환 /audio/speech.
pub async fn speak(ai: &AiService, setting: &TtsSetting, fallback_profile: &str, text: &str, cache_dir: &Path) -> Result<PathBuf> {
    let input: String = speakable(text, setting.dialogue_only).chars().take(MAX_TTS_CHARS).collect();
    if input.is_empty() {
        return Err(AppError::validation("읽을 글이 없습니다", None));
    }
    let profile_id = pick_profile(&setting.api_profile_id, fallback_profile)?;
    let path = cache_dir.join("tts").join(cache_name(&input, setting, &profile_id));
    if path.exists() {
        return Ok(path);
    }
    let profile = ai.load_profile(&profile_id).await?;
    let key = ai.credential(&profile)?;
    ai.ensure_budget(&profile.id, UsageKind::Tts).await?;
    let started = std::time::Instant::now();
    let result = speak_call(ai, setting, &profile, key, &input).await;
    let mut entry = UsageEntry::new(
        UsageKind::Tts,
        &profile.id,
        if setting.model.is_empty() { &profile.model } else { &setting.model },
    );
    entry.characters = input.chars().count() as u64;
    entry.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    if let Err(e) = &result {
        entry.ok = false;
        entry.error = Some(e.to_response().message);
    }
    ai.record_usage(entry).await;
    let bytes = result?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, &bytes)?;
    Ok(path)
}

/// 음성 공급자 호출 → mp3 바이트
async fn speak_call(ai: &AiService, setting: &TtsSetting, profile: &ApiProfiles, key: String, input: &str) -> Result<Vec<u8>> {
    let speed = setting.speed.clamp(0.25, 4.0);
    let res = if profile.format_type == "elevenlabs" {
        if setting.voice.trim().is_empty() {
            return Err(AppError::validation("ElevenLabs 목소리를 먼저 고르세요 (설정 > 음성 읽기)", None));
        }
        ai.http()
            .post(format!(
                "{}/text-to-speech/{}?output_format=mp3_44100_128",
                base_url_or(profile, ELEVENLABS_BASE),
                setting.voice.trim()
            ))
            .header("xi-api-key", key)
            .header("Accept", "audio/mpeg")
            // ElevenLabs의 속도는 0.7~1.2
            .json(&json!({ "text": input, "model_id": setting.model, "voice_settings": { "speed": speed.clamp(0.7, 1.2) } }))
            .send()
            .await?
    } else {
        ai.http()
            .post(format!("{}/audio/speech", base_url(profile)))
            .bearer_auth(key)
            .json(&json!({ "model": setting.model, "voice": setting.voice, "input": input, "speed": speed, "response_format": "mp3" }))
            .send()
            .await?
    };
    Ok(ok_bytes(res).await?.to_vec())
}

// =========================================================
// 이미지 생성
// =========================================================

const ELEVENLABS_BASE: &str = "https://api.elevenlabs.io/v1";
const NOVELAI_BASE: &str = "https://image.novelai.net";

fn base_url_or(profile: &ApiProfiles, default: &str) -> String {
    let b = profile.base_url.trim().trim_end_matches('/');
    if b.is_empty() {
        default.to_string()
    } else {
        b.to_string()
    }
}

/// 고를 수 있는 목소리 (ElevenLabs는 계정의 목소리 목록, 그 밖은 OpenAI 기본 목소리)
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

pub async fn voices(ai: &AiService, profile_id: &str) -> Result<Vec<Voice>> {
    let profile = ai.load_profile(profile_id).await?;
    if profile.format_type != "elevenlabs" {
        return Ok(["alloy", "ash", "ballad", "coral", "echo", "fable", "nova", "onyx", "sage", "shimmer"]
            .into_iter()
            .map(|v| Voice {
                id: v.into(),
                name: v.into(),
                description: None,
            })
            .collect());
    }
    let key = ai.credential(&profile)?;
    let res = ai
        .http()
        .get(format!("{}/voices", base_url_or(&profile, ELEVENLABS_BASE)))
        .header("xi-api-key", key)
        .send()
        .await?;
    let v: Value = serde_json::from_slice(&ok_bytes(res).await?)?;
    Ok(v["voices"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    let labels = x["labels"]
                        .as_object()
                        .map(|o| o.values().filter_map(Value::as_str).collect::<Vec<_>>().join(", "));
                    Some(Voice {
                        id: x["voice_id"].as_str()?.to_string(),
                        name: x["name"].as_str().unwrap_or("?").to_string(),
                        description: labels.filter(|s| !s.is_empty()),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

// =========================================================
// 이미지 생성 (OpenAI 호환 / NovelAI)
// =========================================================

/// 메시지로 이미지 프롬프트를 만든다 (ai_prompt면 채팅 AI가 정리 — NovelAI면 태그로, 아니면 문장으로) + 화풍
pub async fn image_prompt(ai: &AiService, setting: &ImageGenSetting, chat_profile: &str, message: &str, context: &str) -> Result<String> {
    let body = if setting.ai_prompt {
        let user = if context.trim().is_empty() {
            message.to_string()
        } else {
            format!("[Characters]\n{context}\n\n[Message]\n{message}")
        };
        let rules = crate::service::app_prompts::image_prompt(image_is_novelai(ai, setting, chat_profile).await);
        let model = own_model(&setting.prompt_api_profile_id, &setting.prompt_model);
        ask(ai, pick_profile(&setting.prompt_api_profile_id, chat_profile)?, model, rules.into(), user, 300).await?
    } else {
        speakable(message, false)
    };
    // 화풍·품질 태그 칸은 없앴다: 품질은 NovelAI 품질 태그(quality)로 붙는다 (novelai_body)
    Ok(body)
}

/// NovelAI 품질 태그 (프롬프트 끝). 4.5·5 모델 기준
pub fn quality_tags(quality: &str) -> &'static str {
    match quality {
        "none" => "",
        "light" => "very aesthetic, masterpiece",
        _ => "very aesthetic, masterpiece, no text",
    }
}

/// V4 이후 모델(4, 4.5, 5 …)은 v4_prompt 구조로 보내야 한다 (5도 없으면 500 오류)
fn is_v4_plus(model: &str) -> bool {
    model
        .strip_prefix("nai-diffusion-")
        .and_then(|rest| rest.split('-').next())
        .and_then(|n| n.parse::<u32>().ok())
        .is_some_and(|n| n >= 4)
}

async fn image_is_novelai(ai: &AiService, setting: &ImageGenSetting, fallback: &str) -> bool {
    match pick_profile(&setting.api_profile_id, fallback) {
        Ok(id) => ai.load_profile(&id).await.is_ok_and(|p| p.format_type == "novelai"),
        Err(_) => false,
    }
}

/// "832x1216" → (832, 1216). NovelAI는 64의 배수여야 해서 맞춘다
fn parse_size(size: &str) -> (u32, u32) {
    let mut it = size.split(['x', 'X', '*']).filter_map(|n| n.trim().parse::<u32>().ok());
    let (w, h) = (it.next().unwrap_or(832), it.next().unwrap_or(1216));
    let snap = |n: u32| (n.clamp(64, 2048) / 64).max(1) * 64;
    (snap(w), snap(h))
}

/// NovelAI 요청 본문 (v4 이상은 v4_prompt 구조가 필요하다)
pub fn novelai_body(setting: &ImageGenSetting, prompt: &str, seed: u32) -> Value {
    let (width, height) = parse_size(&setting.size);
    let v4 = is_v4_plus(&setting.model);
    let tags = quality_tags(&setting.quality);
    let prompt_owned = if tags.is_empty() || prompt.contains(tags) {
        prompt.to_string()
    } else {
        format!("{}, {tags}", prompt.trim_end().trim_end_matches(','))
    };
    let prompt = prompt_owned.as_str();
    let mut params = json!({
        "params_version": 3,
        "width": width,
        "height": height,
        "scale": setting.scale,
        "sampler": setting.sampler,
        "steps": setting.steps.clamp(1, 50),
        "n_samples": 1,
        "seed": seed,
        "ucPreset": 0,
        "qualityToggle": !tags.is_empty(),
        "negative_prompt": setting.negative_prompt,
        "noise_schedule": if v4 { "karras" } else { "native" },
    });
    if v4 {
        params["v4_prompt"] = json!({ "caption": { "base_caption": prompt, "char_captions": [] }, "use_coords": false, "use_order": true });
        params["v4_negative_prompt"] = json!({ "caption": { "base_caption": setting.negative_prompt, "char_captions": [] }, "legacy_uc": false });
    }
    json!({ "input": prompt, "model": setting.model, "action": "generate", "parameters": params })
}

/// NovelAI 응답(zip)에서 첫 그림
fn first_png_in_zip(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| AppError::ai_provider(ErrorCode::AiProviderError, format!("NovelAI 응답을 열 수 없습니다: {e}"), false))?;
    for i in 0..zip.len() {
        let mut f = zip
            .by_index(i)
            .map_err(|e| AppError::ai_provider(ErrorCode::AiProviderError, e.to_string(), false))?;
        if f.name().to_ascii_lowercase().ends_with(".png") {
            let mut out = Vec::new();
            std::io::Read::read_to_end(&mut f, &mut out)?;
            return Ok(out);
        }
    }
    Err(AppError::ai_provider(ErrorCode::AiProviderError, "NovelAI 응답에 그림이 없습니다", false))
}

/// 만든 그림 하나 (갤러리). 파일 옆의 같은 이름 .json에 정보를 둔다
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedImage {
    pub path: String,
    pub prompt: String,
    pub model: String,
    /// "message" | "manual" | "game" | "upload"
    pub source: String,
    /// 메시지에서 만들었으면 그 메시지·대화
    pub message_id: Option<String>,
    pub conversation_id: Option<String>,
    pub created_at: i64,
    /// 생성 정보 (만들 때 쓴 설정, 올린 그림이면 파일에 들어 있던 정보)
    #[serde(default)]
    pub meta: Vec<crate::service::image_meta::MetaItem>,
    /// 앨범: 폰 카메라로 만든 그림이면 그 폰 주인 ("persona:<id>" / "character:<id>")
    #[serde(default)]
    pub album: Option<String>,
    /// 원래 파일 이름 (올린 그림)
    #[serde(default)]
    pub name: Option<String>,
    /// 태그: "preset"(생성 프리셋으로 씀) / "upload"(그냥 올린 그림) / "background"(배경) 등
    #[serde(default)]
    pub tags: Vec<String>,
    /// 다른 곳(캐릭터·페르소나·에셋)의 그림: 갤러리에서 지우거나 태그를 달 수 없다 (저장하지 않음)
    #[serde(default, skip_deserializing)]
    pub readonly: bool,
}

impl GeneratedImage {
    /// 앱의 다른 곳에 있는 그림을 갤러리에 보일 때
    pub fn external(path: String, source: &str, name: String, tags: Vec<String>) -> Self {
        let created_at = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis() as i64);
        Self {
            path,
            prompt: String::new(),
            model: String::new(),
            source: source.into(),
            message_id: None,
            conversation_id: None,
            created_at,
            meta: vec![],
            album: None,
            name: Some(name),
            tags,
            readonly: true,
        }
    }
}

/// 갤러리 그림의 보일 이름 바꾸기 (파일 자체 이름은 그대로, 정보 .json의 name). out_dir 안의 그림만
pub fn rename_image(out_dir: &Path, path: &str, name: &str) -> Result<GeneratedImage> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::validation("이름을 입력하세요", None));
    }
    let target = std::fs::canonicalize(path).map_err(|_| AppError::not_found("그림을 찾을 수 없습니다"))?;
    let root = std::fs::canonicalize(out_dir).map_err(|_| AppError::not_found("그림 폴더가 없습니다"))?;
    if !target.starts_with(&root) {
        return Err(AppError::validation("갤러리의 그림만 이름을 바꿀 수 있습니다", None));
    }
    let mut info = list_images(out_dir)
        .into_iter()
        .find(|i| Path::new(&i.path) == Path::new(path))
        .ok_or_else(|| AppError::not_found("그림을 찾을 수 없습니다"))?;
    info.name = Some(name.chars().take(200).collect());
    std::fs::write(Path::new(&info.path).with_extension("json"), serde_json::to_vec(&info)?)?;
    Ok(info)
}

/// 갤러리 그림의 태그 바꾸기 (정보 .json에 저장). out_dir 안의 그림만
pub fn set_image_tags(out_dir: &Path, path: &str, tags: Vec<String>) -> Result<GeneratedImage> {
    let target = std::fs::canonicalize(path).map_err(|_| AppError::not_found("그림을 찾을 수 없습니다"))?;
    let root = std::fs::canonicalize(out_dir).map_err(|_| AppError::not_found("그림 폴더가 없습니다"))?;
    if !target.starts_with(&root) {
        return Err(AppError::validation("갤러리의 그림에만 태그를 달 수 있습니다", None));
    }
    let mut info = list_images(out_dir)
        .into_iter()
        .find(|i| Path::new(&i.path) == Path::new(path))
        .ok_or_else(|| AppError::not_found("그림을 찾을 수 없습니다"))?;
    let mut tags: Vec<String> = tags.into_iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).take(20).collect();
    tags.dedup();
    info.tags = tags;
    std::fs::write(Path::new(&info.path).with_extension("json"), serde_json::to_vec(&info)?)?;
    Ok(info)
}

/// 갤러리가 다루는 그림 확장자
const GALLERY_EXTS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];
fn is_gallery_image(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| GALLERY_EXTS.contains(&e.to_ascii_lowercase().as_str()))
}

/// 생성할 때 쓴 설정 → 정보 줄
fn setting_meta(setting: &ImageGenSetting, novelai: bool) -> Vec<crate::service::image_meta::MetaItem> {
    let mut v = vec![("크기", setting.size.clone())];
    if novelai {
        v.extend([
            ("네거티브", setting.negative_prompt.clone()),
            ("단계", setting.steps.to_string()),
            ("프롬프트 반영", format!("{}", setting.scale)),
            ("샘플러", setting.sampler.clone()),
        ]);
    }
    v.into_iter()
        .filter(|(_, val)| !val.trim().is_empty())
        .map(|(k, val)| crate::service::image_meta::MetaItem { key: k.into(), value: val })
        .collect()
}

/// 갤러리에 그림 올리기: out_dir에 복사하고, PNG에 든 AI 생성 정보를 읽어 .json에 정리해 둔다
pub fn import_images(out_dir: &Path, paths: &[String]) -> Result<Vec<GeneratedImage>> {
    std::fs::create_dir_all(out_dir)?;
    let mut out = Vec::new();
    for src in paths.iter().take(100) {
        let src = Path::new(src);
        if !is_gallery_image(src) {
            return Err(AppError::validation("png·jpg·webp·gif 그림만 올릴 수 있습니다", None));
        }
        let bytes = std::fs::read(src)?;
        let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
        let id = uuid::Uuid::new_v4();
        let path = out_dir.join(format!("{id}.{ext}"));
        std::fs::write(&path, &bytes)?;
        let meta = crate::service::image_meta::parse(&crate::service::image_meta::png_text_chunks(&bytes));
        let model = meta.items.iter().find(|i| i.key == "모델").map(|i| i.value.clone()).unwrap_or_default();
        let mut items = vec![crate::service::image_meta::MetaItem {
            key: "원래 파일".into(),
            value: src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        }];
        items.extend(meta.items);
        let info = GeneratedImage {
            path: path.to_string_lossy().into_owned(),
            prompt: meta.prompt,
            model,
            source: "upload".into(),
            message_id: None,
            conversation_id: None,
            created_at: chrono::Utc::now().timestamp_millis(),
            meta: items,
            album: None,
            name: src.file_name().map(|n| n.to_string_lossy().into_owned()),
            tags: vec!["upload".into()],
            readonly: false,
        };
        std::fs::write(out_dir.join(format!("{id}.json")), serde_json::to_vec(&info)?)?;
        out.push(info);
    }
    Ok(out)
}

/// 프롬프트로 그림을 만들어 out_dir에 png(+ 정보 .json)로 저장한다. message = (메시지 id, 대화 id)
pub async fn generate_image(
    ai: &AiService,
    setting: &ImageGenSetting,
    fallback_profile: &str,
    prompt: &str,
    out_dir: &Path,
    source: &str,
    message: Option<(&str, &str)>,
) -> Result<GeneratedImage> {
    let profile_id = pick_profile(&setting.api_profile_id, fallback_profile)?;
    let profile = ai.load_profile(&profile_id).await?;
    let key = ai.credential(&profile)?;
    ai.ensure_budget(&profile.id, UsageKind::Image).await?;
    let started = std::time::Instant::now();
    let result = image_call(ai, setting, &profile, key, prompt).await;
    let mut entry = UsageEntry::new(
        UsageKind::Image,
        &profile.id,
        if setting.model.is_empty() { &profile.model } else { &setting.model },
    );
    entry.images = u64::from(result.is_ok());
    entry.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    if let Err(e) = &result {
        entry.ok = false;
        entry.error = Some(e.to_response().message);
    }
    ai.record_usage(entry).await;
    let png = result?;
    let mut info = save_image(png, setting, prompt, out_dir, source, message)?;
    info.meta = setting_meta(setting, profile.format_type == "novelai");
    std::fs::write(Path::new(&info.path).with_extension("json"), serde_json::to_vec(&info)?)?;
    Ok(info)
}

/// 이미지 공급자 호출 → png 바이트
async fn image_call(ai: &AiService, setting: &ImageGenSetting, profile: &ApiProfiles, key: String, prompt: &str) -> Result<Vec<u8>> {
    let png = if profile.format_type == "novelai" {
        let seed = rand::random::<u32>() % 4_294_967_000;
        let res = ai
            .http()
            .post(format!("{}/ai/generate-image", base_url_or(profile, NOVELAI_BASE)))
            .bearer_auth(key)
            .json(&novelai_body(setting, prompt, seed))
            .send()
            .await?;
        first_png_in_zip(&ok_bytes(res).await?)?
    } else {
        let mut body = json!({ "model": setting.model, "prompt": prompt, "size": setting.size, "n": 1 });
        // dall-e 계열은 응답 형식을 따로 고르지 않으면 URL을 준다. gpt-image 계열은 항상 b64_json
        if setting.model.starts_with("dall-e") {
            body["response_format"] = json!("b64_json");
        }
        let res = ai
            .http()
            .post(format!("{}/images/generations", base_url(profile)))
            .bearer_auth(key)
            .json(&body)
            .send()
            .await?;
        let v: Value = serde_json::from_slice(&ok_bytes(res).await?)?;
        let item = v["data"]
            .get(0)
            .ok_or_else(|| AppError::ai_provider(ErrorCode::AiProviderError, "이미지 응답에 그림이 없습니다", false))?;
        if let Some(b64) = item["b64_json"].as_str() {
            base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| AppError::ai_provider(ErrorCode::AiProviderError, format!("이미지를 읽을 수 없습니다: {e}"), false))?
        } else if let Some(url) = item["url"].as_str() {
            ok_bytes(ai.http().get(url).send().await?).await?
        } else {
            return Err(AppError::ai_provider(ErrorCode::AiProviderError, "이미지 응답 형식을 알 수 없습니다", false));
        }
    };
    Ok(png.to_vec())
}

fn save_image(png: Vec<u8>, setting: &ImageGenSetting, prompt: &str, out_dir: &Path, source: &str, message: Option<(&str, &str)>) -> Result<GeneratedImage> {
    std::fs::create_dir_all(out_dir)?;
    let id = uuid::Uuid::new_v4();
    let path = out_dir.join(format!("{id}.png"));
    std::fs::write(&path, png)?;
    let info = GeneratedImage {
        path: path.to_string_lossy().into_owned(),
        prompt: prompt.to_string(),
        model: setting.model.clone(),
        source: source.to_string(),
        message_id: message.map(|(m, _)| m.to_string()),
        conversation_id: message.map(|(_, c)| c.to_string()),
        created_at: chrono::Utc::now().timestamp_millis(),
        meta: vec![],
        album: None,
        name: None,
        tags: vec![],
        readonly: false,
    };
    std::fs::write(out_dir.join(format!("{id}.json")), serde_json::to_vec(&info)?)?;
    Ok(info)
}

/// NovelAI 연결 확인 (요금 없음): 계정 구독 정보를 읽는다. 이미지 주소와 다른 api.novelai.net을 쓴다
pub async fn novelai_check(ai: &AiService, profile: &ApiProfiles) -> Result<String> {
    let v = novelai_subscription(ai, profile).await?;
    Ok(format!("{} · Anlas {}", novelai_tier(&v), novelai_anlas(&v)))
}

async fn novelai_subscription(ai: &AiService, profile: &ApiProfiles) -> Result<Value> {
    let key = ai.credential(profile)?;
    let res = ai.http().get("https://image.novelai.net/user/subscription").bearer_auth(key).send().await?;
    Ok(serde_json::from_slice(&ok_bytes(res).await?).unwrap_or(Value::Null))
}

fn novelai_tier(v: &Value) -> &'static str {
    match v["tier"].as_i64() {
        Some(0) => "무료",
        Some(1) => "Tablet",
        Some(2) => "Scroll",
        Some(3) => "Opus",
        _ => "구독 정보 없음",
    }
}

fn novelai_anlas(v: &Value) -> i64 {
    v["trainingStepsLeft"]["fixedTrainingStepsLeft"].as_i64().unwrap_or(0) + v["trainingStepsLeft"]["purchasedTrainingSteps"].as_i64().unwrap_or(0)
}

/// 공급자 계정의 잔여 (구독형은 우리 앱이 막지 않고 보여주기만 한다. 요금 없음, 누를 때만 읽음)
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct QuotaLine {
    pub label: String,
    pub value: String,
}

fn line(label: &str, value: impl Into<String>) -> QuotaLine {
    QuotaLine {
        label: label.into(),
        value: value.into(),
    }
}

/// NovelAI·ElevenLabs의 잔여. 다른 공급자는 빈 목록
pub async fn provider_quota(ai: &AiService, profile: &ApiProfiles) -> Result<Vec<QuotaLine>> {
    match profile.format_type.as_str() {
        "novelai" => {
            let v = novelai_subscription(ai, profile).await?;
            let mut out = vec![line("구독", novelai_tier(&v)), line("Anlas", novelai_anlas(&v).to_string())];
            if let Some(unlimited) = v["perks"]["unlimitedImageGeneration"].as_bool() {
                out.push(line("무제한 그림", if unlimited { "가능" } else { "아님" }));
            }
            // 공식 문서가 없어 응답에서 알아볼 수 있는 값(숫자·참/거짓)을 그대로 모아 보여준다 (남은 한도 항목을 찾는 데 씀)
            if let Some(perks) = v["perks"].as_object() {
                let rest: Vec<String> = perks
                    .iter()
                    .filter(|(k, _)| k.as_str() != "unlimitedImageGeneration")
                    .filter_map(|(k, x)| match x {
                        Value::Bool(b) => Some(format!("{k}={b}")),
                        Value::Number(n) => Some(format!("{k}={n}")),
                        _ => None,
                    })
                    .collect();
                if !rest.is_empty() {
                    out.push(line("기타", rest.join(" · ")));
                }
            }
            Ok(out)
        }
        "elevenlabs" => {
            let key = ai.credential(profile)?;
            let res = ai
                .http()
                .get(format!("{}/user/subscription", base_url_or(profile, ELEVENLABS_BASE)))
                .header("xi-api-key", key)
                .send()
                .await?;
            let v: Value = serde_json::from_slice(&ok_bytes(res).await?).unwrap_or(Value::Null);
            let (used, limit) = (v["character_count"].as_i64().unwrap_or(0), v["character_limit"].as_i64().unwrap_or(0));
            let mut out = vec![
                line("남은 크레딧", (limit - used).max(0).to_string()),
                line("사용", format!("{used} / {limit}")),
            ];
            if let Some(tier) = v["tier"].as_str() {
                out.push(line("요금제", tier));
            }
            if let Some(at) = v["next_character_count_reset_unix"]
                .as_i64()
                .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
            {
                out.push(line("초기화", at.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string()));
            }
            Ok(out)
        }
        _ => Ok(vec![]),
    }
}

/// 갤러리: out_dir의 그림들 (최신 순). 정보 파일이 없는 그림도 보인다
pub fn list_images(out_dir: &Path) -> Vec<GeneratedImage> {
    let Ok(entries) = std::fs::read_dir(out_dir) else { return vec![] };
    let mut out: Vec<GeneratedImage> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_gallery_image(p))
        .map(|p| {
            let path = p.to_string_lossy().into_owned();
            match std::fs::read(p.with_extension("json"))
                .ok()
                .and_then(|b| serde_json::from_slice::<GeneratedImage>(&b).ok())
            {
                Some(m) => GeneratedImage { path, ..m },
                None => GeneratedImage {
                    path,
                    prompt: String::new(),
                    model: String::new(),
                    source: "unknown".into(),
                    message_id: None,
                    conversation_id: None,
                    meta: vec![],
                    album: None,
                    name: p.file_name().map(|n| n.to_string_lossy().into_owned()),
                    tags: vec![],
                    readonly: false,
                    created_at: std::fs::metadata(&p)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_millis() as i64),
                },
            }
        })
        .collect();
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out
}

/// 갤러리에서 지우기. out_dir 안의 png만 지운다 (경로 밖으로 나가지 못하게)
pub fn delete_image(out_dir: &Path, path: &str) -> Result<()> {
    let target = std::fs::canonicalize(path).map_err(|_| AppError::not_found("그림을 찾을 수 없습니다"))?;
    let root = std::fs::canonicalize(out_dir).map_err(|_| AppError::not_found("그림 폴더가 없습니다"))?;
    if !target.starts_with(&root) || !is_gallery_image(&target) {
        return Err(AppError::validation("갤러리의 그림만 지울 수 있습니다", None));
    }
    std::fs::remove_file(&target)?;
    let _ = std::fs::remove_file(target.with_extension("json"));
    Ok(())
}

/// 성공 응답의 본문. 실패면 공급자 오류 메시지를 담은 오류
async fn ok_bytes(res: reqwest::Response) -> Result<Vec<u8>> {
    let status = res.status();
    let bytes = res.bytes().await?.to_vec();
    if status.is_success() {
        return Ok(bytes);
    }
    let detail = serde_json::from_slice::<Value>(&bytes)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| String::from_utf8_lossy(&bytes).chars().take(300).collect());
    let code = match status.as_u16() {
        401 | 403 => ErrorCode::AiProviderAuthFailed,
        404 => ErrorCode::AiProviderModelNotFound,
        429 => ErrorCode::AiProviderRateLimit,
        _ => ErrorCode::AiProviderError,
    };
    Err(AppError::ai_provider(
        code,
        format!("{status}: {detail}"),
        status.as_u16() == 429 || status.is_server_error(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // 기능의 모델은 그 기능에 API를 따로 정했을 때만 쓴다 (모델은 API에 딸린 이름 — ADR-020)
    #[test]
    fn a_features_model_only_applies_with_its_own_api() {
        let api = Some("a1".to_string());
        assert_eq!(own_model(&api, " m1 ").as_deref(), Some("m1"));
        assert_eq!(own_model(&api, "  "), None, "비우면 그 API의 기본 모델");
        assert_eq!(own_model(&None, "m1"), None, "API가 앱·대화 기본이면 모델도 따르지 않는다");
        assert_eq!(own_model(&Some(String::new()), "m1"), None);
    }

    #[test]
    fn speakable_strips_markup() {
        let t = "*웃으며* \"안녕\" <emotion::미나>happy</emotion>\n```html\n<b>x</b>\n```\n끝";
        assert_eq!(speakable(t, false), "웃으며 \"안녕\" happy 끝");
        assert_eq!(speakable(t, true), "\"안녕\" happy 끝");
    }

    #[test]
    fn profile_falls_back_to_app_choice() {
        assert_eq!(pick_profile(&Some("p1".into()), "app").unwrap(), "p1");
        assert_eq!(pick_profile(&None, "app").unwrap(), "app");
        assert_eq!(pick_profile(&Some(String::new()), "app").unwrap(), "app");
        assert!(pick_profile(&None, "").is_err());
    }

    #[test]
    fn tts_cache_name_changes_with_voice() {
        let a = TtsSetting::default();
        let b = TtsSetting {
            voice: "nova".into(),
            ..TtsSetting::default()
        };
        assert_ne!(cache_name("hi", &a, "p"), cache_name("hi", &b, "p"));
        assert_eq!(cache_name("hi", &a, "p"), cache_name("hi", &a, "p"));
    }

    #[test]
    fn v4_plus_models_and_quality_tags() {
        assert!(is_v4_plus("nai-diffusion-4-5-full"));
        assert!(is_v4_plus("nai-diffusion-5-full"));
        assert!(!is_v4_plus("nai-diffusion-3"));
        let s = ImageGenSetting {
            model: "nai-diffusion-5-full".into(),
            quality: "light".into(),
            ..ImageGenSetting::default()
        };
        let b = novelai_body(&s, "1girl", 1);
        assert_eq!(b["input"], "1girl, very aesthetic, masterpiece");
        assert!(b["parameters"]["v4_prompt"].is_object());
        assert_eq!(b["parameters"]["qualityToggle"], true);
        let none = ImageGenSetting { quality: "none".into(), ..s };
        assert_eq!(novelai_body(&none, "1girl", 1)["input"], "1girl");
    }

    #[test]
    fn novelai_body_uses_v4_structure_and_snaps_size() {
        let s = ImageGenSetting {
            size: "800x1200".into(),
            ..ImageGenSetting::default()
        };
        let b = novelai_body(&s, "1girl, forest", 7);
        assert_eq!(b["parameters"]["width"], 768);
        assert_eq!(b["parameters"]["height"], 1152);
        // 기본 품질(standard) 태그가 끝에 붙는다
        assert_eq!(
            b["parameters"]["v4_prompt"]["caption"]["base_caption"],
            "1girl, forest, very aesthetic, masterpiece, no text"
        );
        let v3 = novelai_body(
            &ImageGenSetting {
                model: "nai-diffusion-3".into(),
                ..ImageGenSetting::default()
            },
            "x",
            1,
        );
        assert!(v3["parameters"].get("v4_prompt").is_none());
    }

    #[test]
    fn gallery_lists_and_deletes_only_inside() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.png"), b"x").unwrap();
        let info = json!({ "path": "old", "prompt": "p", "model": "m", "source": "manual", "messageId": null, "conversationId": null, "createdAt": 5 });
        std::fs::write(dir.path().join("a.json"), serde_json::to_vec(&info).unwrap()).unwrap();
        let list = list_images(dir.path());
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].prompt, "p");
        let outside = tempfile::NamedTempFile::new().unwrap();
        assert!(delete_image(dir.path(), outside.path().to_str().unwrap()).is_err());
        delete_image(dir.path(), &list[0].path).unwrap();
        assert!(list_images(dir.path()).is_empty());
    }
}
