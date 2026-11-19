-- 폰 연락처를 넓힘: 폰 주인과 연락처가 각각 페르소나 또는 캐릭터일 수 있음
-- (캐릭터의 폰: 주인 = 캐릭터, 연락처 = 다른 캐릭터나 내 페르소나)
-- 종류가 둘이라 외래 키 대신 트리거로 지워진 쪽을 정리한다
CREATE TABLE IF NOT EXISTS phone_links (
  owner_kind   TEXT NOT NULL CHECK (owner_kind IN ('persona', 'character')),
  owner_id     TEXT NOT NULL,
  contact_kind TEXT NOT NULL CHECK (contact_kind IN ('persona', 'character')),
  contact_id   TEXT NOT NULL,
  status       TEXT NOT NULL DEFAULT '',
  sort_order   INTEGER NOT NULL DEFAULT 0,
  created_at   INTEGER NOT NULL,
  updated_at INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (owner_kind, owner_id, contact_kind, contact_id)
);

INSERT OR IGNORE INTO phone_links (owner_kind, owner_id, contact_kind, contact_id, status, sort_order, created_at)
  SELECT 'persona', persona_id, 'character', character_id, status, sort_order, created_at FROM phone_contacts;

DROP TABLE IF EXISTS phone_contacts;

CREATE TRIGGER IF NOT EXISTS phone_links_character_deleted AFTER DELETE ON characters
BEGIN
  DELETE FROM phone_links
   WHERE (owner_kind = 'character' AND owner_id = OLD.id) OR (contact_kind = 'character' AND contact_id = OLD.id);
END;

CREATE TRIGGER IF NOT EXISTS phone_links_persona_deleted AFTER DELETE ON personas
BEGIN
  DELETE FROM phone_links
   WHERE (owner_kind = 'persona' AND owner_id = OLD.id) OR (contact_kind = 'persona' AND contact_id = OLD.id);
END;
