-- =========================================================
-- 채팅 부가 설정 (메모리 방식, 선톡, API 프로필)
-- =========================================================

CREATE TABLE IF NOT EXISTS memory_settings (
  id TEXT PRIMARY KEY,
  character_id TEXT NOT NULL UNIQUE REFERENCES characters(id) ON DELETE CASCADE,
  mode TEXT NOT NULL DEFAULT 'truncate' CHECK (mode IN ('full', 'truncate', 'summarize')),
  max_turns INTEGER DEFAULT 30,
  summary_interval INTEGER DEFAULT 20,
  updated_at INTEGER
);
CREATE TABLE IF NOT EXISTS memory_embeddings (
  id TEXT PRIMARY KEY,
  embedding BLOB NOT NULL,
  dimension INTEGER NOT NULL,
  created_at_at INTEGER NOT NULL,
  updated_at INTEGER
);
CREATE TABLE IF NOT EXISTS proactive_settings (
  id TEXT PRIMARY KEY,
  character_id TEXT NOT NULL UNIQUE REFERENCES characters(id) ON DELETE CASCADE,
  enabled INTEGER DEFAULT 1,
  avg_interval_hours REAL,          -- 최근 대화 간격 이동평균 (앱이 주기적으로 갱신)
  dormant_after_days INTEGER DEFAULT 10,
  last_computed_at INTEGER,
  updated_at INTEGER
);