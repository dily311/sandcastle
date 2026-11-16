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
    PromptMessageItem { images: vec![], role, token: 0, content }
}

/// 채팅 AI에 지시 하나를 보내 답을 받는다 (번역, 이미지 프롬프트 쓰기)
async fn ask(ai: &AiService, profile_id: String, system: String, user: String, max_tokens: u32) -> Result<String> {
    let prompt = PromptMessage {
        chat_options: ChatOptions { max_tokens: Some(max_tokens), stream: false, temperature: Some(0.2), ..ChatOptions::default() },
        messages: vec![message(PromptRole::System, system), message(PromptRole::User, user)],
    };
    Ok(ai.generate_once(AiRequest { profile_id, prompt }).await?.content.trim().to_string())
}

// =========================================================
// 번역
// =========================================================

pub fn translate_rules(to: &str) -> String {
    crate::service::app_prompts::translate(to)
}

pub async fn translate(ai: &AiService, setting: &TranslateSetting, fallback_profile: &str, text: &str, to: &str) -> Result<String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }
    let profile = pick_profile(&setting.api_profile_id, fallback_profile)?;
    let max = ((text.chars().count() * 3) as u32).clamp(256, 8192);
    ask(ai, profile, translate_rules(to), text.to_string(), max).await
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
    let started = std::time::Instant::now();
    let result = speak_call(ai, setting, &profile, key, &input).await;
    let mut entry = UsageEntry::new(UsageKind::Tts, &profile.id, if setting.model.is_empty() { &profile.model } else { &setting.model });
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
            .post(format!("{}/text-to-speech/{}?output_format=mp3_44100_128", base_url_or(profile, ELEVENLABS_BASE), setting.voice.trim()))
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
            .map(|v| Voice { id: v.into(), name: v.into(), description: None })
            .collect());
    }
    let key = ai.credential(&profile)?;
    let res = ai.http().get(format!("{}/voices", base_url_or(&profile, ELEVENLABS_BASE))).header("xi-api-key", key).send().await?;
    let v: Value = serde_json::from_slice(&ok_bytes(res).await?)?;
    Ok(v["voices"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    let labels = x["labels"].as_object().map(|o| o.values().filter_map(Value::as_str).collect::<Vec<_>>().join(", "));
                    Some(Voice { id: x["voice_id"].as_str()?.to_string(), name: x["name"].as_str().unwrap_or("?").to_string(), description: labels.filter(|s| !s.is_empty()) })
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
        let user = if context.trim().is_empty() { message.to_string() } else { format!("[Characters]\n{context}\n\n[Message]\n{message}") };
        let rules = crate::service::app_prompts::image_prompt(image_is_novelai(ai, setting, chat_profile).await);
        ask(ai, pick_profile(&setting.prompt_api_profile_id, chat_profile)?, rules.into(), user, 300).await?
    } else {
        speakable(message, false)
    };
    let style = setting.style_prompt.trim();
    Ok(if style.is_empty() { body } else { format!("{body}, {style}") })
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
    let v4 = setting.model.starts_with("nai-diffusion-4");
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
        "qualityToggle": true,
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
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| AppError::ai_provider(ErrorCode::AiProviderError, format!("NovelAI 응답을 열 수 없습니다: {e}"), false))?;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| AppError::ai_provider(ErrorCode::AiProviderError, e.to_string(), false))?;
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
    /// "message" | "manual" | "game"
    pub source: String,
    /// 메시지에서 만들었으면 그 메시지·대화
    pub message_id: Option<String>,
    pub conversation_id: Option<String>,
    pub created_at: i64,
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
    let started = std::time::Instant::now();
    let result = image_call(ai, setting, &profile, key, prompt).await;
    let mut entry = UsageEntry::new(UsageKind::Image, &profile.id, if setting.model.is_empty() { &profile.model } else { &setting.model });
    entry.images = u64::from(result.is_ok());
    entry.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    if let Err(e) = &result {
        entry.ok = false;
        entry.error = Some(e.to_response().message);
    }
    ai.record_usage(entry).await;
    let png = result?;
    save_image(png, setting, prompt, out_dir, source, message)
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
        let res = ai.http().post(format!("{}/images/generations", base_url(profile))).bearer_auth(key).json(&body).send().await?;
        let v: Value = serde_json::from_slice(&ok_bytes(res).await?)?;
        let item = v["data"].get(0).ok_or_else(|| AppError::ai_provider(ErrorCode::AiProviderError, "이미지 응답에 그림이 없습니다", false))?;
        if let Some(b64) = item["b64_json"].as_str() {
            base64::engine::general_purpose::STANDARD.decode(b64).map_err(|e| AppError::ai_provider(ErrorCode::AiProviderError, format!("이미지를 읽을 수 없습니다: {e}"), false))?
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
    };
    std::fs::write(out_dir.join(format!("{id}.json")), serde_json::to_vec(&info)?)?;
    Ok(info)
}

/// 갤러리: out_dir의 그림들 (최신 순). 정보 파일이 없는 그림도 보인다
pub fn list_images(out_dir: &Path) -> Vec<GeneratedImage> {
    let Ok(entries) = std::fs::read_dir(out_dir) else { return vec![] };
    let mut out: Vec<GeneratedImage> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
        .map(|p| {
            let path = p.to_string_lossy().into_owned();
            match std::fs::read(p.with_extension("json")).ok().and_then(|b| serde_json::from_slice::<GeneratedImage>(&b).ok()) {
                Some(m) => GeneratedImage { path, ..m },
                None => GeneratedImage {
                    path,
                    prompt: String::new(),
                    model: String::new(),
                    source: "unknown".into(),
                    message_id: None,
                    conversation_id: None,
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
    if !target.starts_with(&root) || !target.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
        return Err(AppError::validation("생성한 그림만 지울 수 있습니다", None));
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
    Err(AppError::ai_provider(code, format!("{status}: {detail}"), status.as_u16() == 429 || status.is_server_error()))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let b = TtsSetting { voice: "nova".into(), ..TtsSetting::default() };
        assert_ne!(cache_name("hi", &a, "p"), cache_name("hi", &b, "p"));
        assert_eq!(cache_name("hi", &a, "p"), cache_name("hi", &a, "p"));
    }

    #[test]
    fn novelai_body_uses_v4_structure_and_snaps_size() {
        let s = ImageGenSetting { size: "800x1200".into(), ..ImageGenSetting::default() };
        let b = novelai_body(&s, "1girl, forest", 7);
        assert_eq!(b["parameters"]["width"], 768);
        assert_eq!(b["parameters"]["height"], 1152);
        assert_eq!(b["parameters"]["v4_prompt"]["caption"]["base_caption"], "1girl, forest");
        let v3 = novelai_body(&ImageGenSetting { model: "nai-diffusion-3".into(), ..ImageGenSetting::default() }, "x", 1);
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
