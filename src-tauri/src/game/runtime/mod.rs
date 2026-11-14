pub mod command;
pub mod definition;
pub mod event;
pub mod repository;
pub mod state;
mod systems;
mod assets;
mod view;

pub use command::GameCommand;
pub use definition::GameDefinition;
pub use event::GameEvent;
pub use repository::{GameRepository, SqlxGameRepository};
pub use assets::DefResolver;
pub use state::{AvailableMove, GameState, NpcState, RenderState};

use anyhow::Result;
use rand::rngs::StdRng;
use serde::Serialize;

/// 이벤트 전파를 몇 단계까지 허용할지. 데이터 설계 실수로 두 시스템이 서로
/// 계속 이벤트를 내는 순환이 생겨도, 여기서 끊겨서 앱이 안 멈춘다.
const MAX_EVENT_DEPTH: usize = 8;

fn period_tuples(definition: &GameDefinition) -> Vec<(String, u32, u32)> {
    definition.time_periods.iter().map(|p| (p.id.clone(), p.start_minute, p.end_minute)).collect()
}

/// null이거나 빈 객체면 "없음"으로 본다 (DB의 NULL이 Value::Null로 온다).
fn json_is_empty(v: &serde_json::Value) -> bool {
    v.is_null() || v.as_object().map_or(false, |o| o.is_empty())
}

/// 정의에서 사라진 위치/씬/날씨를 가리키는 상태를 되돌린다.
///  - 위치가 없거나 사라졌으면 시작 위치(없으면 첫 location)로 옮기고, zone/map도 그 위치에서 다시 구한다.
///  - 위치가 있으면 zone/map 캐시를 위치와 다시 맞춘다 (location이 다른 zone으로 옮겨졌을 수 있다).
///  - 사라진 씬은 닫고, 대기열에서도 뺀다. 사라진 날씨는 첫 날씨로 바꾼다.
fn repair_state(def: &GameDefinition, state: &mut GameState) {
    let valid_location = state.location_id.as_deref().and_then(|id| def.location(id)).map(|l| l.id.clone());
    let location_id = valid_location.or_else(|| {
        def.start_point().location_id.filter(|id| def.location(id).is_some()).or_else(|| def.locations.first().map(|l| l.id.clone()))
    });
    state.location_id = location_id.clone();
    if let Some(zone) = location_id.as_deref().and_then(|id| def.zone_of_location(id)) {
        state.zone_id = Some(zone.id.clone());
        state.map_id = zone.map_id.clone();
    }
    if !def.weather_types.iter().any(|w| w.id == state.weather_id) {
        if let Some(w) = def.weather_types.first() {
            state.weather_id = w.id.clone();
        }
    }
    // 사라진 캐릭터/위치를 가리키는 NPC 상태는 정리한다 (위치만 사라졌으면 "어디에도 없음"으로).
    state.npc_states.retain(|id, _| def.character(id).is_some());
    for n in state.npc_states.values_mut() {
        if n.location_id.as_deref().map_or(false, |l| def.location(l).is_none()) {
            n.location_id = None;
            n.status = "absent".into();
        }
    }
    // 정의에서 지워진 아이템은 인벤토리에서 뺀다.
    if let Some(inv) = state.inventory_json.as_object_mut() {
        inv.retain(|id, v| def.items.contains_key(id) && v.as_i64().map_or(false, |n| n > 0));
    }
    if state.current_scene_id.as_deref().map_or(false, |id| def.scene(id).is_none()) {
        state.current_scene_id = None;
    }
    if let Some(arr) = state.pending_scenes_json.as_array() {
        let kept: Vec<&serde_json::Value> = arr.iter().filter(|v| v.as_str().map_or(false, |id| def.scene(id).is_some())).collect();
        state.pending_scenes_json = serde_json::json!(kept);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameResponse {
    pub events: Vec<GameEvent>,
    pub state: RenderState,
}

/// 정적 데이터(GameDefinition)는 load() 시점에 한 번 다 읽어오고, 그 이후로는
/// DB를 안 건드린다. dispatch()가 sync인 이유가 이거다 — repository는 load/save에서만 쓴다.
pub struct GameRuntime {
    definition: std::sync::Arc<GameDefinition>,
    state: GameState,
    /// handle_command가 채우고 process_events가 비우는 "이번 턴의 씨앗 이벤트" 큐.
    /// dispatch() 호출 사이에는 항상 비어 있어야 한다.
    queue: Vec<GameEvent>,
    rng: StdRng,
}

impl GameRuntime {
    pub fn new(definition: std::sync::Arc<GameDefinition>, state: GameState) -> Self {
        Self { definition, state, queue: Vec::new(), rng: rand::make_rng() }
    }

    /// 새 게임 시작 — 세이브가 없는 첫 진입.
    /// 시작 location은 game_projects.start_location_id, 날씨는 settings_json의 startWeatherId를
    /// 따르고, 없으면 목록의 첫 항목으로 폴백한다 (GameDefinition::start_point 참고).
    pub fn from_new_game(definition: std::sync::Arc<GameDefinition>, session_id: String, player_id: String) -> Result<Self> {
        let start = definition.start_point();

        let start_seconds = start.start_minute.unwrap_or(0).clamp(0, 1439) * 60;
        let initial_location = match &start.location_id {
            Some(id) => definition
                .location(id)
                .ok_or_else(|| anyhow::anyhow!("설정된 시작 location(start_location_id={id})을 찾을 수 없습니다"))?,
            None => definition.locations.first().ok_or_else(|| anyhow::anyhow!("게임에 location이 하나도 없습니다"))?,
        };
        let zone = definition
            .zone_of_location(&initial_location.id)
            .ok_or_else(|| anyhow::anyhow!("시작 location이 어떤 zone에도 속해있지 않습니다"))?;
        let map = definition.map_by_id(&zone.map_id).ok_or_else(|| anyhow::anyhow!("zone이 가리키는 맵을 찾을 수 없습니다"))?;

        let initial_weather = match &start.weather_id {
            Some(id) => definition
                .weather_types
                .iter()
                .find(|w| &w.id == id)
                .ok_or_else(|| anyhow::anyhow!("설정된 시작 날씨(startWeatherId={id})을 찾을 수 없습니다"))?,
            None => definition.weather_types.first().ok_or_else(|| anyhow::anyhow!("게임에 날씨 타입이 하나도 없습니다"))?,
        };
        let time_period_id = crate::game::engine::resolve_time_period(&period_tuples(&definition), start_seconds)
            .ok_or_else(|| anyhow::anyhow!("게임에 시간대가 하나도 없습니다"))?;

        // game_state_defs의 기본값으로 flags를 채운다 (안 하면 "기본값이 0인 변수"도 키가 없어서
        // FlagGte 같은 조건과 AI에게 보여줄 현재 값이 비어버린다).
        let mut initial_flags = serde_json::Map::new();
        for d in &definition.state_defs {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&d.default_value) {
                initial_flags.insert(d.key.clone(), v);
            }
        }
        let initial_flags = serde_json::Value::Object(initial_flags);

        // 제작자가 심어둔 초기 관계(game_relationship_defs)를 세션의 관계로 복사한다.
        // 키는 "보낸쪽>받는쪽"이고 값은 {축: 수치, ..., "tags": [...]}.
        let mut initial_relationships = serde_json::Map::new();
        for d in &definition.relationship_defs {
            let mut rel = d.values_json.as_object().cloned().unwrap_or_default();
            rel.insert("tags".into(), if d.tags.is_array() { d.tags.clone() } else { serde_json::json!([]) });
            initial_relationships.insert(format!("{}>{}", d.from_id, d.to_id), serde_json::Value::Object(rel));
        }
        let initial_relationships = serde_json::Value::Object(initial_relationships);

        let state = GameState {
            session_id,
            player_id, // 새 게임 시작 시점에 선택돼 있던 페르소나 (start_new_game이 설정에서 읽어 넘김). 세션 동안 고정.
            parent_session_id: None,
            forked_from_seq: None,
            head_seq: 0,
            location_id: Some(initial_location.id.clone()),
            map_id: map.id.clone(),
            zone_id: Some(zone.id.clone()),
            game_time: start_seconds,
            weather_id: initial_weather.id.clone(),
            time_period_id,
            // 시작 씬: game_projects.start_scene_key. TODO: 씬 진입을 이벤트(SceneChanged)로 알리지 않는다
            // — 대기열/진입 경로가 정해지면 GameStarted 뒤에 같은 경로로 열도록 바꿀 것.
            current_scene_id: definition.project.start_scene_key.as_ref().and_then(|k| definition.scenes.iter().find(|s| &s.scene_key == k)).map(|s| s.id.clone()),
            pending_scenes_json: serde_json::json!([]),
            active_npc_id: None,
            flags: initial_flags,
            inventory_json: serde_json::json!({}),
            relationships_json: initial_relationships,
            stage_json: serde_json::json!({}),
            npc_states: Default::default(), // start()가 GameStarted를 흘려서 처음 위치를 굴린다
            cooldowns: Default::default(),
            auto_active: Default::default(),
            turn_fired: false,
        };
        Ok(Self::new(definition, state))
    }

    /// 정의는 open_game이 이미 올려둔 것을 받는다 (여기서 다시 읽지 않는다).
    pub async fn load(repository: &impl GameRepository, definition: std::sync::Arc<GameDefinition>, session_id: &str) -> Result<Self> {
        let session = repository.load_session(session_id).await?;

        let time_period_id = crate::game::engine::resolve_time_period(&period_tuples(&definition), session.game_time)
            .ok_or_else(|| anyhow::anyhow!("게임에 시간대가 하나도 없습니다"))?;

        let cooldowns = repository.load_cooldowns(&definition.game_id, session_id, session.game_time).await?;
        let mut state = GameState::from_session(session, cooldowns, time_period_id).map_err(|e| anyhow::anyhow!(e))?;
        state.npc_states = repository.load_npc_states(session_id).await?;
        // 에디터에서 정의가 바뀌었을 수 있다 (id 보존 저장이라도 행을 지웠으면 FK가 NULL로 만든다).
        // 사라진 위치/씬/날씨를 가리키는 세션은 여기서 안전한 값으로 되돌린다.
        repair_state(&definition, &mut state);
        // 지금 이미 참인 auto 이벤트는 "참인 상태"로 표시만 해둔다 (이어하기 직후 재발동 방지).
        systems::prime_auto_state(&definition, &mut state);

        Ok(Self::new(definition, state))
    }

    /// 새 게임의 첫 턴: GameStarted를 흘려서 NPC의 처음 위치를 굴린다. 돌려주는 이벤트(GameStarted 포함)를
    /// 그대로 journal::record에 넘겨서 기록한다.
    pub fn start(&mut self) -> Vec<GameEvent> {
        self.state.turn_fired = false;
        self.queue.push(GameEvent::GameStarted);
        self.process_events()
    }

    pub fn dispatch(&mut self, command: GameCommand) -> Result<GameResponse> {
        self.state.turn_fired = false; // random 억제 플래그는 dispatch마다 새로 시작
        self.handle_command(command)?;
        let events = self.process_events();
        Ok(GameResponse { events, state: self.render_state() })
    }

    fn emit(&mut self, event: GameEvent) {
        self.queue.push(event);
    }

    /// 커맨드를 "씨앗 이벤트" 목록으로 바꿔서 큐에 넣는다. 여기서 state를 직접
    /// 건드리지 않는 걸 원칙으로 한다 (state는 시스템들이 handle_event에서 고친다) —
    /// 단, from/old처럼 "커맨드 시점의 이전 상태"가 필요한 값은 여기서 읽어서 이벤트에
    /// 실어 보낸다. 조건/존재 검증(이동 가능한지, 선택지가 실존하는지 등)도 여기서 한다 —
    /// definition은 읽기만 하는 조회라 DB 없이 바로 할 수 있다.
    fn handle_command(&mut self, command: GameCommand) -> Result<()> {
        match command {
            GameCommand::MoveTo { location_id } => {
                if self.state.current_scene_id.is_some() {
                    return Err(anyhow::anyhow!("씬이 진행 중일 때는 이동할 수 없습니다"));
                }
                let from = self.state.location_id.clone();
                let Some(from_id) = from.clone() else {
                    return Err(anyhow::anyhow!("현재 위치가 없어 이동할 수 없습니다"));
                };
                let connection = self
                    .definition
                    .connection(&from_id, &location_id)
                    .ok_or_else(|| anyhow::anyhow!("그 경로로는 이동할 수 없습니다"))?;
                // 조건이 적혀 있는데 못 읽으면 통과시키지 않는다 (fail-closed). 빈 문자열은 조건 없음.
                if !self.condition_open(connection.condition.as_deref()) {
                    return Err(anyhow::anyhow!("이동 조건이 충족되지 않았습니다"));
                }
                let seconds = connection.travel_seconds as i64;
                self.emit(GameEvent::LocationChanged { from, to: location_id });
                self.emit(GameEvent::TimeAdvanceRequested { seconds });
            }
            GameCommand::Interact { entity_id } => {
                self.definition
                    .interactable(&entity_id)
                    .ok_or_else(|| anyhow::anyhow!("존재하지 않는 상호작용 대상입니다: {entity_id}"))?;
                self.emit(GameEvent::TriggerActivated { trigger_id: entity_id });
            }
            GameCommand::Talk { game_character_id } => {
                // 씬이 열려 있는 동안은 1:1 대화 대신 씬 입력(sceneSay)을 쓴다 (AI가 서술자로 장면 전체를 진행한다).
                if self.state.current_scene_id.is_some() {
                    return Err(anyhow::anyhow!("씬이 진행 중일 때는 씬 입력(sceneSay)을 사용하세요"));
                }
                self.definition
                    .character(&game_character_id)
                    .ok_or_else(|| anyhow::anyhow!("존재하지 않는 캐릭터입니다: {game_character_id}"))?;
                // 위치를 추적하는 NPC(스폰 규칙이 있는 캐릭터)는 같은 위치에 있을 때만 말을 걸 수 있다.
                // 스폰 규칙이 없는 캐릭터는 항상 가능하다 (스폰을 안 쓰는 기존 게임이 안 깨지게).
                let tracked = self.definition.spawn_locations.get(&game_character_id).map_or(false, |v| !v.is_empty());
                if tracked {
                    let here = self.state.npc_states.get(&game_character_id).and_then(|n| n.location_id.as_deref());
                    if here.is_none() || here != self.state.location_id.as_deref() {
                        return Err(anyhow::anyhow!("그 캐릭터는 지금 여기에 없습니다"));
                    }
                }
                self.emit(GameEvent::DialogueStarted { game_character_id });
            }
            GameCommand::StopTalk => {
                let game_character_id = self.state.active_npc_id.clone().ok_or_else(|| anyhow::anyhow!("열린 대화가 없습니다"))?;
                self.emit(GameEvent::DialogueEnded { game_character_id });
            }
            GameCommand::SetVariable { key, value } => {
                let old = self.state.flags.get(&key).cloned().unwrap_or(serde_json::Value::Null);
                self.emit(GameEvent::VariableChanged { key, old, new: value });
            }
            GameCommand::Choose { choice_id } => {
                let (scene, choice) = self
                    .definition
                    .find_choice(&choice_id)
                    .ok_or_else(|| anyhow::anyhow!("존재하지 않는 선택지입니다: {choice_id}"))?;
                // 지금 열려 있는 씬의 선택지만 고를 수 있다 (아니면 아무 씬의 선택지 id로 씬을 건너뛸 수 있다).
                if self.state.current_scene_id.as_deref() != Some(scene.id.as_str()) {
                    return Err(anyhow::anyhow!("지금 열려 있는 씬의 선택지가 아닙니다"));
                }
                if !self.choice_available(choice) {
                    return Err(anyhow::anyhow!("지금은 고를 수 없는 선택지입니다"));
                }
                let effect: Option<serde_json::Value> = choice.effect.as_ref().and_then(|s| serde_json::from_str(s).ok());
                let effect_ends = effect.as_ref().and_then(|e| e.get("end_scene")).and_then(|v| v.as_bool()) == Some(true);
                let next_scene_id = choice.next_scene_id.clone();
                let ai_continue = definition::scene_flag(scene, "aiContinue", true);
                // 다음 씬이 없고 끝내는 효과도 없으면 씬이 이어진다. (AI 이어가기를 끈 씬은 옛 방식대로 바로 끝난다.)
                let continues_scene = next_scene_id.is_none() && !effect_ends && ai_continue;
                let exit_now = next_scene_id.is_none() && !effect_ends && !ai_continue;
                self.emit(GameEvent::ChoiceMade { choice_id, next_scene_id, effect, continues_scene });
                if exit_now {
                    self.emit(GameEvent::SceneEndRequested);
                }
            }
            GameCommand::EndScene => {
                let ending = self
                    .state
                    .current_scene_id
                    .as_deref()
                    .and_then(|id| self.definition.scene(id))
                    .map(|s| s.is_ending == Some(true))
                    .ok_or_else(|| anyhow::anyhow!("열려 있는 씬이 없습니다"))?;
                if ending {
                    return Err(anyhow::anyhow!("엔딩 씬은 끝낼 수 없습니다"));
                }
                self.emit(GameEvent::SceneEndRequested);
            }
            GameCommand::AdvanceTime { minutes } => {
                if minutes < 0 {
                    return Err(anyhow::anyhow!("minutes는 음수일 수 없습니다"));
                }
                self.emit(GameEvent::TimeAdvanceRequested { seconds: minutes as i64 * 60 });
            }
            GameCommand::UseItem { item_id, target_id } => {
                let item = self.definition.items.get(&item_id).ok_or_else(|| anyhow::anyhow!("존재하지 않는 아이템입니다: {item_id}"))?;
                if self.inventory_count(&item_id) < 1 {
                    return Err(anyhow::anyhow!("가지고 있지 않은 아이템입니다"));
                }
                if json_is_empty(&item.use_effect) {
                    return Err(anyhow::anyhow!("사용할 수 없는 아이템입니다"));
                }
                if !json_is_empty(&item.use_condition) {
                    // 조건이 적혀 있는데 못 읽으면 막는다 (fail-closed).
                    let cond = serde_json::from_value::<crate::game::engine::Condition>(item.use_condition.clone()).unwrap_or(crate::game::engine::Condition::Never);
                    if !cond.eval(&self.engine_state_snapshot()) {
                        return Err(anyhow::anyhow!("지금은 사용할 수 없습니다"));
                    }
                }
                if let Some(t) = &target_id {
                    self.definition.character(t).ok_or_else(|| anyhow::anyhow!("존재하지 않는 대상입니다: {t}"))?;
                }
                self.emit(GameEvent::ItemUsed { item_id, target_id });
            }
            GameCommand::DropItem { item_id, quantity } => {
                let item = self.definition.items.get(&item_id).ok_or_else(|| anyhow::anyhow!("존재하지 않는 아이템입니다: {item_id}"))?;
                if item.is_key {
                    return Err(anyhow::anyhow!("핵심 아이템은 버릴 수 없습니다"));
                }
                let owned = self.inventory_count(&item_id);
                let qty = quantity.unwrap_or(1) as i64;
                if qty < 1 || owned < 1 {
                    return Err(anyhow::anyhow!("버릴 수 있는 아이템이 없습니다"));
                }
                self.emit(GameEvent::ItemChangeRequested { item_id, delta: -qty.min(owned) });
            }
        }
        Ok(())
    }

    fn inventory_count(&self, item_id: &str) -> i64 {
        self.state.inventory_json.get(item_id).and_then(|v| v.as_i64()).unwrap_or(0)
    }

    fn engine_state_snapshot(&self) -> crate::game::engine::EngineState {
        crate::game::engine::EngineState {
            map_id: self.state.map_id.clone(),
            zone_id: self.state.zone_id.clone(),
            location_id: self.state.location_id.clone(),
            game_time: self.state.game_time,
            time_from: self.state.game_time,
            weather_id: self.state.weather_id.clone(),
            time_period_id: self.state.time_period_id.clone(),
            flags: self.state.flags.clone(),
            inventory: self.state.inventory_json.clone(),
            relationships: self.state.relationships_json.clone(),
        }
    }

    /// 깊이 제한 있는 전파: 씨앗 이벤트를 모든 시스템에 통과시키고, 거기서 나온
    /// 이벤트를 다음 라운드의 입력으로 다시 돌린다. MAX_EVENT_DEPTH 라운드가 지나면
    /// 끊는다 (순환 데이터로 인한 무한 루프 방지 — 완전 재귀도, 1단계 고정도 아니다).
    fn process_events(&mut self) -> Vec<GameEvent> {
        let mut frontier: Vec<GameEvent> = std::mem::take(&mut self.queue);
        let mut all_events = Vec::new();
        let mut depth = 0usize;

        while !frontier.is_empty() {
            all_events.extend(frontier.iter().cloned());
            depth += 1;
            if depth > MAX_EVENT_DEPTH {
                // TODO: 여기 도달하면 로깅 — 이벤트가 몇 단계 넘게 서로를 계속 트리거하고
                //   있다는 뜻이라, 데이터(트리거 조건 등) 설계를 점검해야 한다는 신호다.
                break;
            }
            let mut next = Vec::new();
            for event in &frontier {
                for system in systems::all() {
                    next.extend(system.handle_event(&self.definition, &mut self.state, event, &mut self.rng));
                }
            }
            frontier = next;
        }
        all_events
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn definition(&self) -> &GameDefinition {
        &self.definition
    }

    /// 이동 연결의 조건 문자열을 평가한다. 없거나 빈 문자열이면 통과, 적혀 있는데 못 읽으면 막는다 (fail-closed).
    /// MoveTo의 검사와 RenderState.available_moves가 같은 규칙을 쓰도록 여기 하나로 둔다.
    fn condition_open(&self, raw: Option<&str>) -> bool {
        match raw.filter(|s| !s.trim().is_empty()) {
            None => true,
            Some(raw) => serde_json::from_str::<crate::game::engine::Condition>(raw)
                .unwrap_or(crate::game::engine::Condition::Never)
                .eval(&self.engine_state_snapshot()),
        }
    }

    /// 선택지 표시 조건(choice.condition) 평가. 조건이 적혀 있는데 못 읽으면 막는다 (fail-closed).
    fn choice_available(&self, choice: &crate::models::game::GameChoiceRow) -> bool {
        match choice.condition.as_ref().filter(|s| !s.trim().is_empty()) {
            None => true,
            Some(raw) => serde_json::from_str::<crate::game::engine::Condition>(raw)
                .unwrap_or(crate::game::engine::Condition::Never)
                .eval(&self.engine_state_snapshot()),
        }
    }

    pub fn render_state(&self) -> RenderState {
        let mut render = RenderState::from(&self.state);
        render.scene_chat = self.scene_chat_enabled();
        if let Some(here) = self.state.location_id.as_deref() {
            let mut present: Vec<String> = self.state.npc_states.iter().filter(|(_, n)| n.location_id.as_deref() == Some(here)).map(|(id, _)| id.clone()).collect();
            present.sort();
            render.present_npcs = present;
            // 갈 수 있는 곳: 지금 위치에서 나가는 연결들. 씬이 열려 있는 동안은 전부 막힌다 (MoveTo가 거절하므로).
            let scene_open = self.state.current_scene_id.is_some();
            render.available_moves = self
                .definition
                .location_connections
                .iter()
                .filter(|c| c.from_id == here)
                .map(|c| AvailableMove { to_id: c.to_id.clone(), travel_seconds: c.travel_seconds, available: !scene_open && self.condition_open(c.condition.as_deref()) })
                .collect();
        }
        if let Some(scene) = self.state.current_scene_id.as_deref().and_then(|id| self.definition.scene(id)) {
            render.available_choices = scene.choices.iter().flatten().filter(|c| self.choice_available(c)).map(|c| c.id.clone()).collect();
        }
        render
    }

    /// 지금 플레이어와 같은 위치에 있는 NPC들 (정렬). AI 응답에서 말할 수 있는 인물의 범위다.
    pub fn present_npc_ids(&self) -> Vec<String> {
        let Some(here) = self.state.location_id.as_deref() else { return vec![] };
        let mut v: Vec<String> = self.state.npc_states.iter().filter(|(_, n)| n.location_id.as_deref() == Some(here)).map(|(id, _)| id.clone()).collect();
        v.sort();
        v
    }

    /// 이름(또는 id)으로 캐릭터를 찾는다 (AI 태그의 `::이름` 대상 해석용).
    pub fn find_character_id(&self, name: &str) -> Option<String> {
        let n = name.trim().to_lowercase();
        self.definition.characters.values().find(|c| c.name.trim().to_lowercase() == n || c.id.to_lowercase() == n).map(|c| c.id.clone())
    }

    /// 지금 열린 씬이 AI 입력을 받을 수 있는가 (씬이 열려 있고, 엔딩이 아니며, aiContinue가 꺼져 있지 않다).
    pub fn scene_chat_enabled(&self) -> bool {
        self.state.current_scene_id.as_deref().and_then(|id| self.definition.scene(id)).map_or(false, |s| s.is_ending != Some(true) && definition::scene_flag(s, "aiContinue", true))
    }

    /// 지금 씬의 minTurns (AI가 씬을 끝낼 수 있게 되는 플레이어 입력 횟수).
    pub fn scene_min_turns(&self) -> u32 {
        self.state.current_scene_id.as_deref().and_then(|id| self.definition.scene(id)).map_or(1, definition::scene_min_turns)
    }

    /// AI가 <scene>end</scene>로 씬을 끝내려 할 때. allowed=false(아직 이르다)거나 씬이 없으면 아무 일도 없다.
    pub fn end_scene_by_ai(&mut self, allowed: bool) -> Vec<GameEvent> {
        if !allowed || !self.scene_chat_enabled() {
            return vec![];
        }
        self.state.turn_fired = false;
        self.queue.push(GameEvent::SceneEndRequested);
        self.process_events()
    }

    /// AI 응답 텍스트를 줄/연출로 파싱한다 (제작자 대본과 같은 문법, 허용 범위만 AI용).
    /// speakers: 말하거나 연출 대상이 될 수 있는 캐릭터 (대화 상대 + 같은 위치의 NPC).
    /// `이름:` 접두가 하나도 없으면 옛 방식의 응답으로 보고 전부 대화 상대의 대사로 취급한다.
    /// 주의: 연출 태그가 풀리지 않거나 화자가 허용되지 않아 줄이 전부 버려지면 빈 목록이 나온다.
    pub fn parse_ai_stage(&mut self, raw: &str, speakers: std::collections::HashSet<String>, default_speaker: Option<&str>) -> Vec<crate::game::stage::StageLine> {
        use crate::game::stage::{self, Allowed, LineKind};
        let allow = Allowed::ai(speakers);
        let mut resolver = DefResolver { def: &self.definition, rng: &mut self.rng, ai: true, player_name: None };
        let mut lines = stage::parse(raw, &mut resolver, &allow);
        // 씬 입력(서술자)에서는 default_speaker가 없다 — 지문은 지문으로 둔다.
        if let Some(default_speaker) = default_speaker {
            if !lines.iter().any(|l| l.kind == LineKind::Dialogue) {
                for l in lines.iter_mut().filter(|l| l.kind == LineKind::Narration) {
                    l.kind = LineKind::Dialogue;
                    l.speaker = Some(default_speaker.to_string());
                }
            }
        }
        lines
    }

    /// 파싱된 줄의 연출을 무대 상태(stage_json)에 반영한다. 이어하기/분기 때 같은 화면이 복원된다.
    pub fn commit_stage(&mut self, lines: &[crate::game::stage::StageLine]) {
        for l in lines {
            for c in &l.cues {
                crate::game::stage::apply_cue_to_stage(&mut self.state.stage_json, c);
            }
        }
    }

    /// AI가 낸 effect들을 허용 목록/범위로 걸러서 (key, old, new) 변경 목록으로 만든다 (상태는 안 바꿈).
    /// 여러 effect는 앞의 결과를 누적해서 다음에 적용한다 — 같은 키를 두 번 건드려도 max_step이 지켜진다.
    pub fn plan_ai_changes(&self, effects: &[serde_json::Value], max_changes: usize) -> Vec<(String, serde_json::Value, serde_json::Value)> {
        let rules = self.definition.ai_var_rules();
        let mut working = self.state.flags.clone();
        let mut all: Vec<(String, serde_json::Value, serde_json::Value)> = Vec::new();
        for effect in effects {
            let remaining = max_changes.saturating_sub(all.len());
            if remaining == 0 {
                break;
            }
            let changes = crate::game::engine::plan_ai_changes(&working, effect, &rules, remaining);
            if !working.is_object() {
                working = serde_json::json!({});
            }
            if let Some(obj) = working.as_object_mut() {
                for (k, _, new) in &changes {
                    obj.insert(k.clone(), new.clone());
                }
            }
            all.extend(changes);
        }
        all
    }

    /// AI가 낸 관계 effect들을 걸러서 (from, to, axis, old, new) 목록으로 만든다 (상태는 안 바꿈).
    /// 허용 범위: "이 캐릭터가 플레이어에게 느끼는 것"(npc>player)만. 임의의 두 NPC 사이는 AI가 못 바꾼다.
    /// 여러 effect는 앞의 결과를 누적해서 다음에 적용한다 (같은 축을 두 번 건드려도 폭이 지켜진다).
    pub fn plan_ai_relation(&self, npc_id: &str, effects: &[serde_json::Value], max_changes: usize) -> Vec<(String, String, String, i64, i64)> {
        use crate::game::engine::{plan_ai_relation, RELATION_AI_MAX_STEP};
        let mut working = self.state.relationships_json.clone();
        let mut all = Vec::new();
        for effect in effects {
            let remaining = max_changes.saturating_sub(all.len());
            if remaining == 0 {
                break;
            }
            let changes = plan_ai_relation(&working, npc_id, "player", effect, RELATION_AI_MAX_STEP, remaining);
            if !working.is_object() {
                working = serde_json::json!({});
            }
            if let Some(obj) = working.as_object_mut() {
                for (axis, _, new) in &changes {
                    obj.entry(format!("{npc_id}>player")).or_insert_with(|| serde_json::json!({ "tags": [] }))[axis.as_str()] = serde_json::Value::from(*new);
                }
            }
            all.extend(changes.into_iter().map(|(axis, old, new)| (npc_id.to_string(), "player".to_string(), axis, old, new)));
        }
        all
    }

    /// plan_ai_changes로 검증을 통과한 변경만 적용한다. 프론트가 부를 수 있는 GameCommand가 아니라
    /// 런타임 안쪽 메서드다 (아무 변수나 쓸 수 있게 되면 안 되니까). 변경은 VariableChanged로
    /// 환원돼서 평소 이벤트 전파(auto 이벤트, 씬 등)를 그대로 탄다.
    pub fn apply_ai_changes(&mut self, changes: Vec<(String, serde_json::Value, serde_json::Value)>, relations: Vec<(String, String, String, i64, i64)>) -> Vec<GameEvent> {
        self.state.turn_fired = false;
        self.queue.extend(changes.into_iter().map(|(key, old, new)| GameEvent::VariableChanged { key, old, new }));
        self.queue.extend(relations.into_iter().map(|(from, to, axis, old, new)| GameEvent::RelationChangeRequested { from, to, axis, delta: new - old }));
        self.process_events()
    }

    /// 세션 전체에서 단조 증가하는 번호를 발급한다. 일지/메시지/이벤트 로그가 전부 이 번호를 쓴다.
    /// 이 앱은 세션 하나를 런타임 하나(락 하나)가 쓰므로 DB 트랜잭션 없이 메모리에서 발급해도 안전하다.
    /// 발급 후에는 save()로 head_seq를 저장해야 다시 불러왔을 때 번호가 이어진다.
    pub fn next_seq(&mut self) -> u32 {
        self.state.head_seq += 1;
        self.state.head_seq
    }

    /// 매 턴 자동으로 부르는 가벼운 저장 (세션 위치/시간/날씨만). 세이브 슬롯 스냅샷
    /// (game_saves.state_snapshot)은 별개 — 그건 명시적 "저장하기" 액션에서만.
    pub async fn save(&self, repository: &impl GameRepository, game_id: &str) -> Result<()> {
        repository.persist_session(self.state.to_session_input(game_id.to_string())).await
    }
}
