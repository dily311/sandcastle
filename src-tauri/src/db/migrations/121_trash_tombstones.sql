-- 2-A-3 휴지통과 지운 기록 (CLAUDE.md, ADR-011)
-- 휴지통: 지울 때 항목과 함께 지워지는 행(CASCADE)을 통째로 trash_items에 떠 두고 실제로 지운다(목록 조회는 그대로).
-- 지운 기록(sync_tombstones): 동기화 대상 테이블의 모든 DELETE를 (테이블, 키)로 남긴다 — 2-B 병합이 다른 기기에 지우기를 전한다.
--   같은 키로 다시 들어오면(되살리기·다시 연결) 기록을 지운다. 키는 기본 키, 없으면 UNIQUE 묶음의 json_array.

CREATE TABLE trash_items (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,              -- character / persona / conversation / worldinfo_book / prompt_preset / game / game_session
  item_id TEXT NOT NULL,
  label TEXT NOT NULL,
  payload TEXT NOT NULL,           -- 지운 행들 (service/data/trash.rs의 Snapshot)
  files TEXT NOT NULL DEFAULT '[]', -- 휴지통 폴더로 옮긴 파일 [{from, to}]
  size INTEGER NOT NULL DEFAULT 0, -- payload 바이트 수
  row_count INTEGER NOT NULL DEFAULT 0, -- 같이 지운 행 수 (목록에서 payload를 읽지 않게)
  deleted_at INTEGER NOT NULL
);
CREATE INDEX idx_trash_items_deleted_at ON trash_items(deleted_at);

CREATE TABLE sync_tombstones (
  tbl TEXT NOT NULL,
  key TEXT NOT NULL,
  deleted_at INTEGER NOT NULL,
  PRIMARY KEY (tbl, key)
) WITHOUT ROWID;

-- agent_definitions (id)
CREATE TRIGGER agent_definitions_tombstone AFTER DELETE ON agent_definitions FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('agent_definitions', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER agent_definitions_untombstone AFTER INSERT ON agent_definitions FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'agent_definitions' AND key = json_array(NEW.id); END;

-- api_profiles (id)
CREATE TRIGGER api_profiles_tombstone AFTER DELETE ON api_profiles FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('api_profiles', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER api_profiles_untombstone AFTER INSERT ON api_profiles FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'api_profiles' AND key = json_array(NEW.id); END;

-- budget_limits (api_profile_id)
CREATE TRIGGER budget_limits_tombstone AFTER DELETE ON budget_limits FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('budget_limits', json_array(OLD.api_profile_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER budget_limits_untombstone AFTER INSERT ON budget_limits FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'budget_limits' AND key = json_array(NEW.api_profile_id); END;

-- character_images (id)
CREATE TRIGGER character_images_tombstone AFTER DELETE ON character_images FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('character_images', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER character_images_untombstone AFTER INSERT ON character_images FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'character_images' AND key = json_array(NEW.id); END;

-- character_worldinfo (character_id, book_id)
CREATE TRIGGER character_worldinfo_tombstone AFTER DELETE ON character_worldinfo FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('character_worldinfo', json_array(OLD.character_id, OLD.book_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER character_worldinfo_untombstone AFTER INSERT ON character_worldinfo FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'character_worldinfo' AND key = json_array(NEW.character_id, NEW.book_id); END;

-- characters (id)
CREATE TRIGGER characters_tombstone AFTER DELETE ON characters FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('characters', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER characters_untombstone AFTER INSERT ON characters FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'characters' AND key = json_array(NEW.id); END;

-- collection_messages (collection_id, message_id)
CREATE TRIGGER collection_messages_tombstone AFTER DELETE ON collection_messages FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('collection_messages', json_array(OLD.collection_id, OLD.message_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER collection_messages_untombstone AFTER INSERT ON collection_messages FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'collection_messages' AND key = json_array(NEW.collection_id, NEW.message_id); END;

-- conv_settings (id)
CREATE TRIGGER conv_settings_tombstone AFTER DELETE ON conv_settings FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('conv_settings', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER conv_settings_untombstone AFTER INSERT ON conv_settings FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'conv_settings' AND key = json_array(NEW.id); END;

-- conversation_folders (id)
CREATE TRIGGER conversation_folders_tombstone AFTER DELETE ON conversation_folders FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('conversation_folders', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER conversation_folders_untombstone AFTER INSERT ON conversation_folders FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'conversation_folders' AND key = json_array(NEW.id); END;

-- conversations (id)
CREATE TRIGGER conversations_tombstone AFTER DELETE ON conversations FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('conversations', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER conversations_untombstone AFTER INSERT ON conversations FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'conversations' AND key = json_array(NEW.id); END;

-- memory_settings (id)
CREATE TRIGGER memory_settings_tombstone AFTER DELETE ON memory_settings FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('memory_settings', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER memory_settings_untombstone AFTER INSERT ON memory_settings FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'memory_settings' AND key = json_array(NEW.id); END;

-- message_bookmarks (id)
CREATE TRIGGER message_bookmarks_tombstone AFTER DELETE ON message_bookmarks FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('message_bookmarks', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER message_bookmarks_untombstone AFTER INSERT ON message_bookmarks FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'message_bookmarks' AND key = json_array(NEW.id); END;

-- message_collections (id)
CREATE TRIGGER message_collections_tombstone AFTER DELETE ON message_collections FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('message_collections', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER message_collections_untombstone AFTER INSERT ON message_collections FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'message_collections' AND key = json_array(NEW.id); END;

-- message_tag_relations (message_id, tag_id)
CREATE TRIGGER message_tag_relations_tombstone AFTER DELETE ON message_tag_relations FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('message_tag_relations', json_array(OLD.message_id, OLD.tag_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER message_tag_relations_untombstone AFTER INSERT ON message_tag_relations FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'message_tag_relations' AND key = json_array(NEW.message_id, NEW.tag_id); END;

-- message_tags (id)
CREATE TRIGGER message_tags_tombstone AFTER DELETE ON message_tags FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('message_tags', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER message_tags_untombstone AFTER INSERT ON message_tags FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'message_tags' AND key = json_array(NEW.id); END;

-- messages (id)
CREATE TRIGGER messages_tombstone AFTER DELETE ON messages FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('messages', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER messages_untombstone AFTER INSERT ON messages FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'messages' AND key = json_array(NEW.id); END;

-- model_prices (model)
CREATE TRIGGER model_prices_tombstone AFTER DELETE ON model_prices FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('model_prices', json_array(OLD.model), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER model_prices_untombstone AFTER INSERT ON model_prices FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'model_prices' AND key = json_array(NEW.model); END;

-- persona_worldinfo (persona_id, book_id)
CREATE TRIGGER persona_worldinfo_tombstone AFTER DELETE ON persona_worldinfo FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('persona_worldinfo', json_array(OLD.persona_id, OLD.book_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER persona_worldinfo_untombstone AFTER INSERT ON persona_worldinfo FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'persona_worldinfo' AND key = json_array(NEW.persona_id, NEW.book_id); END;

-- personas (id)
CREATE TRIGGER personas_tombstone AFTER DELETE ON personas FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('personas', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER personas_untombstone AFTER INSERT ON personas FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'personas' AND key = json_array(NEW.id); END;

-- proactive_settings (id)
CREATE TRIGGER proactive_settings_tombstone AFTER DELETE ON proactive_settings FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('proactive_settings', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER proactive_settings_untombstone AFTER INSERT ON proactive_settings FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'proactive_settings' AND key = json_array(NEW.id); END;

-- prompt_blocks (id)
CREATE TRIGGER prompt_blocks_tombstone AFTER DELETE ON prompt_blocks FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('prompt_blocks', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER prompt_blocks_untombstone AFTER INSERT ON prompt_blocks FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'prompt_blocks' AND key = json_array(NEW.id); END;

-- prompt_presets (id)
CREATE TRIGGER prompt_presets_tombstone AFTER DELETE ON prompt_presets FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('prompt_presets', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER prompt_presets_untombstone AFTER INSERT ON prompt_presets FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'prompt_presets' AND key = json_array(NEW.id); END;

-- worldinfo_books (id)
CREATE TRIGGER worldinfo_books_tombstone AFTER DELETE ON worldinfo_books FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('worldinfo_books', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER worldinfo_books_untombstone AFTER INSERT ON worldinfo_books FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'worldinfo_books' AND key = json_array(NEW.id); END;

-- worldinfo_entries (id)
CREATE TRIGGER worldinfo_entries_tombstone AFTER DELETE ON worldinfo_entries FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('worldinfo_entries', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER worldinfo_entries_untombstone AFTER INSERT ON worldinfo_entries FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'worldinfo_entries' AND key = json_array(NEW.id); END;
