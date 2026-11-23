-- 프로필 그림 확대·위치 (5-E-17-3): {"x":0~100,"y":0~100,"zoom":1~4} JSON, NULL = 가운데·확대 없음
ALTER TABLE characters ADD COLUMN avatar_fit TEXT;
ALTER TABLE personas ADD COLUMN avatar_fit TEXT;
