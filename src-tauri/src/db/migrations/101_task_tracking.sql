-- =========================================================
-- 사용량 (CLAUDE.md M2, ADR-006)
-- ---------------------------------------------------------
-- usage_log: 공급자 호출 한 번마다 사용량 (ResourceManager). 작업이 지워져도 사용량은 남긴다
-- =========================================================

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
  -- - 호출마다 그때의 단가로 계산한 금액(cost_usd)을 같이 기록한다. 나중에 단가를 고쳐도 지난 기록은 안 바뀐다.
  --   cost_usd가 NULL이면 계산하지 않은 줄(음성·그림·이 기능 이전의 줄). 단가를 몰라 계산하지 못한 호출은 unpriced = 1.
  -- - budget_limits에는 monthly_usd를 더한다. 예전 칸(하루·한 달 토큰, 음성 글자, 그림 장 수)은 더 이상 쓰지 않지만
  --   지우지 않는다(새 한도를 정할 때 비워 "이전 한도" 안내가 사라진다).
  cost_usd REAL,
  unpriced INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_usage_log_created ON usage_log(created_at);
CREATE INDEX IF NOT EXISTS idx_usage_log_task ON usage_log(task_id);
