//! 코어 플러그인: 껐다 켤 수 있는 부가 기능 목록 (Obsidian의 코어 플러그인처럼).
//!
//! 켜짐/꺼짐은 기능 설정(FeatureSetting, `feature-setting` 저장소)에 id별로 저장한다. 값이 없으면 목록의 기본값.
//! 위젯 하위 기능(widget.music 등)도 같은 저장소를 쓰지만 이 목록에는 없다 (위젯 설정 화면이 따로 다룬다).
//!
//! 기능을 끄면:
//! - 화면: 메뉴·설정 탭·패널에서 숨기고, 그 화면에 들어가면 "꺼져 있음"을 보여준다 (프론트 plugins 스토어)
//! - 백엔드: AI 결과에 영향을 주는 것은 여기서도 막는다 (정규식 적용, 캐릭터 이미지 키, HTML 서빙, 번역·음성·이미지 생성 호출)
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::{
    error::{AppError, ErrorCode, Result},
    models::settings::FeatureSetting,
    sys::settings::SettingsState,
};

/// 플러그인 id (기능 설정의 키). 바꾸면 저장된 켜짐/꺼짐이 사라지니 바꾸지 않는다.
pub mod id {
    pub const PHONE: &str = "phone";
    pub const GAME: &str = "game";
    /// 캐릭터 이미지 에셋 (<cg> 태그). 예전부터 쓰던 키를 그대로 쓴다.
    pub const IMAGES: &str = "chat.images";
    pub const HTML: &str = "html";
    pub const WIDGET: &str = "widget";
    pub const REGEX: &str = "regex";
    pub const TRANSLATE: &str = "translate";
    pub const TTS: &str = "tts";
    pub const IMAGE_GEN: &str = "imagegen";
    /// 글을 쓰는 AI가 응답과 함께 그림 프롬프트를 쓰고, 응답 뒤 자동으로 그린다
    pub const IMAGE_AUTO: &str = "imageauto";
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CorePlugin {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// 설정에 값이 없을 때
    pub default_enabled: bool,
    /// 데스크톱에서만 되는 기능 (모바일에서는 켜도 동작하지 않음)
    pub desktop_only: bool,
    pub enabled: bool,
}

struct Def {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    default_enabled: bool,
    desktop_only: bool,
}

const CATALOG: &[Def] = &[
    Def {
        id: id::PHONE,
        name: "폰",
        description: "캐릭터와 메신저처럼 대화하는 폰 화면, 캐릭터 폰 엿보기",
        default_enabled: true,
        desktop_only: false,
    },
    Def {
        id: id::GAME,
        name: "게임 모드",
        description: "게임 만들기·가져오기와 플레이 (지도·이벤트·NPC 대화)",
        default_enabled: true,
        desktop_only: false,
    },
    Def {
        id: id::IMAGES,
        name: "캐릭터 이미지",
        description: "캐릭터 이미지 에셋을 AI에게 알려 주고 <cg> 태그로 대화에 표시",
        default_enabled: true,
        desktop_only: false,
    },
    // AI가 쓴 JS를 실행하므로 기본 꺼짐 (켜기 전에는 코드로만 보인다)
    Def {
        id: id::HTML,
        name: "HTML 렌더링",
        description: "AI 답변의 ```html``` 코드 블록을 격리된 창에서 실행해 보여줌 (JS 포함)",
        default_enabled: false,
        desktop_only: false,
    },
    Def {
        id: id::REGEX,
        name: "정규식",
        description: "메시지 글자를 찾아 바꾸는 정규식 (SillyTavern 정규식 가져오기)",
        default_enabled: true,
        desktop_only: false,
    },
    Def {
        id: id::WIDGET,
        name: "위젯",
        description: "바탕화면 위젯 (음악 플레이어 등)",
        default_enabled: false,
        desktop_only: true,
    },
    Def {
        id: id::TRANSLATE,
        name: "번역",
        description: "AI 답변을 내 언어로, 내 입력을 다른 언어로 번역 (연결한 AI로)",
        default_enabled: false,
        desktop_only: false,
    },
    Def {
        id: id::TTS,
        name: "음성 읽기",
        description: "AI 답변을 음성으로 읽어 줌 (OpenAI 호환 음성 API)",
        default_enabled: false,
        desktop_only: false,
    },
    Def {
        id: id::IMAGE_GEN,
        name: "이미지 생성",
        description: "직접 그림 만들기: 이미지 페이지의 새로 만들기, 메시지의 '이 장면 그리기' (NovelAI·OpenAI 호환)",
        default_enabled: false,
        desktop_only: false,
    },
    // 응답마다 이미지 요금이 나가므로 기본 꺼짐
    Def {
        id: id::IMAGE_AUTO,
        name: "이미지 자동 생성",
        description:
            "글을 쓰는 AI가 응답 끝에 그림 프롬프트도 쓰고, 응답이 오면 그 프롬프트로 바로 그림 (응답마다 이미지 요금, 설정 > 이미지 생성의 API·프리셋 사용)",
        default_enabled: false,
        desktop_only: false,
    },
];

fn default_of(plugin_id: &str) -> bool {
    CATALOG.iter().find(|d| d.id == plugin_id).map_or(true, |d| d.default_enabled)
}

/// 저장값(없으면 기본값)으로 켜져 있는가
pub fn enabled_in(setting: &FeatureSetting, plugin_id: &str) -> bool {
    // 데스크톱 전용 기능(위젯)은 모바일에서 늘 꺼짐 (1-C-4)
    if cfg!(mobile) && is_desktop_only(plugin_id) {
        return false;
    }
    setting.feature.get(plugin_id).copied().unwrap_or_else(|| default_of(plugin_id))
}

/// 데스크톱 전용인가 (위젯 하위 기능 widget.* 포함)
fn is_desktop_only(plugin_id: &str) -> bool {
    let root = plugin_id.split('.').next().unwrap_or(plugin_id);
    CATALOG.iter().any(|d| d.desktop_only && d.id == root)
}

/// 앱 상태에서 켜져 있는가. 설정을 읽을 수 없으면 기본값.
pub fn is_enabled(app: &AppHandle, plugin_id: &str) -> bool {
    app.try_state::<SettingsState<FeatureSetting>>()
        .and_then(|s| s.0.lock().ok().map(|f| enabled_in(&f, plugin_id)))
        .unwrap_or_else(|| default_of(plugin_id))
}

/// 꺼져 있으면 오류 (번역·음성·이미지 생성처럼 호출 자체를 막을 때)
pub fn require(app: &AppHandle, plugin_id: &str) -> Result<()> {
    if is_enabled(app, plugin_id) {
        return Ok(());
    }
    let name = CATALOG.iter().find(|d| d.id == plugin_id).map_or(plugin_id, |d| d.name);
    Err(AppError::Validation {
        code: ErrorCode::InvalidInput,
        message: format!("'{name}' 기능이 꺼져 있습니다 (설정 > 코어 플러그인)"),
        details: Some(serde_json::json!({ "plugin": plugin_id })),
    })
}

pub fn list(setting: &FeatureSetting) -> Vec<CorePlugin> {
    CATALOG
        .iter()
        // 모바일에서는 데스크톱 전용 기능을 목록에서 뺀다 (설정 화면·메뉴에 나오지 않게)
        .filter(|d| !(cfg!(mobile) && d.desktop_only))
        .map(|d| CorePlugin {
            id: d.id,
            name: d.name,
            description: d.description,
            default_enabled: d.default_enabled,
            desktop_only: d.desktop_only,
            enabled: enabled_in(setting, d.id),
        })
        .collect()
}

/// 코어 플러그인 목록과 켜짐 상태. 켜고 끄기는 update_feature_setting(id, enabled).
#[tauri::command]
#[specta::specta]
pub fn list_core_plugins(state: State<'_, SettingsState<FeatureSetting>>) -> Result<Vec<CorePlugin>> {
    let setting = state.0.lock().map_err(|_| AppError::internal("feature setting lock poisoned"))?;
    Ok(list(&setting))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn saved_value_wins_over_default() {
        let mut s = FeatureSetting { feature: HashMap::new() };
        assert!(enabled_in(&s, id::PHONE));
        assert!(!enabled_in(&s, id::TRANSLATE));
        s.feature.insert(id::PHONE.into(), false);
        s.feature.insert(id::TRANSLATE.into(), true);
        assert!(!enabled_in(&s, id::PHONE));
        assert!(enabled_in(&s, id::TRANSLATE));
        // 목록에 없는 키(위젯 하위 기능 등)는 켜진 것으로 본다
        assert!(enabled_in(&s, "widget.music"));
    }

    #[test]
    fn catalog_ids_are_unique() {
        let mut ids: Vec<_> = CATALOG.iter().map(|d| d.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), CATALOG.len());
    }
}
