-- 2-D 점검에서 찾음: 받은 메시지를 분기 대화로 옮길 때 새 id를 준 것을 기억한다 (받은 id → 이 기기 id).
-- 같은 메시지가 다른 기기를 거쳐 다시 와도 분기를 또 만들지 않고, 그 메시지를 가리키는 책갈피 등도 같은 새 id로 (sync_merge)
CREATE TABLE sync_id_map (
  tbl TEXT NOT NULL,
  remote_id TEXT NOT NULL,
  local_id TEXT NOT NULL,
  PRIMARY KEY (tbl, remote_id)
) WITHOUT ROWID;
