//! 템플릿 = 게임 엔진 안에서 쓰이는 "시스템 규칙" (id, 버전, 쓸 수 있는 기능 capabilities).
//! 템플릿 패키지 = 그 템플릿으로 새 게임을 만들 때 복사되는 시작 데이터 (.tgame 파일).
//! 둘은 앱에 같이 포함해서 배포한다 (include_str!/include_bytes! — 별도 리소스 설정 없이 모든 플랫폼에서 동작).
//! 사용자 템플릿은 지원하지 않는다. 템플릿을 추가하려면 resources/templates/<id>/ 에
//! template.json 과 package.tgame 을 만들고 아래 BUNDLED에 한 줄 추가한다.
//!
//! ⚠ capabilities는 지금 "선언"일 뿐이다 — 엔진이 강제하지 않고 get_game_view로 프론트에 알려주기만 한다
//!   (프론트가 지도/인벤토리 UI를 숨기는 데 쓴다). location/time 기능 끄기는 나중에 구현.
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
    /// 위치(location) 개념을 쓰는가. 빠져 있으면 켜진 것으로 본다. (엔진 강제는 아직 없음)
    #[serde(default = "yes")]
    pub location: bool,
    /// 시간/날씨 개념을 쓰는가. 빠져 있으면 켜진 것으로 본다. (엔진 강제는 아직 없음)
    #[serde(default = "yes")]
    pub time: bool,
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
}
