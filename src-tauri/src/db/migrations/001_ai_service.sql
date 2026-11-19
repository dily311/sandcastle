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
  -- 한 키로 여러 모델을 쓰려고, 직접 입력한 모델 이름을 프로필에 남겨 목록에 계속 보이게 한다.
  -- JSON 문자열 배열(예: ["gpt-6-luna"]). 프로필 행에 들어 있어 프로필과 함께 동기화된다.
  -- 앱에 내장된 모델 목록은 앱 코드에 있고 여기에 넣지 않는다.
  custom_models TEXT NOT NULL DEFAULT '[]',
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


-- =========================================================
-- 기본 프리셋 (앱 설정의 selectedPreset 기본값 "default")
-- =========================================================
INSERT OR IGNORE INTO prompt_presets (id, name, description, preset_type, source_format, config_json, settings_json, created_at, updated_at) VALUES
('default', 'Default', 'Balanced roleplay preset. Edit freely or duplicate it.', 'chat_completion', 'native', '{}',
 '{"temperature":1,"max_tokens":1024,"context_size":16384,"top_p":0.95,"stream":true}',
 strftime('%s','now') * 1000, strftime('%s','now') * 1000);

INSERT OR IGNORE INTO prompt_blocks (id, preset_id, identifier, block_type, name, role, content, enabled, position, depth, sort_order, triggers_json, created_at, updated_at) VALUES
('default-main', 'default', 'main', 'text', 'Main Prompt', 'system',
 'You are {{char}} in an ongoing, never-ending roleplay with {{user}}. Write {{char}}''s next reply only.

- Stay true to {{char}}''s personality, voice, knowledge and motives as described below. Let them grow naturally from what happens.
- Never speak, act, think or decide for {{user}}. End your reply at a point where {{user}} can respond.
- Show, don''t tell: use concrete actions, sensory detail and dialogue. Put actions in *asterisks* and speech in "quotes".
- Keep the story moving. Introduce small events, choices or complications instead of repeating or summarizing.
- Match the length and tone of the scene. Avoid repeating phrases from earlier replies.
- Reply in the same language {{user}} writes in.',
 1, 'relative', NULL, 0, '[]', strftime('%s','now') * 1000, strftime('%s','now') * 1000),
('default-wi', 'default', 'worldInfoBefore', 'text', 'World Info', 'system', '', 1, 'relative', NULL, 1, '[]', strftime('%s','now') * 1000, strftime('%s','now') * 1000),
('default-char', 'default', 'charDescription', 'text', 'Character Description', 'system', '', 1, 'relative', NULL, 2, '[]', strftime('%s','now') * 1000, strftime('%s','now') * 1000),
('default-persona', 'default', 'personaDescription', 'text', 'User Persona', 'system', '', 1, 'relative', NULL, 3, '[]', strftime('%s','now') * 1000, strftime('%s','now') * 1000),
('default-examples', 'default', 'dialogueExamples', 'text', 'Example Dialogue', 'system', '', 1, 'relative', NULL, 4, '[]', strftime('%s','now') * 1000, strftime('%s','now') * 1000),
('default-history', 'default', 'chatHistory', 'text', 'Chat History', 'system', '', 1, 'relative', NULL, 5, '[]', strftime('%s','now') * 1000, strftime('%s','now') * 1000),
('default-post', 'default', 'postHistory', 'text', 'Reminder', 'system',
 '[Continue as {{char}}. Do not write {{user}}''s words or actions. Keep the reply focused on what happens next.]',
 1, 'relative', NULL, 6, '[]', strftime('%s','now') * 1000, strftime('%s','now') * 1000);
