//! 프론트(게임 iframe/호스트 UI)가 화면에 그리는 데이터. GameDefinition에서 "보여줄 것만" 추린다.
//! DB 접근이 없다 — 플레이가 시작될 때 이미 메모리에 올라온 정의에서 만든다.
//!
//! 일부러 안 주는 것:
//!   - 이벤트의 조건/효과/NPC 후보 풀, 인카운터 테이블, 선택지의 효과/다음 씬/조건
//!     (화면엔 필요 없고, 게임 스크립트가 보고 엔진을 속일 단서가 된다. 선택 가능 여부는
//!      RenderState.available_choices로 엔진이 계산해서 준다.)
//! 이동 경로 조건은 플레이 UI가 잠긴 경로와 필요한 요구사항을 표시하도록 전달한다.
//!   - 캐릭터의 system_prompt_override / example_dialogue (프롬프트 유출 방지)
//!   - 날씨의 인카운터 배율, 월드인포
//! 현재 변수 값은 여기가 아니라 RenderState.flags에 있다 — 여기엔 "변수가 뭔지"(이름/설명/범위)만 있다.
//! TODO: 인벤토리 시스템이 생기면 아이템 정의(이름/아이콘/설명)를 여기에 추가하고,
//!   "지금 가진 것"은 RenderState 쪽에 추가한다.
//! json!으로 만드는 이유: 필드 타입이 모델마다 Option/String으로 달라서, 구조체로 옮기면
//! 모델이 바뀔 때마다 이 파일이 같이 깨진다. 키 이름(camelCase)이 프론트와의 계약이다.
use super::definition::GameDefinition;
use serde_json::{json, Value};

fn parse_asset_path(game_dir: &str, file_path: &str) -> String {
    format!("{}\\{}", game_dir, file_path.trim_start_matches('/'))
}

impl GameDefinition {
    pub fn view(&self, game_dir: &str) -> Value {
        json!({
            "game": {
                "id": self.project.id,
                "title": self.project.title,
                "description": self.project.description,
                "coverImage": self.project.cover_image,
                "customCss": self.project.custom_css,
                "settings": self.project.settings_json,
            },
            // 이 게임의 템플릿과 capabilities. 프론트가 지도/인벤토리 같은 UI를 켜고 끄는 데 쓴다. (엔진이 강제하지는 않는다)
            "template": crate::game::template::find(&self.project.template_id),
            "maps": self.maps.iter().map(|m| json!({
                "id": m.id, "name": m.name, "dangerLevel": m.danger_level, "settings": m.settings_json,
            })).collect::<Vec<_>>(),
            "zones": self.zones.iter().map(|z| json!({
                "id": z.id, "mapId": z.map_id, "name": z.name, "tags": z.tags, "settings": z.settings_json,
            })).collect::<Vec<_>>(),
            "locations": self.locations.iter().map(|l| json!({
                "id": l.id, "zoneId": l.zone_id, "name": l.name, "description": l.description, "settings": l.settings_json,
            })).collect::<Vec<_>>(),
            // 이동 조건은 플레이 UI에서 필요한 아이템/상태를 안내하기 위한 표시용으로 보낸다.
            "connections": self.location_connections.iter().map(|c| json!({
                "fromId": c.from_id, "toId": c.to_id, "travelSeconds": c.travel_seconds,
                "condition": c.condition, "settings": c.settings_json,
            })).collect::<Vec<_>>(),
            "interactables": self.interactables.values().flatten().map(|i| json!({
                "id": i.id, "locationId": i.location_id, "name": i.name,
            })).collect::<Vec<_>>(),
            "characters": self.characters.values().map(|c| json!({
                "id": c.id, "name": c.name, "avatar": c.avatar, "description": c.description,
                "firstMessage": c.first_message, "tags": c.tags, "settings": c.settings_json,
            })).collect::<Vec<_>>(),
            "timePeriods": self.time_periods.iter().map(|p| json!({
                "id": p.id, "name": p.name, "startMinute": p.start_minute, "endMinute": p.end_minute,
            })).collect::<Vec<_>>(),
            "weatherTypes": self.weather_types.iter().map(|w| json!({ "id": w.id, "name": w.name })).collect::<Vec<_>>(),
            "scenes": self.scenes.iter().map(|s| json!({
                "id": s.id, "sceneKey": s.scene_key, "name": s.scene_name, "description": s.description,
                "isEnding": s.is_ending, "settings": s.settings_json,
                "choices": s.choices.iter().flatten().map(|c| json!({ "id": c.id, "text": c.text, "sortOrder": c.sort_order })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            // 아이템 정의 (표시용). 사용 효과/조건/획득 이벤트는 안 준다 — usable만 알려준다.
            // 지금 가진 수량은 RenderState.inventory.
            "items": self.items.values().map(|i| json!({
                "id": i.id, "name": i.name, "description": i.description, "icon": i.icon, "kind": i.kind,
                "maxStack": i.max_stack, "isUnique": i.is_unique, "isKey": i.is_key, "consumable": i.consumable,
                "usable": !(i.use_effect.is_null() || i.use_effect.as_object().map_or(false, |o| o.is_empty())),
                "settings": i.settings_json,
            })).collect::<Vec<_>>(),
            // 관계의 고정 축 이름. 현재 관계는 RenderState.relationships ("보낸쪽>받는쪽" 키).
            "relationshipAxes": crate::game::engine::RELATION_AXES,
            // 에셋 목록 (파일 경로는 게임 폴더 기준 상대경로 — iframe이 그대로 쓴다). 생성 프롬프트/시드 같은 제작 메타는 안 준다.
            "assets": {
                "audio": self.assets.audio.iter().map(|a| json!({
                    "id": a.base.id, "key": a.base.key, "category": a.base.category, "path": parse_asset_path(&game_dir, &a.base.file_path),
                    "settings": a.base.settings_json, "tag": a.tag, "characterId": a.game_character_id, "transcript": a.transcript,
                })).collect::<Vec<_>>(),
                "characters": self.assets.characters.iter().map(|a| json!({
                    "id": a.base.id, "key": a.base.key, "path": parse_asset_path(&game_dir, &a.base.file_path),
                    "settings": a.base.settings_json, "characterId": a.game_character_id, "usage": a.usage, "emotion": a.emotion,
                })).collect::<Vec<_>>(),
                "cgs": self.assets.cgs.iter().map(|a| json!({
                    "id": a.base.id, "key": a.base.key, "path": parse_asset_path(&game_dir, &a.base.file_path),
                    "settings": a.base.settings_json, "characterId": a.game_character_id, "description": a.description,
                })).collect::<Vec<_>>(),
                "backgrounds": self.assets.backgrounds.iter().map(|a| json!({
                    "id": a.base.id, "key": a.base.key, "path": parse_asset_path(&game_dir, &a.base.file_path),
                    "settings": a.base.settings_json, "locationId": a.location_id, "timeId": a.time_id, "weatherId": a.weather_id,
                })).collect::<Vec<_>>(),
                "other": self.assets.all.iter().filter(|a| matches!(a.category.as_str(), "font" | "other")).map(|a| json!({
                    "id": a.id, "key": a.key, "category": a.category, "path": parse_asset_path(&game_dir, &a.file_path), "settings": a.settings_json,
                })).collect::<Vec<_>>(),
            },
            // 기본 제공 연출 목록 (프론트가 구현해야 하는 것들)
            "stageCatalog": { "bgmMoods": crate::game::stage::BGM_MOODS, "screenEffects": crate::game::stage::SCREEN_EFFECTS, "motions": crate::game::stage::MOTIONS },
            "endings": self.endings.iter().map(|e| json!({
                "id": e.id, "endingKey": e.ending_key, "title": e.title, "description": e.description,
            })).collect::<Vec<_>>(),
            // 게임 변수의 정의. min/max는 UI(게이지 등)에도 쓰이도록 config_json에서 꺼낸다.
            // ai_readable/ai_writable 같은 AI 접근 설정은 안 준다.
            "variables": self.state_defs.iter().map(|d| {
                let cfg: Value = serde_json::from_str(&d.config_json).unwrap_or(Value::Null);
                json!({
                    "key": d.key, "description": d.description, "type": d.def_type,
                    "default": serde_json::from_str::<Value>(&d.default_value).unwrap_or(Value::Null),
                    "min": cfg.get("min"), "max": cfg.get("max"),
                })
            }).collect::<Vec<_>>(),
        })
    }
}
