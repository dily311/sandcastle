-- =========================================================
-- 빠른 답장 세트 (5-E-20-5-4, ADR-025)
-- ---------------------------------------------------------
-- 정규식처럼 scope: global / character / preset (scope_id = character_id / preset_id).
-- 버튼 목록은 items(JSON 배열: { id, label, message, hidden }) 한 칸에 둔다 — 세트 단위로 고치고 동기화한다.
-- 켜짐/꺼짐은 코어 플러그인 slash. 대화마다 끈 세트는 대화 meta.quickReplyOff.
-- =========================================================
CREATE TABLE IF NOT EXISTS quick_reply_sets (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  scope TEXT NOT NULL DEFAULT 'global' CHECK (scope IN ('global', 'character', 'preset')),
  scope_id TEXT,
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  -- 스크립트가 아닌 글: 1이면 바로 보냄(AI 답장 요금), 0이면 입력칸에 넣기만
  auto_send INTEGER NOT NULL DEFAULT 0 CHECK (auto_send IN (0, 1)),
  items TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(items)),
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_quick_reply_sets_scope ON quick_reply_sets (scope, scope_id);
