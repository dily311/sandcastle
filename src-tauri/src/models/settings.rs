use std::collections::HashMap;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppSetting {
    pub auto_update: bool,
    pub language: String,
    pub selected_preset: String,
    pub send_on_enter: bool,
    pub show_timestamps: bool,
    pub selected_api_profile: Option<String>,
    pub selected_character_id: Option<String>,
    pub selected_persona_id: String,
    // "이어하기" UX용. 실제 게임 실행 컨텍스트는 여전히 AppState.active_archive가 기준.
    pub last_played_game_id: Option<String>, 
    /// phone 모드 대화에만 붙는 기본 시스템 프롬프트 ({{char}}/{{user}} 사용 가능, 비우면 붙이지 않음)
    /// - 기존 저장값에 이 필드가 없어도 설정 전체가 초기화되지 않도록 serde default 필수
    #[serde(default = "default_phone_mode_prompt")]
    pub phone_mode_prompt: String,
}

pub const DEFAULT_PHONE_MODE_PROMPT: &str = "[폰 메신저 대화]
{{char}}와(과) {{user}}는 지금 서로 떨어진 곳에서 스마트폰 메신저로 대화하고 있다.
- {{char}}는 실제 메신저 메시지처럼 짧고 자연스럽게 답한다.
- 행동·표정 묘사나 나레이션 없이, 실제로 보낼 메시지 내용만 쓴다.
- 여러 메시지로 나눠 보내고 싶으면 줄바꿈으로 구분한다.";

fn default_phone_mode_prompt() -> String {
    DEFAULT_PHONE_MODE_PROMPT.to_string()
}

impl Default for AppSetting {
    fn default() -> Self {
        Self {
            auto_update: true,
            language: "ko".into(),
            selected_preset: "default".into(),
            send_on_enter: false,
            show_timestamps: true,
            selected_api_profile: None,
            selected_character_id: None,
            selected_persona_id: "default".into(),
            last_played_game_id: None,
            phone_mode_prompt: default_phone_mode_prompt(),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ColorSchema {
    Light,
    Dark,
    System,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSetting {
    pub mode: ColorSchema,
    pub select_theme: String,
    pub is_frameless: bool,
    pub accent_color: String,
    pub font_size: i64,
    /// 회색 톤 (Radix 회색 계열: slate, gray, mauve, sage, olive, sand)
    #[serde(default = "default_gray_tone")]
    pub gray_tone: String,
    /// CSS 스니펫 (옵시디언처럼 켜고 끄는 작은 CSS 조각)
    #[serde(default)]
    pub snippets: Vec<CssSnippet>,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CssSnippet {
    pub id: String,
    pub name: String,
    pub css: String,
    pub enabled: bool,
}

fn default_gray_tone() -> String {
    "slate".into()
}

impl Default for ThemeSetting {
    fn default() -> Self {
        Self {
            mode: ColorSchema::System,
            select_theme: "default".into(),
            is_frameless: true,
            accent_color: "#8b85f9".into(),
            font_size: 16,
            gray_tone: default_gray_tone(),
            snippets: vec![],
        }
    }
}

/// 테마 색: 화면 모드별로 덮어쓸 CSS 변수 ("--bg" → "#ffffff"). 비어 있으면 앱 기본값
#[derive(Serialize, Deserialize, Debug, Clone, Default, specta::Type)]
pub struct ThemeColors {
    #[serde(default)]
    pub light: HashMap<String, String>,
    #[serde(default)]
    pub dark: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CssThemeMeta {
    #[serde(default, skip_deserializing, skip_serializing_if = "String::is_empty")]
    pub filename: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: String,
    /// 목록 카드 미리보기용
    #[serde(default)]
    pub colors: ThemeColors,
}

// 2. 기존 저장/Tauri 통신용 전체 구조체
#[derive(Serialize, Deserialize, Debug, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CssTheme {
    #[serde(default, skip_deserializing, skip_serializing_if = "String::is_empty")]
    pub filename: String,
    pub name: String,
    pub author: String,
    pub version: String,
    /// 직접 쓰는 CSS (색 편집으로 부족할 때)
    #[serde(default)]
    pub css_content: String,
    #[serde(default)]
    pub colors: ThemeColors,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct FeatureSetting {
    pub feature: HashMap<String, bool>,
}

impl Default for FeatureSetting {
    fn default() -> Self {
        Self { feature: HashMap::new() }
    }
}

// ---------------------------------------------------------
// 코어 플러그인 설정 (번역 · 음성 · 이미지 생성). 켜짐/꺼짐은 FeatureSetting, 세부 설정은 여기.
// ---------------------------------------------------------
#[derive(Debug, Serialize, Deserialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PluginSetting {
    pub translate: TranslateSetting,
    pub tts: TtsSetting,
    pub image_gen: ImageGenSetting,
    pub phone: PhoneSetting,
}

/// 폰 대화의 기본 API·프리셋 (대화에 따로 고르지 않았을 때. 비우면 앱 기본값)
#[derive(Debug, Serialize, Deserialize, Clone, Default, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PhoneSetting {
    pub api_profile_id: Option<String>,
    pub preset_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TranslateSetting {
    /// 번역에 쓸 API 프로필 (비우면 앱에서 고른 API)
    pub api_profile_id: Option<String>,
    /// AI 답변을 이 언어로 (예: "한국어")
    pub target_lang: String,
    /// 내 입력을 이 언어로 (예: "English")
    pub input_lang: String,
    /// 새 AI 답변을 자동으로 번역
    pub auto_translate: bool,
}

impl Default for TranslateSetting {
    fn default() -> Self {
        Self { api_profile_id: None, target_lang: "한국어".into(), input_lang: "English".into(), auto_translate: false }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct TtsSetting {
    /// 음성 API 프로필 (주소 + 키). 종류가 ElevenLabs면 ElevenLabs, 아니면 OpenAI 호환 /audio/speech
    pub api_profile_id: Option<String>,
    /// OpenAI: tts-1 등 / ElevenLabs: model_id (eleven_multilingual_v2 등)
    pub model: String,
    /// OpenAI: 목소리 이름 (alloy 등) / ElevenLabs: voice_id
    pub voice: String,
    /// 0.25 ~ 4.0
    pub speed: f32,
    /// 새 AI 답변을 자동으로 읽기
    pub auto_read: bool,
    /// *지문*은 빼고 대사만 읽기
    pub dialogue_only: bool,
}

impl Default for TtsSetting {
    fn default() -> Self {
        Self { api_profile_id: None, model: "eleven_multilingual_v2".into(), voice: String::new(), speed: 1.0, auto_read: false, dialogue_only: false }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ImageGenSetting {
    /// 이미지 API 프로필 (주소 + 키). 종류가 NovelAI면 NovelAI, 아니면 OpenAI 호환 /images/generations
    pub api_profile_id: Option<String>,
    pub model: String,
    /// 예: "1024x1024", NovelAI는 "832x1216"
    pub size: String,
    /// 모든 그림에 붙일 화풍·품질 태그 (예: "anime style, soft lighting" / NovelAI: "masterpiece, best quality")
    pub style_prompt: String,
    /// 메시지를 바로 쓰지 않고 채팅 AI가 먼저 이미지 프롬프트를 쓴다
    pub ai_prompt: bool,
    /// 이미지 프롬프트를 쓸 채팅 API (비우면 대화/앱의 것)
    pub prompt_api_profile_id: Option<String>,
    /// NovelAI 생성 프리셋 (OpenAI는 쓰지 않음)
    pub negative_prompt: String,
    pub steps: u32,
    pub scale: f32,
    pub sampler: String,
}

impl Default for ImageGenSetting {
    fn default() -> Self {
        Self {
            api_profile_id: None,
            model: "nai-diffusion-4-5-full".into(),
            size: "832x1216".into(),
            style_prompt: String::new(),
            ai_prompt: true,
            prompt_api_profile_id: None,
            negative_prompt: "lowres, bad anatomy, bad hands, text, error, missing fingers, worst quality, low quality, jpeg artifacts, watermark, blurry".into(),
            steps: 28,
            scale: 5.0,
            sampler: "k_euler_ancestral".into(),
        }
    }
}

// 각 설정 타입이 자기 "키"와 "이벤트 이름"만 정의하면 됨
pub trait SettingsDomain:
    Serialize + DeserializeOwned + Default + Clone + Send + Sync + 'static
{
    const STORE_KEY: &'static str;
    const EVENT_NAME: &'static str;
}

impl SettingsDomain for AppSetting {
    const STORE_KEY: &'static str = "app-setting";
    const EVENT_NAME: &'static str = "app_setting::changed";
}

impl SettingsDomain for ThemeSetting {
    const STORE_KEY: &'static str = "theme-setting";
    const EVENT_NAME: &'static str = "theme_setting::changed";
}
impl SettingsDomain for FeatureSetting {
    const STORE_KEY: &'static str = "feature-setting";
    const EVENT_NAME: &'static str = "feature_setting::changed";
}
impl SettingsDomain for PluginSetting {
    const STORE_KEY: &'static str = "plugin-setting";
    const EVENT_NAME: &'static str = "plugin_setting::changed";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_app_setting_without_phone_prompt_keeps_other_fields() {
        // phone_mode_prompt가 생기기 전에 저장된 설정
        let old = serde_json::json!({
            "autoUpdate": false,
            "language": "en",
            "selectedPreset": "p1",
            "sendOnEnter": true,
            "showTimestamps": false,
            "selectedApiProfile": "api-1",
            "selectedCharacterId": null,
            "selectedPersonaId": "me",
            "lastPlayedGameId": null
        });
        let s: AppSetting = serde_json::from_value(old).expect("기존 설정이 그대로 읽혀야 함");
        assert_eq!(s.language, "en");
        assert_eq!(s.selected_api_profile.as_deref(), Some("api-1"));
        assert_eq!(s.phone_mode_prompt, DEFAULT_PHONE_MODE_PROMPT);
    }
}
