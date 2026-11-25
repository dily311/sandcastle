-- 동기화 기기 이름 (5-E-16): 그 기기가 묶음에 적어 보낸 이름. 없으면 화면이 id 앞자리를 보인다
ALTER TABLE sync_peers ADD COLUMN name TEXT;
