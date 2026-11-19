-- 대화 요약 (CLAUDE.md 3-D, ADR-016). 메모리 확장(코어 플러그인 summary)이 만드는 "지난 줄거리"로, 사용자의 메모와는 따로다.
-- 사용자가 고친 글이 들어 있으므로 백업에는 들어가지만, 자동 요약이 쓰는 행이 동기화 충돌을 만들지 않게 동기화하지 않는다(change_tracking::UNSYNCED_TABLES) — 기기마다 따로 요약한다.
-- 대화를 지우면 같이 지워지고 휴지통 스냅샷(자식 행)에 들어가 되살릴 때 돌아온다.
CREATE TABLE IF NOT EXISTS conversation_summaries (
  conversation_id TEXT PRIMARY KEY NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  text TEXT NOT NULL DEFAULT '',
  -- 요약에 반영된 마지막 보이는 메시지 id와 개수 (id 위치로 다시 계산해 지움·분기에도 스스로 맞춘다)
  covered_until_id TEXT,
  covered_count INTEGER NOT NULL DEFAULT 0,
  -- 쓸 때마다 올라가는 판. 자동 요약은 시작할 때의 판을 기억했다가 같을 때만 저장한다 (그사이 사용자가 고쳤으면 버림)
  rev INTEGER NOT NULL DEFAULT 0,
  edited_by_user INTEGER NOT NULL DEFAULT 0,
  -- 한 단계 전 글 ("되돌리기")
  prev_text TEXT,
  tokens INTEGER NOT NULL DEFAULT 0,
  -- {at, reason, visibleCount}: 자동 요약이 실패했으면 메시지가 더 쌓이기 전에는 다시 시도하지 않는다
  failure_json TEXT,
  updated_at INTEGER NOT NULL DEFAULT 0
);
