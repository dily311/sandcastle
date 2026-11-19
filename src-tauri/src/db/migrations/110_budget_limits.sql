-- =========================================================
-- 예산 한도 (CLAUDE.md 1-A-1, ADR-007)
-- ---------------------------------------------------------
-- API 프로필별 하루/한 달 상한. NULL = 한도 없음 (행이 없어도 한도 없음)
-- 하루·한 달은 기기의 현지 시각 기준 (오늘 0시 / 이번 달 1일 0시부터)
-- 토큰 = 입력 + 출력, 글자 = 음성으로 만든 글자 수, 그림 = 만든 장 수
-- =========================================================

CREATE TABLE IF NOT EXISTS budget_limits (
  api_profile_id TEXT PRIMARY KEY REFERENCES api_profiles(id) ON DELETE CASCADE,
  daily_tokens INTEGER,
  monthly_tokens INTEGER,
  daily_characters INTEGER,
  monthly_characters INTEGER,
  daily_images INTEGER,
  monthly_images INTEGER,
  monthly_usd REAL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_usage_log_profile_created ON usage_log(api_profile_id, created_at);
