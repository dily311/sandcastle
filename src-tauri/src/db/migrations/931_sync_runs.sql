-- 2-C 폴더 동기화 (CLAUDE.md, ADR-011)

-- 동기화 실행 기록 (설정 > 동기화의 기록). 작업·시도 기록(generation_tasks)과 따로, 숫자 요약을 남긴다
CREATE TABLE IF NOT EXISTS sync_runs (
  id TEXT PRIMARY KEY,
  target TEXT NOT NULL,            -- 'folder' (나중에 'drive')
  run_by TEXT NOT NULL,            -- user / auto
  started_at INTEGER NOT NULL,
  finished_at INTEGER,
  ok INTEGER NOT NULL DEFAULT 0,
  error TEXT,
  summary TEXT                     -- SyncRunReport JSON
);
CREATE INDEX IF NOT EXISTS idx_sync_runs_started ON sync_runs(started_at);

-- 받은 행이 어느 기기의 어느 판이었나 (기기가 셋 이상일 때). 이 기기가 그 뒤로 안 고쳤고(updated_at = merged_at),
-- 다른 기기의 묶음이 그 판을 봤다고 하면(acks) 그 기기의 고침을 충돌이 아니라 새 판으로 받는다
CREATE TABLE IF NOT EXISTS sync_row_origin (
  tbl TEXT NOT NULL,
  key TEXT NOT NULL,
  device_id TEXT NOT NULL,
  remote_at INTEGER NOT NULL,      -- 그 기기 시계의 updated_at
  merged_at INTEGER NOT NULL,      -- 이 기기가 받아 쓴 시각 (= 이 기기 행의 updated_at)
  PRIMARY KEY (tbl, key)
) WITHOUT ROWID;
