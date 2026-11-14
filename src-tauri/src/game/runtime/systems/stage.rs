//! 무대(stage_json)의 유일한 writer. 연출을 "내보내는" 일도 여기서 한다:
//!   - 씬이 열리면 그 씬의 대본(game_scenes.script)을 줄/연출로 파싱해 SceneScript로 내보낸다.
//!   - 위치/시간대/날씨가 바뀌면 배경을 다시 고른다. 지금 배경이 새 조건의 최적 후보 안에 있으면 그대로 둔다.
//!   - 위치가 바뀌면 CG와 서 있던 캐릭터를 내린다. 씬이 끝나면 CG를 내린다 (탐험 중에 CG가 계속 남지 않게).
//!   - 이벤트 효과 stage(StageRequested)를 대본 문법으로 바꿔 같은 파서로 해석한다.
//! AI 응답의 연출은 npc_say가 GameRuntime::parse_ai_stage/commit_stage로 같은 상태에 반영한다.
use super::super::assets::{asset_ref, DefResolver};
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::GameSystem;
use crate::game::stage::{self, apply_cue_to_stage, Allowed, Cue, LineKind, StageLine};
use rand::rngs::StdRng;
use rand::RngExt;

pub struct StageSystem;

fn direction(cues: Vec<Cue>) -> StageLine {
    StageLine { kind: LineKind::Direction, speaker: None, text: String::new(), cues }
}

fn commit(state: &mut GameState, lines: &[StageLine]) {
    for l in lines {
        for c in &l.cues {
            apply_cue_to_stage(&mut state.stage_json, c);
        }
    }
}

fn background(def: &GameDefinition, state: &mut GameState, rng: &mut StdRng) -> Vec<GameEvent> {
    let cands = def.background_candidates(state.location_id.as_deref(), Some(state.time_period_id.as_str()), Some(state.weather_id.as_str()));
    let current = state.stage_json.get("background").and_then(|b| b.get("id")).and_then(|v| v.as_str()).map(str::to_string);
    let cue = if cands.is_empty() {
        if current.is_none() {
            return vec![];
        }
        Cue::BackgroundOff
    } else {
        if cands.iter().any(|c| Some(c.base.id.as_str()) == current.as_deref()) {
            return vec![]; // 지금 배경이 여전히 최적 후보 — 안 바꾼다
        }
        let pick = cands[rng.random_range(0..cands.len())];
        Cue::Background { asset: asset_ref(&pick.base) }
    };
    let lines = vec![direction(vec![cue])];
    commit(state, &lines);
    vec![GameEvent::StageLines { lines }]
}

fn scene_script(def: &GameDefinition, state: &mut GameState, scene_id: &str, rng: &mut StdRng) -> Vec<GameEvent> {
    let Some(script) = def.scene(scene_id).and_then(|s| s.script.as_deref()).filter(|t| !t.trim().is_empty()) else {
        return vec![];
    };
    let mut resolver = DefResolver { def, rng, ai: false, player_name: None };
    let lines = stage::parse(script, &mut resolver, &Allowed::author());
    commit(state, &lines);

    // 대본에 나오는 인물(화자, 표정/모션 대상)은 이 장소에 있는 것으로 한다 — 씬이 끝난 뒤 갑자기 사라지면 어색하다.
    let mut appeared: Vec<String> = Vec::new();
    for l in &lines {
        if let Some(sp) = l.speaker.as_deref().filter(|s| *s != "player") {
            appeared.push(sp.to_string());
        }
        for c in &l.cues {
            match c {
                Cue::Standing { character_id, .. } | Cue::Motion { character_id, .. } => appeared.push(character_id.clone()),
                _ => {}
            }
        }
    }
    appeared.sort();
    appeared.dedup();

    let mut out = vec![GameEvent::SceneScript { scene_id: scene_id.to_string(), lines }];
    if !appeared.is_empty() {
        out.push(GameEvent::CharactersAppeared { character_ids: appeared });
    }
    out
}

/// CG와 스탠딩을 내리는 큐 (있는 것만).
fn clear_cues(state: &GameState, cg: bool, standings: bool) -> Vec<Cue> {
    let mut cues = Vec::new();
    if cg && state.stage_json.get("cg").is_some() {
        cues.push(Cue::CgOff);
    }
    if standings {
        if let Some(m) = state.stage_json.get("standings").and_then(|s| s.as_object()) {
            let mut ids: Vec<&String> = m.keys().collect();
            ids.sort();
            cues.extend(ids.into_iter().map(|id| Cue::StandingOff { character_id: id.clone() }));
        }
    }
    cues
}

impl GameSystem for StageSystem {
    fn name(&self) -> &'static str {
        "stage"
    }

    fn handle_event(&self, def: &GameDefinition, state: &mut GameState, event: &GameEvent, rng: &mut StdRng) -> Vec<GameEvent> {
        match event {
            GameEvent::GameStarted => {
                let mut out = background(def, state, rng);
                if let Some(scene_id) = state.current_scene_id.clone() {
                    out.extend(scene_script(def, state, &scene_id, rng)); // 시작 씬은 SceneChanged 없이 열려 있다
                }
                out
            }
            GameEvent::SceneChanged { scene_id } => scene_script(def, state, scene_id, rng),
            GameEvent::SceneExited { .. } => {
                let cues = clear_cues(state, true, false);
                if cues.is_empty() {
                    return vec![];
                }
                let lines = vec![direction(cues)];
                commit(state, &lines);
                vec![GameEvent::StageLines { lines }]
            }
            GameEvent::LocationChanged { .. } => {
                let mut out = Vec::new();
                let cues = clear_cues(state, true, true);
                if !cues.is_empty() {
                    let lines = vec![direction(cues)];
                    commit(state, &lines);
                    out.push(GameEvent::StageLines { lines });
                }
                out.extend(background(def, state, rng));
                out
            }
            GameEvent::TimePeriodChanged { .. } | GameEvent::WeatherChanged { .. } => background(def, state, rng),
            GameEvent::StageRequested { spec } => {
                let script = stage::spec_to_script(spec);
                let mut resolver = DefResolver { def, rng, ai: false, player_name: None };
                let lines = stage::parse(&script, &mut resolver, &Allowed::author());
                if lines.is_empty() {
                    return vec![];
                }
                commit(state, &lines);
                vec![GameEvent::StageLines { lines }]
            }
            _ => vec![],
        }
    }
}
