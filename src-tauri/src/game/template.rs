//! 템플릿 = 게임 엔진 안에서 쓰이는 "시스템 규칙" (id, 버전, 쓸 수 있는 기능 capabilities).
//! 템플릿 패키지 = 그 템플릿으로 새 게임을 만들 때 복사되는 시작 데이터 (.tgame 파일).
//! 둘은 앱에 같이 포함해서 배포한다 (include_str!/include_bytes! — 별도 리소스 설정 없이 모든 플랫폼에서 동작).
//! 사용자 템플릿은 지원하지 않는다. 템플릿을 추가하려면 resources/templates/<id>/ 에
//! template.json 과 package.tgame 을 만들고 아래 BUNDLED에 한 줄 추가한다.
//!
//! capabilities 중 map/location/time은 엔진이 강제한다 (GameDefinition::capabilities):
//!   - map 또는 location이 꺼지면 이동(MoveTo)이 막히고 지도 데이터/현재 맵·존이 화면용 데이터에서 빠진다.
//!     location이 꺼지면 상호작용(Interact)도 막히고 위치 목록/현재 위치까지 빠진다.
//!   - time이 꺼지면 시간 진행(AdvanceTime)이 막히고 이동해도 시간이 흐르지 않으며, 시간/시간대/날씨가 빠진다.
//! 게임은 project.settings_json의 {"capabilities": {"map": false}}로 템플릿의 기능을 **끄기만** 할 수 있다(켤 수는 없다).
//! 나머지(choices/freeText/inventory/aiNarration)는 아직 선언일 뿐이고 프론트가 UI를 숨기는 데 쓴다.
use serde::{Deserialize, Serialize};

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    #[serde(default)]
    pub choices: bool,
    #[serde(default)]
    pub free_text: bool,
    #[serde(default)]
    pub inventory: bool,
    #[serde(default)]
    pub map: bool,
    #[serde(default)]
    pub ai_narration: bool,
    /// 위치(location) 개념을 쓰는가. 빠져 있으면 켜진 것으로 본다.
    #[serde(default = "yes")]
    pub location: bool,
    /// 시간/날씨 개념을 쓰는가. 빠져 있으면 켜진 것으로 본다.
    #[serde(default = "yes")]
    pub time: bool,
}

impl Capabilities {
    /// 전부 켜진 상태. 템플릿을 찾지 못한 게임(옛 데이터)은 이전처럼 아무것도 막지 않는다.
    pub fn all() -> Self {
        Self { choices: true, free_text: true, inventory: true, map: true, ai_narration: true, location: true, time: true }
    }

    /// 게임 설정의 capabilities로 기능을 끈다. 값이 false인 키만 반영하고 true는 무시한다
    /// (템플릿이 꺼둔 기능을 게임 데이터가 다시 켤 수는 없다). 객체가 아니면 그대로 둔다.
    pub fn restricted_by(mut self, overrides: Option<&serde_json::Value>) -> Self {
        let Some(o) = overrides.and_then(|v| v.as_object()) else { return self };
        let off = |key: &str| o.get(key).and_then(|v| v.as_bool()) == Some(false);
        for (key, field) in [
            ("choices", &mut self.choices),
            ("freeText", &mut self.free_text),
            ("inventory", &mut self.inventory),
            ("map", &mut self.map),
            ("aiNarration", &mut self.ai_narration),
            ("location", &mut self.location),
            ("time", &mut self.time),
        ] {
            if off(key) {
                *field = false;
            }
        }
        self
    }

    /// 플레이어가 위치를 옮길 수 있는가 (지도와 위치 개념이 둘 다 켜져 있어야 한다).
    pub fn movement(&self) -> bool {
        self.map && self.location
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub name: String,
    pub api_version: String,
    pub template_version: String,
    pub capabilities: Capabilities,
}

struct Bundled {
    template_json: &'static str,
    package: &'static [u8],
}

const BUNDLED: &[Bundled] = &[Bundled {
    template_json: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/templates/dating-sim/template.json")),
    package: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/templates/dating-sim/package.tgame")),
}];

fn parse(b: &Bundled) -> Option<Template> {
    serde_json::from_str(b.template_json).ok()
}

pub fn all() -> Vec<Template> {
    BUNDLED.iter().filter_map(parse).collect()
}

pub fn find(id: &str) -> Option<Template> {
    BUNDLED.iter().filter_map(parse).find(|t| t.id == id)
}

/// 이 템플릿으로 게임을 만들 때 가져오는 템플릿 패키지(.tgame) 바이트.
pub fn package_bytes(id: &str) -> Option<&'static [u8]> {
    BUNDLED.iter().find(|b| parse(b).map_or(false, |t| t.id == id)).map(|b| b.package)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn bundled_templates_parse_with_unique_ids() {
        let all = all();
        assert_eq!(all.len(), BUNDLED.len(), "template.json을 읽지 못한 템플릿이 있다");
        let mut ids: Vec<&str> = all.iter().map(|t| t.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len(), "템플릿 id가 겹친다");
    }

    /// 번들된 패키지의 data/definition.json이 PackageData로 읽히는지 (필드 하나만 어긋나도 import가 실패한다).
    #[test]
    fn bundled_packages_deserialize() {
        for t in all() {
            let bytes = package_bytes(&t.id).expect("패키지가 있어야 한다");
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip");
            let mut text = String::new();
            zip.by_name("data/definition.json").expect("definition.json").read_to_string(&mut text).unwrap();
            let data: crate::game::package::PackageData = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", t.id));
            assert_eq!(data.project.template_id, t.id, "패키지의 template_id가 템플릿 id와 달라야 하지 않는다");
        }
    }

    #[test]
    fn game_settings_can_only_turn_capabilities_off() {
        let base = Capabilities::all();
        let c = base.clone().restricted_by(Some(&serde_json::json!({ "map": false, "time": false, "inventory": "no" })));
        assert!(!c.map && !c.time && !c.movement());
        assert!(c.inventory && c.location, "false가 아닌 값은 무시한다");

        let mut off = Capabilities::all();
        off.time = false;
        let c = off.restricted_by(Some(&serde_json::json!({ "time": true })));
        assert!(!c.time, "템플릿이 꺼둔 기능은 게임 설정으로 켤 수 없다");

        assert!(base.restricted_by(Some(&serde_json::json!("map"))).map, "객체가 아니면 무시");
    }
}
