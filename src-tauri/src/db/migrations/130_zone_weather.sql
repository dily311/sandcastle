-- zone별 날씨: {zone_id: {"weather": 날씨 id, "day": 일차, "period": 시간대 id}}
-- 같은 zone·같은 날·같은 시간대 안에서는 날씨가 그대로다 (game/runtime/systems/time.rs)
ALTER TABLE game_player_sessions ADD COLUMN zone_weather_json TEXT;
