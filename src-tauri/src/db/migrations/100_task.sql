-- =========================================================
-- 작업관리 테이블
-- ---------------------------------------------------------
-- 설명
-- - 외부 작업 태스크들은 모두 여기 기록
-- - 외부 api를 사용이나 실패할 수 있는 작업관리가 주 목적
-- ---------------------------------------------------------
-- 테이블 추가 원칙
-- 1. 작업과 생명 주기 명시관리
-- 2. 실행결과를 이벤트로 남겨서 언제든 상태 복구(재시도) 가능하게
-- 3. Agent의 상태와 능력을 분리해 동적 확장 가능하게
-- 4. agent와 session 분리
-- =========================================================
-- 전역 관리 테이블
CREATE TABLE IF NOT EXISTS generation_tasks (
  id TEXT PRIMARY KEY,
  -- 'chat_stream', 'image_gen', 'background_sync' 등
  task_type TEXT NOT NULL,
  -- pending: 실행대기 / running: 응답 생성 중 / completed: 생성완료 / failed: 오류 발생 / cancelled: 사용자 취소 / interrupted: 비정상 종료로 중단
  status TEXT NOT NULL,
  -- 작업에 필요한 인자 (JSON 문자열)
  payload_json TEXT,
  -- 실패 시 오류 기록
  error_message TEXT,
  -- chat / game / image / system
  owner TEXT NOT NULL DEFAULT 'system',
  -- conversation_id, game session_id 등 owner 쪽 식별자
  owner_ref TEXT,
  -- 성공 시 TaskOutput (JSON)
  result_json TEXT,
  -- 실패 시 ErrorCode 문자열 (error_message와 함께 사용)
  error_code TEXT,
  attempt_count INTEGER NOT NULL DEFAULT 0,
  -- 한 번의 실행에서 허용하는 최대 시도 횟수 (재시도 가능한 오류일 때만 다시 시도)
  max_attempts INTEGER NOT NULL DEFAULT 1,

  -- generation_tasks: 어느 기능이(feature), 누가 시작했는지(run_by), 화면에 보일 이름(label)
  -- 코어 플러그인·기능 id: chat / phone / game / translate / tts / imagegen / settings
  feature TEXT,
  -- user: 사용자가 누른 실행 / auto: 사용자가 켜 둔 자동 실행(자동 번역·자동 읽기 등)
  run_by TEXT NOT NULL DEFAULT 'user',
  -- 화면 표시용 한 줄 ("번역", "NPC 대화 · 엘리안")
  label TEXT,

  created_at INTEGER NOT NULL,
  started_at INTEGER,
  updated_at INTEGER NOT NULL,
  finished_at INTEGER
);

-- 입력 작업의 접수와 최종상태
CREATE TABLE IF NOT EXISTS ai_submissions (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL UNIQUE REFERENCES generation_tasks(id) ON DELETE CASCADE,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL,
  input_message_id TEXT REFERENCES messages(id) ON DELETE SET NULL,  
  status TEXT NOT NULL,
  -- 현재 실행시도
  current_attempt_id TEXT,
  attempt_count INTEGER NOT NULL DEFAULT 0,
  -- 최대 실행횟수
  max_attempts INTEGER NOT NULL DEFAULT 3,
  created_at INTEGER NOT NULL,
  started_at INTEGER,
  updated_at INTEGER NOT NULL,
  finished_at INTEGER,

  UNIQUE(conversation_id, seq)
);

-- 실행 시도, 오류 재시도 기록
CREATE TABLE IF NOT EXISTS ai_attempts (
  id TEXT PRIMARY KEY,
  submission_id TEXT NOT NULL REFERENCES ai_submissions(id) ON DELETE CASCADE,
  attempt_no INTEGER NOT NULL,
  status TEXT NOT NULL,
  error_code TEXT,
  error_message TEXT,
  started_at INTEGER,
  finished_at INTEGER,
  UNIQUE(submission_id, attempt_no)
);

-- 스트리밍 및 실행 이벤트
-- ai 답변 원본 저장 / message는 UI 표시용
CREATE TABLE IF NOT EXISTS conversation_events (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  submission_id TEXT NOT NULL REFERENCES ai_submissions(id) ON DELETE CASCADE,
  attempt_id TEXT REFERENCES ai_attempts(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL,
  event_type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,

  UNIQUE(conversation_id, seq)
);

-- - task_attempts: 작업 종류와 무관한 실행 시도 기록
--   (ai_attempts는 채팅 submission 전용이라 이미지/동기화 작업에는 쓸 수 없음)
-- - 상태 변경은 앱 코드에서 TaskManager만 수행한다.
CREATE TABLE IF NOT EXISTS task_attempts (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES generation_tasks(id) ON DELETE CASCADE,
  -- 작업 전체 기준 순번 (재개해도 이어서 증가)
  attempt_no INTEGER NOT NULL,
  -- running / completed / failed / cancelled / interrupted
  status TEXT NOT NULL,
  error_code TEXT,
  error_message TEXT,
  started_at INTEGER NOT NULL,
  finished_at INTEGER,
  UNIQUE(task_id, attempt_no)
);

-- ---------------------------------------------------------
-- 에이전트
-- ---------------------------------------------------------
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

-- =========================================================
-- 인덱스
-- =========================================================

CREATE INDEX IF NOT EXISTS idx_ai_submissions_status ON ai_submissions(status);
CREATE INDEX IF NOT EXISTS idx_ai_submissions_conversation ON ai_submissions(conversation_id);
CREATE INDEX IF NOT EXISTS idx_generation_tasks_status ON generation_tasks(status);
CREATE INDEX IF NOT EXISTS idx_generation_tasks_owner ON generation_tasks(owner, owner_ref);
CREATE INDEX IF NOT EXISTS idx_task_attempts_task ON task_attempts(task_id, attempt_no);
CREATE INDEX IF NOT EXISTS idx_generation_tasks_created ON generation_tasks(created_at);
