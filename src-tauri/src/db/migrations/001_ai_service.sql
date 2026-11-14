-- =========================================================
-- 채팅 부가 설정 (메모리 방식, 선톡, API 프로필)
-- =========================================================

-- ---------------------------------------------------------
-- API 세팅
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS api_profiles (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  provider TEXT NOT NULL,
  format_type TEXT NOT NULL,
  base_url TEXT NOT NULL,
  model TEXT NOT NULL,
  selected_preset_id TEXT,
  -- api_key 컬럼 없음! 키링에만 존재
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

-- ---------------------------------------------------------
-- 프롬프트 프리셋
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS prompt_presets (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  -- chat_completion / text_completion
  preset_type TEXT NOT NULL DEFAULT 'chat_completion' CHECK (preset_type IN ('chat_completion', 'text_completion')),
  -- native / sillytavern
  source_format TEXT NOT NULL DEFAULT 'native',
  config_json TEXT NOT NULL,
  -- 추가 설정 및 호환성 데이터
  settings_json TEXT,
  -- 가져온 원본 데이터 보존용
  compatibility_json TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS prompt_blocks (
  id TEXT PRIMARY KEY,
  preset_id TEXT NOT NULL REFERENCES prompt_presets(id) ON DELETE CASCADE,
  -- main / charDescription / scenario / custom 등
  identifier TEXT NOT NULL,
  -- text	일반 프롬프트 / memory_extraction	메모리 추출 / format_instruction	JSON/XML 출력 지침 / system_instruction 시스템 지침
  block_type TEXT NOT NULL DEFAULT 'text',
  name TEXT NOT NULL,
  -- system / assistant / user
  role TEXT NOT NULL DEFAULT 'system' CHECK (role IN ('system', 'assistant', 'user')),
  content TEXT NOT NULL DEFAULT '',
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  -- relative / in_chat
  position TEXT NOT NULL DEFAULT 'relative' CHECK (position IN ('relative', 'in_chat')),
  -- in_chat 삽입 깊이
  depth INTEGER CHECK (depth IS NULL OR depth >= 0),
  -- 프롬프트 실행 순서
  sort_order INTEGER NOT NULL DEFAULT 0,
  -- 생성 유형별 실행 조건
  triggers_json TEXT NOT NULL DEFAULT '[]',
  -- 미지원 필드 보존
  extra_json TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);


-- ---------------------------------------------------------
-- 메모리 방식
-- ---------------------------------------------------------
-- 캐릭터 단위로 할지, 대화 단위로 할지 고민돼
-- ---------------------------------------------------------

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

-- =========================================================
-- 인덱스
-- =========================================================

CREATE INDEX IF NOT EXISTS idx_prompt_blocks_preset ON prompt_blocks(preset_id, sort_order);