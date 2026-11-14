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
}

impl Default for ThemeSetting {
    fn default() -> Self {
        Self {
            mode: ColorSchema::System,
            select_theme: "default".into(),
            is_frameless: true,
            accent_color: "#8b85f9".into(),
            font_size: 16,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CssThemeMeta {
    #[serde(default, skip_deserializing, skip_serializing_if = "String::is_empty")]
    pub filename: String,
    pub name: String,
    pub author: String,
    pub version: String,
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
    pub css_content: String, // 저장할 때는 사용하므로 유지
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
