-- 1-F-3-3: 오래 못 본 기기를 사용자가 "제외"하면 지운 기록 정리가 그 기기를 기다리지 않는다. 기기마다 (sync_peers와 같이 동기화 안 함).
ALTER TABLE sync_peers ADD COLUMN excluded INTEGER NOT NULL DEFAULT 0;
