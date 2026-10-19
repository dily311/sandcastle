//! TimeAdvanceRequested에 반응해 시간을 흘리고, 시간대/날씨를 갱신한다.
//! "왜 시간이 흐르는지"(이동/기다림)는 모르고, 요청된 seconds만 본다.
//! 결과로 TimeAdvanced(+WeatherChanged)를 내면, 다음 라운드에서 EncounterSystem이 그걸 받아 굴린다.
use super::super::definition::GameDefinition;
use super::super::event::GameEvent;
use super::super::state::GameState;
use super::GameSystem;
use crate::game::engine;
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
            state.time_period_id = period_id;
        }

        let candidates: Vec<(String, f64)> = def
            .weather_transitions_for(&state.map_id, &state.weather_id)
            .into_iter()
            .map(|t| (t.to_weather_id.clone(), t.probability))
            .collect();
        let new_weather = engine::roll_next_weather(&candidates, &state.weather_id, rng);
        if new_weather != state.weather_id {
            state.weather_id = new_weather.clone();
            out.push(GameEvent::WeatherChanged { weather_id: new_weather });
        }
        out
    }
}
