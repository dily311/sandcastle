//! 설정의 "같이 쓰는 것 / 기기 전용" 나누기 (CLAUDE.md 2-A-4, ADR-011).
//!
//! settings.json의 칸마다 동기화할지 적는다. 2-B 묶음에는 `shared_part`만 넣고, 받은 쪽은 `merge_shared`로
//! 자기 기기 전용 칸을 그대로 둔 채 합친다. 새 설정 칸을 만들면 아래 표에 적어야 테스트가 통과한다.
//!
//! 기준
//! - 기기 전용: 창·화면 상태, 그 기기의 API 선택(키가 기기마다 다르고 로컬 모델은 그 PC에서만 닿음), 자동 실행 스위치와
//!   기능 켜기(요금이 나가는 동작은 기기마다 사용자가 직접 켠다 — "사용자 동의 없는 리소스 소비 금지"), 그 기기의
//!   기록 보관 기간(실행 기록·휴지통은 기기마다 따로).
//! - 같이 씀: 테마·프리셋·페르소나 선택, 프롬프트, 언어, 생성 옵션 같은 "취향".
//! - 같이 쓰는 칸 중 앱 데이터 경로가 든 것(`SharedPath`)은 받는 쪽 경로로 바꿔야 한다(2-B, 백업 복원의 경로 보정과 같음).

use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Shared,
    /// 같이 쓰되 앱 데이터 경로가 들어 있어 받는 쪽에서 경로를 바꿔야 함
    SharedPath,
    Device,
}

/// (settings.json의 키/칸 경로, 범위). 객체는 통째로 적거나, 안의 칸을 하나씩 적는다
pub const SETTINGS_SCOPE: &[(&str, Scope)] = &[
    // ---- app-setting ----
    ("app-setting/autoUpdate", Scope::Device), // 쓰지 않는 예전 칸
    ("app-setting/language", Scope::Shared),
    ("app-setting/selectedPreset", Scope::Shared),
    ("app-setting/sendOnEnter", Scope::Device), // 키보드가 있는 PC와 폰이 다름
    ("app-setting/showTimestamps", Scope::Shared),
    ("app-setting/selectedApiProfile", Scope::Device),
    ("app-setting/selectedModel", Scope::Device),       // 전역 API에 딸린 모델이라 API와 같이 기기 전용
    ("app-setting/reasoningEffort", Scope::Device),     // 받는 강도가 모델마다 달라 모델과 같이
    ("app-setting/selectedCharacterId", Scope::Device), // 화면 상태
    ("app-setting/selectedPersonaId", Scope::Shared),
    ("app-setting/lastPlayedGameId", Scope::Device), // 화면 상태
    ("app-setting/phoneModePrompt", Scope::Shared),
    ("app-setting/home", Scope::Shared),
    ("app-setting/checkUpdatesOnStart", Scope::Device),
    ("app-setting/usageRetentionDays", Scope::Device),
    ("app-setting/trashRetentionDays", Scope::Device),
    ("app-setting/cgGroupVariants", Scope::Shared),
    ("app-setting/doneNotify", Scope::Device), // 알림은 기기마다 (아이콘 경로도 그 기기 것)
    ("app-setting/aiCallLog", Scope::Device),  // 보낸 글·받은 글 기록은 기기마다 켠다
    ("app-setting/typing", Scope::SharedPath), // 타이핑 표시는 취향 (그림은 앱 데이터 themes/typing/)
    // ---- theme-setting ----
    ("theme-setting/mode", Scope::Shared),
    ("theme-setting/selectTheme", Scope::Shared),
    ("theme-setting/isFrameless", Scope::Device), // 데스크톱 창 테두리
    ("theme-setting/accentColor", Scope::Shared),
    ("theme-setting/fontSize", Scope::Device), // 화면 크기에 따라 다름
    ("theme-setting/grayTone", Scope::Shared),
    ("theme-setting/snippets", Scope::Shared),
    ("theme-setting/backgroundImage", Scope::SharedPath),
    ("theme-setting/backgrounds", Scope::SharedPath),
    ("theme-setting/backgroundOpacity", Scope::Shared),
    ("theme-setting/fonts", Scope::SharedPath),
    ("theme-setting/appFont", Scope::SharedPath),
    // ---- feature-setting: 기능 켜기는 기기마다 ----
    ("feature-setting", Scope::Device),
    // ---- plugin-setting ----
    ("plugin-setting/translate/apiProfileId", Scope::Device),
    ("plugin-setting/translate/model", Scope::Device), // 모델은 API에 딸린 이름이라 API와 같이 기기마다 (ADR-020)
    ("plugin-setting/translate/targetLang", Scope::Shared),
    ("plugin-setting/translate/inputLang", Scope::Shared),
    ("plugin-setting/translate/autoTranslate", Scope::Device),
    ("plugin-setting/tts/apiProfileId", Scope::Device),
    ("plugin-setting/tts/model", Scope::Shared),
    ("plugin-setting/tts/voice", Scope::Shared),
    ("plugin-setting/tts/speed", Scope::Shared),
    ("plugin-setting/tts/autoRead", Scope::Device),
    ("plugin-setting/tts/dialogueOnly", Scope::Shared),
    ("plugin-setting/imageGen/apiProfileId", Scope::Device),
    ("plugin-setting/imageGen/promptApiProfileId", Scope::Device),
    ("plugin-setting/imageGen/promptModel", Scope::Device),
    ("plugin-setting/imageGen/model", Scope::Shared),
    ("plugin-setting/imageGen/size", Scope::Shared),
    ("plugin-setting/imageGen/stylePrompt", Scope::Shared),
    ("plugin-setting/imageGen/aiPrompt", Scope::Shared),
    ("plugin-setting/imageGen/negativePrompt", Scope::Shared),
    ("plugin-setting/imageGen/steps", Scope::Shared),
    ("plugin-setting/imageGen/scale", Scope::Shared),
    ("plugin-setting/imageGen/sampler", Scope::Shared),
    ("plugin-setting/imageGen/quality", Scope::Shared),
    ("plugin-setting/phone/apiProfileId", Scope::Device),
    ("plugin-setting/phone/model", Scope::Device),
    ("plugin-setting/phone/presetId", Scope::Shared),
    ("plugin-setting/prompts", Scope::Shared),
    ("plugin-setting/ghost", Scope::Device),      // 대필 API·옵션은 기기마다
    ("plugin-setting/quickReply", Scope::Shared), // 빠른 답장 세트는 내가 쓴 글이라 같이 씀
    ("plugin-setting/rewrite", Scope::Device),    // 요금이 나가는 자동 동작은 기기마다 켠다
    ("plugin-setting/summary", Scope::Device),    // 요약 API는 기기마다
    ("plugin-setting/memory", Scope::Device),     // 임베딩 API·모델은 기기마다 (벡터 파일도 기기 전용)
    ("plugin-setting/lint", Scope::Shared),       // 규칙은 취향. 켜기(feature-setting)는 기기마다
    ("plugin-setting/sync", Scope::Device),       // 폴더 경로·자동 동기화는 기기마다
];

/// settings.json에 있는 설정 묶음 키 (sys::settings의 STORE_KEY)
pub const SETTINGS_KEYS: &[&str] = &["app-setting", "theme-setting", "feature-setting", "plugin-setting"];

fn scope_of(path: &str) -> Option<Scope> {
    SETTINGS_SCOPE.iter().find(|(p, _)| *p == path).map(|(_, s)| *s)
}

/// 같이 쓰는 칸만 남긴 사본 (모르는 칸은 넣지 않는다 — 새 칸은 표에 적기 전까지 기기 전용)
pub fn shared_part(store: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for key in SETTINGS_KEYS {
        if let Some(v) = store.get(*key) {
            if let Some(s) = pick(key, v) {
                out.insert(key.to_string(), s);
            }
        }
    }
    out
}

fn pick(path: &str, v: &Value) -> Option<Value> {
    match scope_of(path) {
        Some(Scope::Shared | Scope::SharedPath) => Some(v.clone()),
        Some(Scope::Device) => None,
        None => {
            let obj = v.as_object()?;
            let m: Map<String, Value> = obj
                .iter()
                .filter_map(|(k, cv)| pick(&format!("{path}/{k}"), cv).map(|x| (k.clone(), x)))
                .collect();
            (!m.is_empty()).then_some(Value::Object(m))
        }
    }
}

/// 받은 같이 쓰는 칸을 이 기기의 설정에 합친다. 기기 전용 칸과 표에 없는 칸은 건드리지 않는다
pub fn merge_shared(local: &mut Map<String, Value>, incoming: &Map<String, Value>) {
    for key in SETTINGS_KEYS {
        if let Some(v) = incoming.get(*key) {
            let slot = local.entry(key.to_string()).or_insert_with(|| Value::Object(Map::new()));
            merge_at(key, slot, v);
        }
    }
}

/// 받은 값으로 덮어쓰지 않고 합치는 목록 칸. 배경 목록은 기기마다 따로 늘어나므로 덮어쓰면
/// 다른 기기의 (아직 이쪽 것을 받지 못한) 목록이 이 기기에서 방금 넣은 배경을 지운다.
/// 대신 다른 기기에서 뺀 배경은 이쪽에서 빠지지 않는다 (지우는 것보다 남기는 쪽이 안전)
const UNION_LISTS: &[&str] = &["theme-setting/backgrounds"];

/// 경로 비교용: Windows의 `\`와 동기화로 받은 `/`를 같은 경로로 본다
pub fn same_path(a: &str, b: &str) -> bool {
    a == b || a.replace('\\', "/") == b.replace('\\', "/")
}

fn union_into(local: &mut Value, incoming: &Value) {
    let Some(add) = incoming.as_array() else { return };
    let Some(list) = local.as_array_mut() else {
        *local = incoming.clone();
        return;
    };
    for v in add {
        let dup = match v.as_str() {
            Some(s) => list.iter().any(|x| x.as_str().is_some_and(|x| same_path(x, s))),
            None => list.contains(v),
        };
        if !dup {
            list.push(v.clone());
        }
    }
}

fn merge_at(path: &str, local: &mut Value, incoming: &Value) {
    if UNION_LISTS.contains(&path) {
        return union_into(local, incoming);
    }
    match scope_of(path) {
        Some(Scope::Shared | Scope::SharedPath) => *local = incoming.clone(),
        Some(Scope::Device) => {}
        None => {
            let (Some(l), Some(i)) = (local.as_object_mut(), incoming.as_object()) else {
                return;
            };
            for (k, iv) in i {
                let child = format!("{path}/{k}");
                // 표에 없는 칸은 받지 않는다 (상대가 더 새 버전이라 모르는 칸을 보냈을 수 있음)
                if scope_of(&child).is_none() && !SETTINGS_SCOPE.iter().any(|(p, _)| p.starts_with(&format!("{child}/"))) {
                    continue;
                }
                let slot = l.entry(k.clone()).or_insert_with(|| Value::Object(Map::new()));
                merge_at(&child, slot, iv);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::{AppSetting, FeatureSetting, PluginSetting, SettingsDomain, ThemeSetting};

    fn defaults() -> Map<String, Value> {
        let mut m = Map::new();
        m.insert(AppSetting::STORE_KEY.into(), serde_json::to_value(AppSetting::default()).unwrap());
        m.insert(ThemeSetting::STORE_KEY.into(), serde_json::to_value(ThemeSetting::default()).unwrap());
        m.insert(FeatureSetting::STORE_KEY.into(), serde_json::to_value(FeatureSetting::default()).unwrap());
        m.insert(PluginSetting::STORE_KEY.into(), serde_json::to_value(PluginSetting::default()).unwrap());
        m
    }

    /// 모든 칸이 표에 있어야 한다 (새 칸을 만들면 같이 쓸지 정해서 적기)
    #[test]
    fn every_setting_is_classified() {
        fn walk(path: &str, v: &Value, missing: &mut Vec<String>) {
            if scope_of(path).is_some() {
                return;
            }
            match v.as_object() {
                Some(o) if !o.is_empty() => o.iter().for_each(|(k, c)| walk(&format!("{path}/{k}"), c, missing)),
                _ => missing.push(path.to_string()),
            }
        }
        let d = defaults();
        assert_eq!(d.len(), SETTINGS_KEYS.len());
        let mut missing = vec![];
        for (k, v) in &d {
            walk(k, v, &mut missing);
        }
        assert!(missing.is_empty(), "같이 쓸지 정하지 않은 설정 칸 (settings_scope.rs 표에 적기): {missing:?}");
        // 표에 있는데 실제로는 없는 칸 (이름이 바뀌었거나 지운 칸)
        for (p, _) in SETTINGS_SCOPE {
            let mut parts = p.split('/');
            let mut cur = d.get(parts.next().unwrap());
            for part in parts {
                cur = cur.and_then(|c| c.get(part));
            }
            assert!(cur.is_some(), "표에만 있는 칸: {p}");
        }
    }

    #[test]
    fn shared_part_leaves_device_fields_out() {
        let mut d = defaults();
        d["app-setting"]["selectedApiProfile"] = "p-local".into();
        d["app-setting"]["selectedPresetX"] = "unknown".into(); // 표에 없는 칸
        d["feature-setting"]["feature"] = serde_json::json!({ "translate": true });
        d["plugin-setting"]["translate"]["autoTranslate"] = true.into();
        d["plugin-setting"]["translate"]["targetLang"] = "日本語".into();
        let s = shared_part(&d);
        let text = Value::Object(s.clone()).to_string();
        for leaked in [
            "p-local",
            "selectedApiProfile",
            "autoTranslate",
            "feature-setting",
            "unknown",
            "isFrameless",
            "trashRetentionDays",
        ] {
            assert!(!text.contains(leaked), "기기 전용 칸이 묶음에 들어감: {leaked}");
        }
        assert_eq!(s["plugin-setting"]["translate"]["targetLang"], "日本語");
        assert_eq!(s["theme-setting"]["selectTheme"], "default");
    }

    #[test]
    fn merge_keeps_this_devices_fields() {
        let mut desk = defaults();
        desk["app-setting"]["selectedApiProfile"] = "p-local".into();
        desk["theme-setting"]["fontSize"] = 18.into();
        desk["feature-setting"]["feature"] = serde_json::json!({ "widget": true });

        let mut phone = defaults();
        phone["app-setting"]["selectedApiProfile"] = "p-cloud".into();
        phone["app-setting"]["selectedPersonaId"] = "me".into();
        phone["theme-setting"]["selectTheme"] = "forest".into();
        phone["theme-setting"]["fontSize"] = 14.into();
        phone["plugin-setting"]["tts"]["autoRead"] = true.into();
        phone["plugin-setting"]["tts"]["voice"] = "alloy".into();

        merge_shared(&mut desk, &shared_part(&phone));
        // 같이 쓰는 것은 받고
        assert_eq!(desk["app-setting"]["selectedPersonaId"], "me");
        assert_eq!(desk["theme-setting"]["selectTheme"], "forest");
        assert_eq!(desk["plugin-setting"]["tts"]["voice"], "alloy");
        // 이 기기 것은 그대로
        assert_eq!(desk["app-setting"]["selectedApiProfile"], "p-local");
        assert_eq!(desk["theme-setting"]["fontSize"], 18);
        assert_eq!(desk["feature-setting"]["feature"]["widget"], true);
        assert_eq!(desk["plugin-setting"]["tts"]["autoRead"], false, "자동 읽기는 기기마다 켠다");
        // 합친 결과도 설정 구조체로 읽힌다
        let _: AppSetting = serde_json::from_value(desk["app-setting"].clone()).unwrap();
        let _: PluginSetting = serde_json::from_value(desk["plugin-setting"].clone()).unwrap();

        // 모르는 칸은 받지 않는다
        let mut incoming = shared_part(&phone);
        incoming["app-setting"]["futureField"] = 1.into();
        merge_shared(&mut desk, &incoming);
        assert!(desk["app-setting"].get("futureField").is_none());
    }

    #[test]
    fn backgrounds_are_merged_not_overwritten() {
        let mut local = defaults();
        local["theme-setting"]["backgrounds"] = serde_json::json!([r"C:\data\generated\a.png", "C:/data/backgrounds/b.png"]);
        let mut other = defaults();
        // 다른 기기는 a를 아직 모르고 c를 새로 넣었다 (b는 구분자 모양만 다름)
        other["theme-setting"]["backgrounds"] = serde_json::json!([r"C:\data\backgrounds\b.png", "C:/data/backgrounds/c.png"]);
        merge_shared(&mut local, &shared_part(&other));
        assert_eq!(
            local["theme-setting"]["backgrounds"],
            serde_json::json!([r"C:\data\generated\a.png", "C:/data/backgrounds/b.png", "C:/data/backgrounds/c.png"])
        );
        assert!(same_path(r"C:\x\y.png", "C:/x/y.png"));
        assert!(!same_path("C:/x/y.png", "C:/x/z.png"));
    }
}
