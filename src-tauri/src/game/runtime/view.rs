//! 프론트(게임 iframe/호스트 UI)가 화면에 그리는 데이터. GameDefinition에서 "보여줄 것만" 추린다.
//! DB 접근이 없다 — 플레이가 시작될 때 이미 메모리에 올라온 정의에서 만든다.
//!
//! 일부러 안 주는 것:
//!   - 이벤트의 조건/효과/NPC 후보 풀, 인카운터 테이블, 이동 조건, 선택지의 효과/다음 씬/조건
//!     (화면엔 필요 없고, 게임 스크립트가 보고 엔진을 속일 단서가 된다. 선택 가능 여부는
//!      RenderState.available_choices로 엔진이 계산해서 준다.)
//!   - 캐릭터의 system_prompt_override / example_dialogue (프롬프트 유출 방지)
//!   - 날씨의 인카운터 배율, 월드인포
//! 현재 변수 값은 여기가 아니라 RenderState.flags에 있다 — 여기엔 "변수가 뭔지"(이름/설명/범위)만 있다.
//! TODO: 인벤토리 시스템이 생기면 아이템 정의(이름/아이콘/설명)를 여기에 추가하고,
//!   "지금 가진 것"은 RenderState 쪽에 추가한다.
//! json!으로 만드는 이유: 필드 타입이 모델마다 Option/String으로 달라서, 구조체로 옮기면
//! 모델이 바뀔 때마다 이 파일이 같이 깨진다. 키 이름(camelCase)이 프론트와의 계약이다.
use super::definition::GameDefinition;
use serde_json::{json, Value};

/// 정적 데이터의 내용 해시 (FNV-1a 64비트, 16자리 hex). 프론트가 캐시해둔 데이터가 아직 유효한지 비교하는 용도다 (보안용이 아니다).
/// serde_json::Value의 객체는 키 순서가 고정이고(BTreeMap), HashMap에서 나온 목록은 view()가 id로 정렬하므로
/// 내용이 같으면 로드할 때마다 같은 값이 나온다.
fn content_hash(view: &Value) -> String {
    let s = serde_json::to_string(view).unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

impl GameDefinition {
    /// view()와 그 내용 해시. 정의가 바뀌면(에디터 저장 후 다시 열기 등) 해시가 달라진다.
    pub fn view_with_hash(&self) -> (Value, String) {
        let view = self.view();
        let hash = content_hash(&view);
        (view, hash)
    }

    pub fn view(&self) -> Value {
        // HashMap에서 나오는 목록은 id로 정렬한다 — 안 그러면 로드할 때마다 순서가 달라져서 contentHash가 매번 바뀐다.
        // (DB에서 읽은 목록은 순서가 정해져 있고, timePeriods/scenes/choices처럼 sort_order가 의미 있는 건 그대로 둔다.)
        let mut characters: Vec<&_> = self.characters.values().collect();
        characters.sort_by(|a, b| a.id.cmp(&b.id));
        let mut items: Vec<&_> = self.items.values().collect();
        items.sort_by(|a, b| a.id.cmp(&b.id));
        let mut interactables: Vec<&_> = self.interactables.values().flatten().collect();
        interactables.sort_by(|a, b| a.id.cmp(&b.id));

        // 게임이 쓰지 않는 기능(capabilities)의 정적 데이터는 내보내지 않는다 (빈 배열).
        let caps = self.capabilities();
        let movement = caps.movement();
        let mut template = crate::game::template::find(&self.project.template_id);
        if let Some(t) = &mut template {
            t.capabilities = caps.clone(); // 게임 설정으로 끈 것까지 반영한 실제 값
        }

        json!({
            "game": {
                "id": self.project.id,
                "title": self.project.title,
                "description": self.project.description,
                "coverImage": self.project.cover_image,
                "customCss": self.project.custom_css,
                "settings": self.project.settings_json,
            },
            // 이 게임의 템플릿과 실제로 쓰는 capabilities (템플릿 값에서 게임 설정이 끈 것을 뺀 것).
            // map/location/time은 엔진이 강제한다: 꺼진 기능의 데이터는 아래에서 빈 배열로 나가고 해당 커맨드는 거절된다.
            "template": template,
            "capabilities": caps,
            "maps": self.maps.iter().filter(|_| movement).map(|m| json!({
                "id": m.id, "name": m.name, "dangerLevel": m.danger_level, "settings": m.settings_json,
            })).collect::<Vec<_>>(),
            "zones": self.zones.iter().filter(|_| movement).map(|z| json!({
                "id": z.id, "mapId": z.map_id, "name": z.name, "tags": z.tags, "settings": z.settings_json,
            })).collect::<Vec<_>>(),
            "locations": self.locations.iter().filter(|_| caps.location).map(|l| json!({
                "id": l.id, "zoneId": l.zone_id, "name": l.name, "description": l.description, "settings": l.settings_json,
            })).collect::<Vec<_>>(),
            // 이동 조건은 안 준다. 막혀 있으면 MoveTo가 에러로 알려준다.
            "connections": self.location_connections.iter().filter(|_| movement).map(|c| json!({
                "fromId": c.from_id, "toId": c.to_id, "travelSeconds": c.travel_seconds, "settings": c.settings_json,
            })).collect::<Vec<_>>(),
            "interactables": interactables.iter().filter(|_| caps.location).map(|i| json!({
                "id": i.id, "locationId": i.location_id, "name": i.name,
            })).collect::<Vec<_>>(),
            "characters": characters.iter().map(|c| json!({
                "id": c.id, "name": c.name, "avatar": c.avatar, "description": c.description,
                "firstMessage": c.first_message, "tags": c.tags, "settings": c.settings_json,
            })).collect::<Vec<_>>(),
            "timePeriods": self.time_periods.iter().filter(|_| caps.time).map(|p| json!({
                "id": p.id, "name": p.name, "startMinute": p.start_minute, "endMinute": p.end_minute,
            })).collect::<Vec<_>>(),
            "weatherTypes": self.weather_types.iter().filter(|_| caps.time).map(|w| json!({ "id": w.id, "name": w.name })).collect::<Vec<_>>(),
            "scenes": self.scenes.iter().map(|s| json!({
                "id": s.id, "sceneKey": s.scene_key, "name": s.scene_name, "description": s.description,
                "isEnding": s.is_ending, "settings": s.settings_json,
                // 씬 입력(sceneSay)을 쓸 수 있는 씬인가: 엔딩이 아니고 aiContinue가 꺼져 있지 않다.
                "aiChat": s.is_ending != Some(true) && super::definition::scene_flag(s, "aiContinue", true),
                "choices": s.choices.iter().flatten().map(|c| json!({ "id": c.id, "text": c.text, "sortOrder": c.sort_order })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            // 아이템 정의 (표시용). 사용 효과/조건/획득 이벤트는 안 준다 — usable만 알려준다.
            // 지금 가진 수량은 RenderState.inventory.
            "items": items.iter().map(|i| json!({
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
                    "id": a.base.id, "key": a.base.key, "category": a.base.category, "path": a.base.file_path.trim_start_matches('/'),
                    "settings": a.base.settings_json, "tag": a.tag, "characterId": a.game_character_id, "transcript": a.transcript,
                })).collect::<Vec<_>>(),
                "characters": self.assets.characters.iter().map(|a| json!({
                    "id": a.base.id, "key": a.base.key, "path": a.base.file_path.trim_start_matches('/'),
                    "settings": a.base.settings_json, "characterId": a.game_character_id, "usage": a.usage, "emotion": a.emotion,
                })).collect::<Vec<_>>(),
                "cgs": self.assets.cgs.iter().map(|a| json!({
                    "id": a.base.id, "key": a.base.key, "path": a.base.file_path.trim_start_matches('/'),
                    "settings": a.base.settings_json, "characterId": a.game_character_id, "description": a.description,
                })).collect::<Vec<_>>(),
                "backgrounds": self.assets.backgrounds.iter().map(|a| json!({
                    "id": a.base.id, "key": a.base.key, "path": a.base.file_path.trim_start_matches('/'),
                    "settings": a.base.settings_json, "locationId": a.location_id, "timeId": a.time_id, "weatherId": a.weather_id,
                })).collect::<Vec<_>>(),
                "other": self.assets.all.iter().filter(|a| matches!(a.category.as_str(), "font" | "other")).map(|a| json!({
                    "id": a.id, "key": a.key, "category": a.category, "path": a.file_path.trim_start_matches('/'), "settings": a.settings_json,
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
