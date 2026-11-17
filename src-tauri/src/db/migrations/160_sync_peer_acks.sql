-- 2-C/2-D 기기가 셋 이상일 때: 그 기기 묶음의 acks(그 기기가 합친 다른 기기 묶음의 시각)를 기억한다.
-- 받은 행이 이 기기 행의 출처 기기가 이미 본 옛 판이면 충돌이 아니라 건너뛴다 (sync_merge)
ALTER TABLE sync_peers ADD COLUMN acks TEXT;
