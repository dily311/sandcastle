-- =======================================
-- APP
-- =======================================
CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at INTEGER
);

-- =======================================
-- API 세팅
-- =======================================
CREATE TABLE IF NOT EXISTS api_profile (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  provider TEXT NOT NULL,
  format_type TEXT NOT NULL,
  base_url TEXT NOT NULL,
  model TEXT NOT NULL,
  selected_preset_id TEXT,
  -- api_key 컬럼 없음! 키링에만 존재
  created_at INTEGER,
  updated_at INTEGER
);
