-- 커버 이미지에서 보일 세로 위치 (0 = 맨 위, 100 = 맨 아래, background-position-y %)
ALTER TABLE characters ADD COLUMN cover_y REAL NOT NULL DEFAULT 50;
