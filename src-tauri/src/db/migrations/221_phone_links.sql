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

-- ---------------------------------------------------------------------------
-- 변경 추적(updated_at 보정·트리거)과 지운 기록 트리거: 이 파일의 테이블 몫. 공통 테이블(trash_items·sync_tombstones)은 1xx에서 먼저 만들어짐
-- ---------------------------------------------------------------------------

-- phone_links
UPDATE phone_links SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE phone_links SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE phone_links SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER phone_links_touch_insert AFTER INSERT ON phone_links FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE phone_links SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER phone_links_touch_update AFTER UPDATE ON phone_links FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE phone_links SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- phone_links (owner_kind, owner_id, contact_kind, contact_id)
CREATE TRIGGER phone_links_tombstone AFTER DELETE ON phone_links FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('phone_links', json_array(OLD.owner_kind, OLD.owner_id, OLD.contact_kind, OLD.contact_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER phone_links_untombstone AFTER INSERT ON phone_links FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'phone_links' AND key = json_array(NEW.owner_kind, NEW.owner_id, NEW.contact_kind, NEW.contact_id); END;
