-- 고치기 제안 (CLAUDE.md 6-D-3, ADR-012). 플러그인(나중에는 에이전트 5-B)이 데이터를 바로 바꾸지 않고 제안을 만들면,
-- 사용자가 바뀌는 칸을 보고 적용/거절한다. 적용한 것은 되돌릴 수 있게 전과 후를 남긴다.
-- 기기마다 따로 (동기화 안 함 — 적용한 결과의 행은 원래 테이블에서 동기화된다).
CREATE TABLE IF NOT EXISTS change_proposals (
  id TEXT PRIMARY KEY NOT NULL,
  -- 누가: plugin:<id> / agent:<id>
  source TEXT NOT NULL,
  -- character / persona / lorebook / lorebook_entry / message
  kind TEXT NOT NULL,
  -- update / create
  op TEXT NOT NULL,
  -- 고칠 행 (create는 적용한 뒤 만든 행)
  target_id TEXT,
  -- create의 부모 (lorebook_entry의 book_id)
  parent_id TEXT,
  -- 보여줄 이름 (제안할 때의 이름·제목)
  target_label TEXT NOT NULL DEFAULT '',
  -- JSON {칸: 값}: 제안할 때의 값 / 바꿀 값
  before_json TEXT NOT NULL DEFAULT '{}',
  after_json TEXT NOT NULL,
  reason TEXT NOT NULL DEFAULT '',
  -- pending / applied / rejected / reverted
  status TEXT NOT NULL DEFAULT 'pending',
  created_at INTEGER NOT NULL,
  decided_at INTEGER
);
CREATE INDEX change_proposals_status ON change_proposals(status, created_at);
