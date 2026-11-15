//! TimeAdvanceRequested에 반응해 시간을 흘리고, 시간대/날씨를 갱신한다.
//! "왜 시간이 흐르는지"(이동/기다림)는 모르고, 요청된 seconds만 본다.
//!
//! 날씨는 zone 단위다: 같은 zone·같은 날·같은 시간대 안에서는 그대로이고(같은 zone 안에서 이동해도 안 바뀐다),
//! 시간대나 날이 바뀌었거나 그 시점에 처음 들어간 zone이면 맵의 전이 확률로 다시 굴린다.
//! 이동은 같은 라운드에서 MovementSystem이 먼저 zone을 바꿔 두므로, 여기서는 도착한 zone 기준으로 본다.
//! 결과로 TimeAdvanced(+WeatherChanged)를 내면, 다음 라운드에서 EncounterSystem이 그걸 받아 굴린다.
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::GameSystem;
use crate::game::engine;
use crate::game::runtime::state::day_of;
use rand::rngs::StdRng;

pub struct TimeSystem;

impl GameSystem for TimeSystem {
    fn name(&self) -> &'static str {
        "time"
    }

    fn handle_event(&self, def: &GameDefinition, state: &mut GameState, event: &GameEvent, rng: &mut StdRng) -> Vec<GameEvent> {
        let GameEvent::TimeAdvanceRequested { seconds } = event else {
            return vec![];
        };

        let from = state.game_time;
        state.game_time += seconds;
        let mut out = vec![GameEvent::TimeAdvanced { from, to: state.game_time }];

        let periods: Vec<(String, u32, u32)> = def.time_periods.iter().map(|p| (p.id.clone(), p.start_minute, p.end_minute)).collect();
        if let Some(period_id) = engine::resolve_time_period(&periods, state.game_time) {
            if period_id != state.time_period_id {
                out.push(GameEvent::TimePeriodChanged { from: Some(state.time_period_id.clone()), to: period_id.clone() });
                state.time_period_id = period_id;
            }
        }

        let zone_key = state.zone_id.clone().unwrap_or_default();
        let day = day_of(state.game_time);
        let remembered = state.zone_weather.get(&zone_key).cloned();
        let remembered_weather = remembered.as_ref().and_then(|v| v.get("weather")).and_then(|v| v.as_str()).map(str::to_string);
        let same_moment = remembered.as_ref().is_some_and(|v| {
            v.get("day").and_then(|d| d.as_i64()) == Some(day) && v.get("period").and_then(|p| p.as_str()) == Some(state.time_period_id.as_str())
        });
        let new_weather = match remembered_weather {
            // 같은 zone·같은 날·같은 시간대: 기억한 날씨 그대로
            Some(w) if same_moment && def.weather_types.iter().any(|t| t.id == w) => w,
            // 시점이 바뀌었거나 처음 온 zone: 그 zone의 마지막 날씨(없으면 지금 날씨)에서 굴린다
            from => {
                let from = from.filter(|w| def.weather_types.iter().any(|t| &t.id == w)).unwrap_or_else(|| state.weather_id.clone());
                let candidates: Vec<(String, f64)> = def
                    .weather_transitions_for(&state.map_id, &from)
                    .into_iter()
                    .map(|t| (t.to_weather_id.clone(), t.probability))
                    .collect();
                engine::roll_next_weather(&candidates, &from, rng)
            }
        };
        let changed = new_weather != state.weather_id;
        state.weather_id = new_weather.clone();
        state.remember_zone_weather();
        if changed {
            state.weather_id = new_weather.clone();
            out.push(GameEvent::WeatherChanged { weather_id: new_weather });
        }
        out
    }
}
