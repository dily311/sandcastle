-- 캐릭터 에셋 판 폴더 (5-E-21): 그림마다 판 폴더 이름(빈 글 = 공통), 캐릭터마다 쓸 판(NULL = 전체)
ALTER TABLE character_images ADD COLUMN folder TEXT NOT NULL DEFAULT '';
ALTER TABLE characters ADD COLUMN asset_version TEXT;
-- 같은 키는 같은 판 안에서만 겹치지 않으면 된다 (v1/smile·v2/smile)
DROP INDEX IF EXISTS idx_character_images_key;
CREATE UNIQUE INDEX IF NOT EXISTS idx_character_images_key ON character_images(character_id, folder, key COLLATE NOCASE);
