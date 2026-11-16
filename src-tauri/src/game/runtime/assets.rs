//! 에셋을 "고르는" 규칙과, 텍스트 연출(stage.rs)이 쓰는 키 해석기.
//! 같은 조건에 에셋이 여러 개면 그중 랜덤으로 하나를 고른다 (같은 장소라도 구도가 다르게 보이게).
//! 뽑힌 결과(AssetRef)는 StageLine/무대 상태에 박혀서 저장되므로, 다시 볼 때는 다시 굴리지 않는다.
use super::definition::GameDefinition;
use crate::game::stage::{AssetRef, StageResolver};
use crate::models::game::{BackgroundAssets, CharacterAssets, GameAssets};
use rand::rngs::StdRng;
use rand::RngExt;

pub fn asset_ref(a: &GameAssets) -> AssetRef {
    AssetRef {
        id: a.id.clone(),
        key: a.key.clone(),
        path: a.file_path.trim_start_matches('/').to_string(),
        settings: a.settings_json.clone().unwrap_or(serde_json::Value::Null),
    }
}

fn pick<'a, T>(items: Vec<&'a T>, rng: &mut StdRng) -> Option<&'a T> {
    if items.is_empty() {
        None
    } else {
        let i = rng.random_range(0..items.len());
        Some(items[i])
    }
}

impl GameDefinition {
    /// 지금의 위치/시간대/날씨에 맞는 배경. 후보의 비어 있지 않은 조건은 모두 일치해야 하고, 일치하는 조건이 많을수록
    /// (위치 > 시간대 > 날씨 순으로 무겁게) 우선한다: 위치+시간+날씨 → 위치+시간 → 위치+날씨 → 위치 → 시간+날씨 → ... → 전체 기본.
    /// 같은 점수의 후보가 여럿이면 랜덤.
    pub fn pick_background(&self, location: Option<&str>, time: Option<&str>, weather: Option<&str>, rng: &mut StdRng) -> Option<&BackgroundAssets> {
        pick(self.background_candidates(location, time, weather), rng)
    }

    /// 가장 잘 맞는 점수의 후보 전부. 지금 보이는 배경이 이 안에 있으면 바꾸지 않는다 (시간대가 바뀔 때마다 구도가 흔들리지 않게).
    pub fn background_candidates(&self, location: Option<&str>, time: Option<&str>, weather: Option<&str>) -> Vec<&BackgroundAssets> {
        let mut best: Vec<&BackgroundAssets> = Vec::new();
        let mut best_score = -1i32;
        for b in &self.assets.backgrounds {
            let mut score = 0;
            let mut ok = true;
            for (cond, current, weight) in [(&b.location_id, location, 4), (&b.time_id, time, 2), (&b.weather_id, weather, 1)] {
                match cond {
                    None => {}
                    Some(c) if Some(c.as_str()) == current => score += weight,
                    Some(_) => ok = false,
                }
            }
            if !ok {
                continue;
            }
            if score > best_score {
                best_score = score;
                best.clear();
            }
            if score == best_score {
                best.push(b);
            }
        }
        best
    }

    /// 캐릭터의 스탠딩 중 감정이 맞는 것 (여러 개면 랜덤). 감정명은 대소문자를 구분하지 않는다.
    pub fn pick_standing(&self, character_id: &str, emotion: &str, rng: &mut StdRng) -> Option<&CharacterAssets> {
        let want = emotion.trim().to_lowercase();
        let c: Vec<&CharacterAssets> = self
            .assets
            .characters
            .iter()
            .filter(|a| {
                a.usage == "standing"
                    && a.game_character_id.as_deref() == Some(character_id)
                    && a.emotion.as_deref().map(|e| e.trim().to_lowercase()).as_deref() == Some(want.as_str())
            })
            .collect();
        pick(c, rng)
    }

    /// 스탠딩이 가진 감정명 목록 (AI에게 보여줄 목록). 중복 제거, 정렬.
    pub fn standing_emotions(&self, character_id: &str) -> Vec<String> {
        let mut v: Vec<String> = self
            .assets
            .characters
            .iter()
            .filter(|a| a.usage == "standing" && a.game_character_id.as_deref() == Some(character_id))
            .filter_map(|a| a.emotion.clone())
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// 효과음 그룹 이름 목록 (AI에게 보여줄 목록: 파일명은 보내지 않는다).
    pub fn sfx_tags(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .assets
            .audio
            .iter()
            .filter(|a| a.base.category == "sfx")
            .filter_map(|a| a.tag.clone())
            .collect();
        v.sort();
        v.dedup();
        v
    }

    pub fn pick_sfx(&self, tag: &str, rng: &mut StdRng) -> Option<&GameAssets> {
        let c: Vec<&GameAssets> = self
            .assets
            .audio
            .iter()
            .filter(|a| a.base.category == "sfx" && a.tag.as_deref() == Some(tag))
            .map(|a| &a.base)
            .collect();
        pick(c, rng)
    }

    /// 분위기에 맞는 곡. 없으면 default 분위기로 대체한다.
    pub fn pick_bgm(&self, mood: &str, rng: &mut StdRng) -> Option<&GameAssets> {
        for m in [mood, "default"] {
            let c: Vec<&GameAssets> = self
                .assets
                .audio
                .iter()
                .filter(|a| a.base.category == "bgm" && a.tag.as_deref() == Some(m))
                .map(|a| &a.base)
                .collect();
            if let Some(found) = pick(c, rng) {
                return Some(found);
            }
        }
        None
    }

    pub fn voice(&self, key: &str) -> Option<&GameAssets> {
        self.assets
            .audio
            .iter()
            .find(|a| a.base.category == "voice" && a.base.key == key)
            .map(|a| &a.base)
    }

    /// AI가 쓸 수 있는 CG (ai_usable). (key, description)
    pub fn ai_cgs(&self) -> Vec<(String, String)> {
        self.assets
            .cgs
            .iter()
            .filter(|c| c.ai_usable)
            .map(|c| (c.base.key.clone(), c.description.clone().unwrap_or_default()))
            .collect()
    }
}

/// stage.rs의 파서가 쓰는 해석기. ai=true이면 CG는 ai_usable인 것만 풀린다.
pub struct DefResolver<'a> {
    pub def: &'a GameDefinition,
    pub rng: &'a mut StdRng,
    pub ai: bool,
    /// 페르소나 이름 (플레이어 대사 접두 `이름:`으로도 쓸 수 있게). 없으면 '나', 'player'만.
    pub player_name: Option<String>,
}

impl StageResolver for DefResolver<'_> {
    fn character_by_name(&self, name: &str) -> Option<String> {
        let n = name.trim().to_lowercase();
        self.def
            .characters
            .values()
            .find(|c| c.name.trim().to_lowercase() == n || c.id.to_lowercase() == n)
            .map(|c| c.id.clone())
    }

    fn is_player_name(&self, name: &str) -> bool {
        let n = name.trim().to_lowercase();
        matches!(n.as_str(), "나" | "player" | "플레이어") || self.player_name.as_deref().map_or(false, |p| p.trim().to_lowercase() == n)
    }

    fn bgm(&mut self, mood: &str) -> Option<AssetRef> {
        self.def.pick_bgm(mood, self.rng).map(asset_ref)
    }

    fn sfx(&mut self, tag: &str) -> Option<AssetRef> {
        self.def.pick_sfx(tag, self.rng).map(asset_ref)
    }

    fn cg(&mut self, key: &str) -> Option<AssetRef> {
        self.def
            .assets
            .cgs
            .iter()
            .find(|c| c.base.key == key && (!self.ai || c.ai_usable))
            .map(|c| asset_ref(&c.base))
    }

    fn voice(&mut self, key: &str) -> Option<AssetRef> {
        self.def.voice(key).map(asset_ref)
    }

    fn standing(&mut self, character_id: &str, emotion: &str) -> Option<AssetRef> {
        self.def.pick_standing(character_id, emotion, self.rng).map(|a| asset_ref(&a.base))
    }
}
