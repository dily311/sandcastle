use std::collections::HashMap;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
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
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ColorSchema {
    Light,
    Dark,
    System,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CssThemeMeta {
    #[serde(default, skip_deserializing, skip_serializing_if = "String::is_empty")]
    pub filename: String,
    pub name: String,
    pub author: String,
    pub version: String,
}

// 2. 기존 저장/Tauri 통신용 전체 구조체
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CssTheme {
    #[serde(default, skip_deserializing, skip_serializing_if = "String::is_empty")]
    pub filename: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub css_content: String, // 저장할 때는 사용하므로 유지
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
