-- =========================================================
-- 사용량 일별 합계 (CLAUDE.md 1-A-4, ADR-007)
-- ---------------------------------------------------------
-- 보존 기간을 고르면 그보다 오래된 사용량 상세(usage_log)를 여기 하루 단위로 합쳐 두고 상세는 지운다.
-- 사용량 합계·예산 한도는 usage_log + usage_daily를 같이 읽으므로 정리 뒤에도 숫자가 같다.
-- 같은 날·같은 묶음의 행이 여러 개여도 된다(전부 더해서 읽음)
-- =========================================================

CREATE TABLE IF NOT EXISTS usage_daily (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  -- 기기 현지 날짜 YYYY-MM-DD
  day TEXT NOT NULL,
  -- 그날 0시 (현지, ms) — 기간 비교용
  day_start INTEGER NOT NULL,
  api_profile_id TEXT,
  model TEXT,
  kind TEXT NOT NULL,
  feature TEXT,
  calls INTEGER NOT NULL DEFAULT 0,
  failed INTEGER NOT NULL DEFAULT 0,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  characters INTEGER NOT NULL DEFAULT 0,
  images INTEGER NOT NULL DEFAULT 0,
  estimated_calls INTEGER NOT NULL DEFAULT 0,
  cost_usd REAL,
  unpriced_calls INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_usage_daily_profile_day ON usage_daily(api_profile_id, day_start);
