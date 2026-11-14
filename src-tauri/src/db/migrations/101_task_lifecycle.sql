-- =========================================================
-- 전역 작업 관리 (TaskManager) 확장
-- ---------------------------------------------------------
-- - generation_tasks: 요청자(owner), 결과, 오류 코드, 시도 횟수 추가
-- - task_attempts: 작업 종류와 무관한 실행 시도 기록
--   (ai_attempts는 채팅 submission 전용이라 이미지/동기화 작업에는 쓸 수 없음)
-- - 상태 변경은 앱 코드에서 TaskManager만 수행한다.
-- =========================================================

-- chat / game / image / system
ALTER TABLE generation_tasks ADD COLUMN owner TEXT NOT NULL DEFAULT 'system';
-- conversation_id, game session_id 등 owner 쪽 식별자
ALTER TABLE generation_tasks ADD COLUMN owner_ref TEXT;
-- 성공 시 TaskOutput (JSON)
ALTER TABLE generation_tasks ADD COLUMN result_json TEXT;
-- 실패 시 ErrorCode 문자열 (error_message와 함께 사용)
ALTER TABLE generation_tasks ADD COLUMN error_code TEXT;
ALTER TABLE generation_tasks ADD COLUMN attempt_count INTEGER NOT NULL DEFAULT 0;
-- 한 번의 실행에서 허용하는 최대 시도 횟수 (재시도 가능한 오류일 때만 다시 시도)
ALTER TABLE generation_tasks ADD COLUMN max_attempts INTEGER NOT NULL DEFAULT 1;

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

CREATE INDEX IF NOT EXISTS idx_generation_tasks_status ON generation_tasks(status);
CREATE INDEX IF NOT EXISTS idx_generation_tasks_owner ON generation_tasks(owner, owner_ref);
CREATE INDEX IF NOT EXISTS idx_task_attempts_task ON task_attempts(task_id, attempt_no);
