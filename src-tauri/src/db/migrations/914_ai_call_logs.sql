-- AI 호출 기록 (5-E-20-6-7): 보낸 프롬프트·받은 원문·끝난 이유를 작업에 묶어 남긴다.
-- 설정 > 실행 기록에서 펼쳐 본다 (빠른 답장 /gen의 맥락 확인 등). 켜야만 남기고(app-setting aiCallLog, 기본 꺼짐), 최근 200개만 둔다.
-- 기기 전용 — 동기화하지 않는다 (change_tracking UNSYNCED_TABLES).
CREATE TABLE IF NOT EXISTS ai_call_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id TEXT,
  profile_id TEXT NOT NULL,
  model TEXT NOT NULL,
  -- 보낸 것: { messages: [{ role, content }], options: { … } }
  request_json TEXT NOT NULL CHECK (json_valid(request_json)),
  -- 받은 원문 (실패·취소면 그때까지 받은 글)
  response_text TEXT NOT NULL DEFAULT '',
  finish_reason TEXT,
  error TEXT,
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ai_call_logs_task ON ai_call_logs (task_id);
