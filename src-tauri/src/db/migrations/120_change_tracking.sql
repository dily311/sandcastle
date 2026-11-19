-- 2-A-2 변경 추적 (CLAUDE.md, ADR-011)
-- 동기화 대상 테이블(ADR-011 표)에 updated_at(밀리초)을 둔다. 지우기 기록은 157.
-- 모든 쓰기가 updated_at을 갱신하도록 트리거로 보장한다: 코드가 updated_at을 직접 바꾸면 그 값을 두고(동기화 병합이 원래 시각을 쓸 수 있게),
-- 바꾸지 않으면 지금 시각으로. 예전 초 단위 값(< 1e11)은 밀리초로 맞춘다.
-- 새 동기화 대상 테이블을 만들면 같은 컬럼과 트리거를 붙이고 ADR-011 표에 적는다.

-- agent_definitions
UPDATE agent_definitions SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE agent_definitions SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE agent_definitions SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER agent_definitions_touch_insert AFTER INSERT ON agent_definitions FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE agent_definitions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER agent_definitions_touch_update AFTER UPDATE ON agent_definitions FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE agent_definitions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- api_profiles
UPDATE api_profiles SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE api_profiles SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE api_profiles SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER api_profiles_touch_insert AFTER INSERT ON api_profiles FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE api_profiles SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER api_profiles_touch_update AFTER UPDATE ON api_profiles FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE api_profiles SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- budget_limits
UPDATE budget_limits SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE budget_limits SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER budget_limits_touch_insert AFTER INSERT ON budget_limits FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE budget_limits SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER budget_limits_touch_update AFTER UPDATE ON budget_limits FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE budget_limits SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- character_images
UPDATE character_images SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE character_images SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE character_images SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER character_images_touch_insert AFTER INSERT ON character_images FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE character_images SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER character_images_touch_update AFTER UPDATE ON character_images FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE character_images SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- character_worldinfo
UPDATE character_worldinfo SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE character_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER character_worldinfo_touch_insert AFTER INSERT ON character_worldinfo FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE character_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER character_worldinfo_touch_update AFTER UPDATE ON character_worldinfo FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE character_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- characters
UPDATE characters SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE characters SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE characters SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER characters_touch_insert AFTER INSERT ON characters FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE characters SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER characters_touch_update AFTER UPDATE ON characters FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE characters SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- collection_messages
UPDATE collection_messages SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE collection_messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER collection_messages_touch_insert AFTER INSERT ON collection_messages FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE collection_messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER collection_messages_touch_update AFTER UPDATE ON collection_messages FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE collection_messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- conv_settings
UPDATE conv_settings SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE conv_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER conv_settings_touch_insert AFTER INSERT ON conv_settings FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE conv_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER conv_settings_touch_update AFTER UPDATE ON conv_settings FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE conv_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- conversation_folders
UPDATE conversation_folders SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE conversation_folders SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE conversation_folders SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER conversation_folders_touch_insert AFTER INSERT ON conversation_folders FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE conversation_folders SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER conversation_folders_touch_update AFTER UPDATE ON conversation_folders FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE conversation_folders SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- conversations
UPDATE conversations SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE conversations SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE conversations SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER conversations_touch_insert AFTER INSERT ON conversations FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE conversations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER conversations_touch_update AFTER UPDATE ON conversations FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE conversations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- memory_settings
UPDATE memory_settings SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE memory_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER memory_settings_touch_insert AFTER INSERT ON memory_settings FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE memory_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER memory_settings_touch_update AFTER UPDATE ON memory_settings FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE memory_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- message_bookmarks
ALTER TABLE message_bookmarks ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE message_bookmarks SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE message_bookmarks SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE message_bookmarks SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER message_bookmarks_touch_insert AFTER INSERT ON message_bookmarks FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE message_bookmarks SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER message_bookmarks_touch_update AFTER UPDATE ON message_bookmarks FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE message_bookmarks SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- message_collections
UPDATE message_collections SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE message_collections SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE message_collections SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER message_collections_touch_insert AFTER INSERT ON message_collections FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE message_collections SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER message_collections_touch_update AFTER UPDATE ON message_collections FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE message_collections SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- message_tag_relations
UPDATE message_tag_relations SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE message_tag_relations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER message_tag_relations_touch_insert AFTER INSERT ON message_tag_relations FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE message_tag_relations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER message_tag_relations_touch_update AFTER UPDATE ON message_tag_relations FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE message_tag_relations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- message_tags
UPDATE message_tags SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE message_tags SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER message_tags_touch_insert AFTER INSERT ON message_tags FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE message_tags SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER message_tags_touch_update AFTER UPDATE ON message_tags FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE message_tags SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- messages
UPDATE messages SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE messages SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE messages SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER messages_touch_insert AFTER INSERT ON messages FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER messages_touch_update AFTER UPDATE ON messages FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- model_prices
UPDATE model_prices SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE model_prices SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER model_prices_touch_insert AFTER INSERT ON model_prices FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE model_prices SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER model_prices_touch_update AFTER UPDATE ON model_prices FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE model_prices SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- persona_worldinfo
UPDATE persona_worldinfo SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE persona_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER persona_worldinfo_touch_insert AFTER INSERT ON persona_worldinfo FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE persona_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER persona_worldinfo_touch_update AFTER UPDATE ON persona_worldinfo FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE persona_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- personas
UPDATE personas SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE personas SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE personas SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER personas_touch_insert AFTER INSERT ON personas FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE personas SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER personas_touch_update AFTER UPDATE ON personas FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE personas SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- proactive_settings
UPDATE proactive_settings SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE proactive_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER proactive_settings_touch_insert AFTER INSERT ON proactive_settings FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE proactive_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER proactive_settings_touch_update AFTER UPDATE ON proactive_settings FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE proactive_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- prompt_blocks
UPDATE prompt_blocks SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE prompt_blocks SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE prompt_blocks SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER prompt_blocks_touch_insert AFTER INSERT ON prompt_blocks FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE prompt_blocks SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER prompt_blocks_touch_update AFTER UPDATE ON prompt_blocks FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE prompt_blocks SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- prompt_presets
UPDATE prompt_presets SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE prompt_presets SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE prompt_presets SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER prompt_presets_touch_insert AFTER INSERT ON prompt_presets FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE prompt_presets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER prompt_presets_touch_update AFTER UPDATE ON prompt_presets FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE prompt_presets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- worldinfo_books
UPDATE worldinfo_books SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE worldinfo_books SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE worldinfo_books SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER worldinfo_books_touch_insert AFTER INSERT ON worldinfo_books FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE worldinfo_books SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER worldinfo_books_touch_update AFTER UPDATE ON worldinfo_books FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE worldinfo_books SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- worldinfo_entries
UPDATE worldinfo_entries SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE worldinfo_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
CREATE TRIGGER worldinfo_entries_touch_insert AFTER INSERT ON worldinfo_entries FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE worldinfo_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER worldinfo_entries_touch_update AFTER UPDATE ON worldinfo_entries FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE worldinfo_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
