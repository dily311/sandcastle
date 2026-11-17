-- 2-B-2 동기화 병합 (CLAUDE.md, ADR-011)

-- 합친 적 있는 다른 기기. 시각은 늘 같은 시계끼리만 비교한다:
--   last_bundle_created_at = 마지막으로 합친 그 기기 묶음의 created_at (그 기기 시계) — 그보다 오래된 행·지운 기록은 이미 본 것
--   last_merge_at          = 그때 이 기기의 시각 (이 기기 시계) — 그 뒤에 이 기기에서 고친 행은 "이쪽도 바뀜"
CREATE TABLE sync_peers (
  device_id TEXT PRIMARY KEY,
  last_bundle_created_at INTEGER NOT NULL,
  last_merge_at INTEGER NOT NULL,
  merges INTEGER NOT NULL DEFAULT 0
);

-- 합칠 때 둘 다 남긴 것 (2-E 화면에서 고른다)
--   edit   같은 행을 양쪽에서 고침: 이 기기 것을 두고 받은 행을 remote_row에
--   unique 다른 UNIQUE 규칙에 걸려 넣지 못한 받은 행
--   orphan 이 기기에 없는 항목을 가리켜 넣지 못한 받은 행
--   branch 같은 대화에 양쪽이 이어 씀: 받은 메시지를 분기 대화로 (remote_row = {branchId, fromSeq})
CREATE TABLE sync_conflicts (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  tbl TEXT NOT NULL,
  key TEXT NOT NULL,
  local_row TEXT,
  remote_row TEXT,
  device_id TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  resolved_at INTEGER
);
CREATE INDEX idx_sync_conflicts_open ON sync_conflicts(resolved_at, created_at);
