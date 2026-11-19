-- =========================================================
-- 에이전트
-- =========================================================
-- 에이전트 설정
CREATE TABLE IF NOT EXISTS agent_definitions (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,               -- 에이전트 이름 (UI 표시용, 예: "Rust 수석 개발자")
  description TEXT,                 -- 에이전트 설명
  system_prompt TEXT NOT NULL,      -- 에이전트의 핵심 행동 지침 및 페르소나
  model_profile_id TEXT NOT NULL,   -- 사용할 LLM 모델 설정 (API Provider, 모델명 매핑)
  temperature REAL DEFAULT 0.7,     -- 창의성 조절 파라미터
  tools TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(tools)),                       -- JSON Array 형식 (예: '["read_file", "execute_sql"]')
  skills TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(skills)),                      -- JSON Array 형식 (예: '["rust_expert", "refactoring"]')
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

-- 에이전트별 영속 상태
CREATE TABLE IF NOT EXISTS agent_states (
  agent_id TEXT NOT NULL REFERENCES agent_definitions(id) ON DELETE CASCADE,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  scope TEXT NOT NULL,              -- 메모리 공유 범위 (user: 사용자 전체 공유 / conversation: 대화방에서만 공유 / game_session: 게임세션에서만 공유)
  state_key TEXT NOT NULL,          -- 상태 키 (예: 'current_working_dir', 'memory_scratchpad')
  state_value TEXT,                 -- 상태 값 (JSON 또는 문자열)
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (agent_id, conversation_id, state_key)
);
-- 에이전트 도구 호출 및 실행 결과
CREATE TABLE IF NOT EXISTS agent_tool_calls (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES generation_tasks(id) ON DELETE CASCADE,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  attempt_id TEXT REFERENCES ai_attempts(id),
  agent_id TEXT REFERENCES agent_definitions(id) ON DELETE SET NULL,
  tool_name TEXT NOT NULL,          -- 호출된 도구 이름 (예: 'fs_read', 'run_query')
  arguments_json TEXT NOT NULL,     -- 에이전트가 도구에 전달한 인자 (JSON)
  result_json TEXT,                 -- 도구 실행 결과 (성공 시 데이터 또는 에러 메시지)
  status TEXT NOT NULL,             -- 'pending', 'success', 'failed'
  idempotency_key TEXT,             -- 먹등성 고려
  started_at INTEGER NOT NULL,
  finished_at INTEGER
);
