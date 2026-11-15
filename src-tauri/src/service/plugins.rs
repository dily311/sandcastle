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
    format!(
        "You are a translator. Translate the user's text into {to}.\n\
- Output only the translation, nothing else.\n\
- Keep line breaks, *actions in asterisks*, \"quotes\", names, <tags>, {{{{macros}}}} and ``` code blocks ``` exactly as they are (do not translate code).\n\
- Keep the speaker prefixes like \"Name:\" if present."
    )
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

/// 글을 음성 파일(mp3)로. 이미 만든 것이 있으면 그 경로
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
    let res = ai
        .http()
        .post(format!("{}/audio/speech", base_url(&profile)))
        .bearer_auth(key)
        .json(&json!({ "model": setting.model, "voice": setting.voice, "input": input, "speed": setting.speed.clamp(0.25, 4.0), "response_format": "mp3" }))
        .send()
        .await?;
    let bytes = ok_bytes(res).await?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, &bytes)?;
    Ok(path)
}

// =========================================================
// 이미지 생성
// =========================================================

const IMAGE_PROMPT_RULES: &str = "You write prompts for an image generation model. Read the roleplay message and describe ONE vivid scene from it as a single English prompt: subjects and their appearance, action, setting, lighting, mood, camera framing. No dialogue text, no names unless needed, no explanations. Under 80 words. Output only the prompt.";

/// 메시지로 이미지 프롬프트를 만든다 (ai_prompt면 채팅 AI가 영어로 정리, 아니면 그대로) + 화풍
pub async fn image_prompt(ai: &AiService, setting: &ImageGenSetting, chat_profile: &str, message: &str, context: &str) -> Result<String> {
    let body = if setting.ai_prompt {
        let user = if context.trim().is_empty() { message.to_string() } else { format!("[Characters]\n{context}\n\n[Message]\n{message}") };
        ask(ai, pick_profile(&None, chat_profile)?, IMAGE_PROMPT_RULES.into(), user, 300).await?
    } else {
        speakable(message, false)
    };
    let style = setting.style_prompt.trim();
    Ok(if style.is_empty() { body } else { format!("{body}, {style}") })
}

/// 프롬프트로 그림을 만들어 out_dir에 png로 저장하고 경로를 돌려준다
pub async fn generate_image(ai: &AiService, setting: &ImageGenSetting, fallback_profile: &str, prompt: &str, out_dir: &Path) -> Result<PathBuf> {
    let profile_id = pick_profile(&setting.api_profile_id, fallback_profile)?;
    let profile = ai.load_profile(&profile_id).await?;
    let key = ai.credential(&profile)?;
    let mut body = json!({ "model": setting.model, "prompt": prompt, "size": setting.size, "n": 1 });
    // dall-e 계열은 응답 형식을 따로 고르지 않으면 URL을 준다. gpt-image 계열은 항상 b64_json
    if setting.model.starts_with("dall-e") {
        body["response_format"] = json!("b64_json");
    }
    let res = ai.http().post(format!("{}/images/generations", base_url(&profile))).bearer_auth(key).json(&body).send().await?;
    let bytes = ok_bytes(res).await?;
    let v: Value = serde_json::from_slice(&bytes)?;
    let item = v["data"].get(0).ok_or_else(|| AppError::ai_provider(ErrorCode::AiProviderError, "이미지 응답에 그림이 없습니다", false))?;
    let png = if let Some(b64) = item["b64_json"].as_str() {
        base64::engine::general_purpose::STANDARD.decode(b64).map_err(|e| AppError::ai_provider(ErrorCode::AiProviderError, format!("이미지를 읽을 수 없습니다: {e}"), false))?
    } else if let Some(url) = item["url"].as_str() {
        let r = ai.http().get(url).send().await?;
        ok_bytes(r).await?
    } else {
        return Err(AppError::ai_provider(ErrorCode::AiProviderError, "이미지 응답 형식을 알 수 없습니다", false));
    };
    std::fs::create_dir_all(out_dir)?;
    let path = out_dir.join(format!("{}.png", uuid::Uuid::new_v4()));
    std::fs::write(&path, png)?;
    Ok(path)
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
}
