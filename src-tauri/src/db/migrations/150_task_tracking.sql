-- =========================================================
-- 작업 실행 관리 · 사용량 (CLAUDE.md M2, ADR-006)
-- ---------------------------------------------------------
-- generation_tasks: 어느 기능이(feature), 누가 시작했는지(run_by), 화면에 보일 이름(label)
-- usage_log: 공급자 호출 한 번마다 사용량 (ResourceManager). 작업이 지워져도 사용량은 남긴다
-- =========================================================

-- 코어 플러그인·기능 id: chat / phone / game / translate / tts / imagegen / settings
ALTER TABLE generation_tasks ADD COLUMN feature TEXT;
-- user: 사용자가 누른 실행 / auto: 사용자가 켜 둔 자동 실행(자동 번역·자동 읽기 등)
ALTER TABLE generation_tasks ADD COLUMN run_by TEXT NOT NULL DEFAULT 'user';
-- 화면 표시용 한 줄 ("번역", "NPC 대화 · 엘리안")
ALTER TABLE generation_tasks ADD COLUMN label TEXT;

CREATE INDEX IF NOT EXISTS idx_generation_tasks_created ON generation_tasks(created_at);

CREATE TABLE IF NOT EXISTS usage_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id TEXT REFERENCES generation_tasks(id) ON DELETE SET NULL,
  feature TEXT,
  api_profile_id TEXT,
  model TEXT,
  -- chat: 텍스트 생성 / tts: 음성 / image: 이미지
  kind TEXT NOT NULL,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  -- 음성으로 만든 글자 수
  characters INTEGER NOT NULL DEFAULT 0,
  -- 만든 이미지 장 수
  images INTEGER NOT NULL DEFAULT 0,
  -- 1: 토큰을 앱이 추정 / 0: 공급자가 알려준 값
  estimated INTEGER NOT NULL DEFAULT 1,
  -- 1: 성공 / 0: 실패 (실패해도 요금이 나갈 수 있어 남긴다)
  ok INTEGER NOT NULL DEFAULT 1,
  error TEXT,
  duration_ms INTEGER,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_usage_log_created ON usage_log(created_at);
CREATE INDEX IF NOT EXISTS idx_usage_log_task ON usage_log(task_id);
