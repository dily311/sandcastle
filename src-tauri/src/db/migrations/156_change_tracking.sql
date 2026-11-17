-- 2-A-2 변경 추적 (CLAUDE.md, ADR-011)
-- 동기화 대상 테이블(ADR-011 표)에 updated_at(밀리초)과 deleted_at(휴지통 표시, 2-A-3)을 둔다.
-- 모든 쓰기가 updated_at을 갱신하도록 트리거로 보장한다: 코드가 updated_at을 직접 바꾸면 그 값을 두고(동기화 병합이 원래 시각을 쓸 수 있게),
-- 바꾸지 않으면 지금 시각으로. 예전 초 단위 값(< 1e11)은 밀리초로 맞춘다.
-- 새 동기화 대상 테이블을 만들면 같은 컬럼과 트리거를 붙이고 ADR-011 표에 적는다.

-- agent_definitions
UPDATE agent_definitions SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE agent_definitions SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE agent_definitions SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE agent_definitions ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER agent_definitions_touch_insert AFTER INSERT ON agent_definitions FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE agent_definitions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER agent_definitions_touch_update AFTER UPDATE ON agent_definitions FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE agent_definitions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- api_profiles
UPDATE api_profiles SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE api_profiles SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE api_profiles SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE api_profiles ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER api_profiles_touch_insert AFTER INSERT ON api_profiles FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE api_profiles SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER api_profiles_touch_update AFTER UPDATE ON api_profiles FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE api_profiles SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- audio_assets
ALTER TABLE audio_assets ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE audio_assets SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE audio_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE audio_assets ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER audio_assets_touch_insert AFTER INSERT ON audio_assets FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE audio_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER audio_assets_touch_update AFTER UPDATE ON audio_assets FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE audio_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- background_assets
ALTER TABLE background_assets ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE background_assets SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE background_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE background_assets ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER background_assets_touch_insert AFTER INSERT ON background_assets FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE background_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER background_assets_touch_update AFTER UPDATE ON background_assets FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE background_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- budget_limits
UPDATE budget_limits SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE budget_limits SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE budget_limits ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER budget_limits_touch_insert AFTER INSERT ON budget_limits FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE budget_limits SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER budget_limits_touch_update AFTER UPDATE ON budget_limits FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE budget_limits SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- cg_assets
ALTER TABLE cg_assets ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE cg_assets SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE cg_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE cg_assets ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER cg_assets_touch_insert AFTER INSERT ON cg_assets FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE cg_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER cg_assets_touch_update AFTER UPDATE ON cg_assets FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE cg_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- character_assets
ALTER TABLE character_assets ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE character_assets SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE character_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE character_assets ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER character_assets_touch_insert AFTER INSERT ON character_assets FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE character_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER character_assets_touch_update AFTER UPDATE ON character_assets FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE character_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- character_images
UPDATE character_images SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE character_images SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE character_images SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE character_images ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER character_images_touch_insert AFTER INSERT ON character_images FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE character_images SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER character_images_touch_update AFTER UPDATE ON character_images FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE character_images SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- character_worldinfo
ALTER TABLE character_worldinfo ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE character_worldinfo SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE character_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE character_worldinfo ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER character_worldinfo_touch_insert AFTER INSERT ON character_worldinfo FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE character_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER character_worldinfo_touch_update AFTER UPDATE ON character_worldinfo FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE character_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- characters
UPDATE characters SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE characters SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE characters SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE characters ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER characters_touch_insert AFTER INSERT ON characters FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE characters SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER characters_touch_update AFTER UPDATE ON characters FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE characters SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- collection_messages
ALTER TABLE collection_messages ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE collection_messages SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE collection_messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE collection_messages ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER collection_messages_touch_insert AFTER INSERT ON collection_messages FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE collection_messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER collection_messages_touch_update AFTER UPDATE ON collection_messages FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE collection_messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- conv_settings
UPDATE conv_settings SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE conv_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE conv_settings ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER conv_settings_touch_insert AFTER INSERT ON conv_settings FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE conv_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER conv_settings_touch_update AFTER UPDATE ON conv_settings FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE conv_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- conversation_folders
UPDATE conversation_folders SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE conversation_folders SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE conversation_folders SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE conversation_folders ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER conversation_folders_touch_insert AFTER INSERT ON conversation_folders FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE conversation_folders SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER conversation_folders_touch_update AFTER UPDATE ON conversation_folders FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE conversation_folders SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- conversations
UPDATE conversations SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE conversations SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE conversations SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE conversations ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER conversations_touch_insert AFTER INSERT ON conversations FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE conversations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER conversations_touch_update AFTER UPDATE ON conversations FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE conversations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_assets
UPDATE game_assets SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_assets SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_assets SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_assets ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_assets_touch_insert AFTER INSERT ON game_assets FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_assets_touch_update AFTER UPDATE ON game_assets FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_assets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_branches
ALTER TABLE game_branches ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_branches SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_branches SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_branches ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_branches_touch_insert AFTER INSERT ON game_branches FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_branches SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_branches_touch_update AFTER UPDATE ON game_branches FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_branches SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_character_spawn_locations
UPDATE game_character_spawn_locations SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_character_spawn_locations SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_character_spawn_locations SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_character_spawn_locations ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_character_spawn_locations_touch_insert AFTER INSERT ON game_character_spawn_locations FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_character_spawn_locations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_character_spawn_locations_touch_update AFTER UPDATE ON game_character_spawn_locations FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_character_spawn_locations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_character_state_snapshots
UPDATE game_character_state_snapshots SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_character_state_snapshots SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_character_state_snapshots ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_character_state_snapshots_touch_insert AFTER INSERT ON game_character_state_snapshots FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_character_state_snapshots SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_character_state_snapshots_touch_update AFTER UPDATE ON game_character_state_snapshots FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_character_state_snapshots SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_character_worldinfo
ALTER TABLE game_character_worldinfo ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_character_worldinfo SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_character_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_character_worldinfo ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_character_worldinfo_touch_insert AFTER INSERT ON game_character_worldinfo FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_character_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_character_worldinfo_touch_update AFTER UPDATE ON game_character_worldinfo FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_character_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_characters
UPDATE game_characters SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_characters SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_characters SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_characters ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_characters_touch_insert AFTER INSERT ON game_characters FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_characters SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_characters_touch_update AFTER UPDATE ON game_characters FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_characters SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_choices
ALTER TABLE game_choices ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_choices SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_choices SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_choices ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_choices_touch_insert AFTER INSERT ON game_choices FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_choices SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_choices_touch_update AFTER UPDATE ON game_choices FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_choices SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_conversation_participants
ALTER TABLE game_conversation_participants ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_conversation_participants SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_conversation_participants SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_conversation_participants ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_conversation_participants_touch_insert AFTER INSERT ON game_conversation_participants FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_conversation_participants SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_conversation_participants_touch_update AFTER UPDATE ON game_conversation_participants FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_conversation_participants SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_conversations
UPDATE game_conversations SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_conversations SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_conversations SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_conversations ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_conversations_touch_insert AFTER INSERT ON game_conversations FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_conversations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_conversations_touch_update AFTER UPDATE ON game_conversations FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_conversations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_encounter_entries
ALTER TABLE game_encounter_entries ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_encounter_entries SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_encounter_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_encounter_entries ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_encounter_entries_touch_insert AFTER INSERT ON game_encounter_entries FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_encounter_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_encounter_entries_touch_update AFTER UPDATE ON game_encounter_entries FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_encounter_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_encounter_tables
ALTER TABLE game_encounter_tables ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_encounter_tables SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_encounter_tables SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_encounter_tables ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_encounter_tables_touch_insert AFTER INSERT ON game_encounter_tables FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_encounter_tables SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_encounter_tables_touch_update AFTER UPDATE ON game_encounter_tables FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_encounter_tables SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_endings
ALTER TABLE game_endings ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_endings SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_endings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_endings ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_endings_touch_insert AFTER INSERT ON game_endings FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_endings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_endings_touch_update AFTER UPDATE ON game_endings FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_endings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_event_character_pool
ALTER TABLE game_event_character_pool ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_event_character_pool SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_event_character_pool SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_event_character_pool ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_event_character_pool_touch_insert AFTER INSERT ON game_event_character_pool FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_event_character_pool SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_event_character_pool_touch_update AFTER UPDATE ON game_event_character_pool FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_event_character_pool SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_event_conditions
ALTER TABLE game_event_conditions ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_event_conditions SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_event_conditions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_event_conditions ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_event_conditions_touch_insert AFTER INSERT ON game_event_conditions FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_event_conditions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_event_conditions_touch_update AFTER UPDATE ON game_event_conditions FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_event_conditions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_event_log
ALTER TABLE game_event_log ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_event_log SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_event_log SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_event_log ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_event_log_touch_insert AFTER INSERT ON game_event_log FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_event_log SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_event_log_touch_update AFTER UPDATE ON game_event_log FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_event_log SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_event_log_characters
ALTER TABLE game_event_log_characters ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_event_log_characters SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_event_log_characters SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_event_log_characters ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_event_log_characters_touch_insert AFTER INSERT ON game_event_log_characters FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_event_log_characters SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_event_log_characters_touch_update AFTER UPDATE ON game_event_log_characters FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_event_log_characters SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_events
UPDATE game_events SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_events SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_events SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_events ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_events_touch_insert AFTER INSERT ON game_events FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_events SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_events_touch_update AFTER UPDATE ON game_events FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_events SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_interactables
ALTER TABLE game_interactables ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_interactables SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_interactables SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_interactables ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_interactables_touch_insert AFTER INSERT ON game_interactables FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_interactables SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_interactables_touch_update AFTER UPDATE ON game_interactables FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_interactables SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_items
UPDATE game_items SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_items SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_items SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_items ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_items_touch_insert AFTER INSERT ON game_items FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_items SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_items_touch_update AFTER UPDATE ON game_items FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_items SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_journal
ALTER TABLE game_journal ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_journal SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_journal SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_journal SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_journal ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_journal_touch_insert AFTER INSERT ON game_journal FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_journal SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_journal_touch_update AFTER UPDATE ON game_journal FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_journal SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_location_connections
ALTER TABLE game_location_connections ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_location_connections SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_location_connections SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_location_connections ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_location_connections_touch_insert AFTER INSERT ON game_location_connections FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_location_connections SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_location_connections_touch_update AFTER UPDATE ON game_location_connections FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_location_connections SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_locations
ALTER TABLE game_locations ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_locations SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_locations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_locations ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_locations_touch_insert AFTER INSERT ON game_locations FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_locations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_locations_touch_update AFTER UPDATE ON game_locations FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_locations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_map_zones
UPDATE game_map_zones SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_map_zones SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_map_zones SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_map_zones ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_map_zones_touch_insert AFTER INSERT ON game_map_zones FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_map_zones SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_map_zones_touch_update AFTER UPDATE ON game_map_zones FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_map_zones SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_maps
UPDATE game_maps SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_maps SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_maps SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_maps ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_maps_touch_insert AFTER INSERT ON game_maps FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_maps SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_maps_touch_update AFTER UPDATE ON game_maps FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_maps SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_messages
UPDATE game_messages SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_messages SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_messages SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_messages ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_messages_touch_insert AFTER INSERT ON game_messages FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_messages_touch_update AFTER UPDATE ON game_messages FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_player_sessions
UPDATE game_player_sessions SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_player_sessions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_player_sessions ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_player_sessions_touch_insert AFTER INSERT ON game_player_sessions FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_player_sessions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_player_sessions_touch_update AFTER UPDATE ON game_player_sessions FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_player_sessions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_projects
UPDATE game_projects SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_projects SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_projects SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_projects ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_projects_touch_insert AFTER INSERT ON game_projects FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_projects SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_projects_touch_update AFTER UPDATE ON game_projects FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_projects SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_relationship_defs
ALTER TABLE game_relationship_defs ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_relationship_defs SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_relationship_defs SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_relationship_defs ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_relationship_defs_touch_insert AFTER INSERT ON game_relationship_defs FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_relationship_defs SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_relationship_defs_touch_update AFTER UPDATE ON game_relationship_defs FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_relationship_defs SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_relationship_snapshots
ALTER TABLE game_relationship_snapshots ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_relationship_snapshots SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_relationship_snapshots SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_relationship_snapshots SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_relationship_snapshots ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_relationship_snapshots_touch_insert AFTER INSERT ON game_relationship_snapshots FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_relationship_snapshots SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_relationship_snapshots_touch_update AFTER UPDATE ON game_relationship_snapshots FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_relationship_snapshots SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_saves
UPDATE game_saves SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_saves SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_saves ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_saves_touch_insert AFTER INSERT ON game_saves FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_saves SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_saves_touch_update AFTER UPDATE ON game_saves FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_saves SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_scene_conversations
ALTER TABLE game_scene_conversations ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_scene_conversations SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_scene_conversations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_scene_conversations ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_scene_conversations_touch_insert AFTER INSERT ON game_scene_conversations FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_scene_conversations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_scene_conversations_touch_update AFTER UPDATE ON game_scene_conversations FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_scene_conversations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_scenes
UPDATE game_scenes SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_scenes SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_scenes SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_scenes ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_scenes_touch_insert AFTER INSERT ON game_scenes FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_scenes SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_scenes_touch_update AFTER UPDATE ON game_scenes FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_scenes SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_session_options
UPDATE game_session_options SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_session_options SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_session_options ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_session_options_touch_insert AFTER INSERT ON game_session_options FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_session_options SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_session_options_touch_update AFTER UPDATE ON game_session_options FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_session_options SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_state_defs
UPDATE game_state_defs SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_state_defs SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_state_defs SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_state_defs ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_state_defs_touch_insert AFTER INSERT ON game_state_defs FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_state_defs SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_state_defs_touch_update AFTER UPDATE ON game_state_defs FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_state_defs SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_story_plans
ALTER TABLE game_story_plans ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_story_plans SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_story_plans SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_story_plans ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_story_plans_touch_insert AFTER INSERT ON game_story_plans FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_story_plans SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_story_plans_touch_update AFTER UPDATE ON game_story_plans FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_story_plans SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_time_periods
ALTER TABLE game_time_periods ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_time_periods SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_time_periods SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_time_periods ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_time_periods_touch_insert AFTER INSERT ON game_time_periods FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_time_periods SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_time_periods_touch_update AFTER UPDATE ON game_time_periods FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_time_periods SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_weather_pool_entries
ALTER TABLE game_weather_pool_entries ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_weather_pool_entries SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_weather_pool_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_weather_pool_entries ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_weather_pool_entries_touch_insert AFTER INSERT ON game_weather_pool_entries FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_weather_pool_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_weather_pool_entries_touch_update AFTER UPDATE ON game_weather_pool_entries FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_weather_pool_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_weather_pools
UPDATE game_weather_pools SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_weather_pools SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_weather_pools SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_weather_pools ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_weather_pools_touch_insert AFTER INSERT ON game_weather_pools FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_weather_pools SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_weather_pools_touch_update AFTER UPDATE ON game_weather_pools FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_weather_pools SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_weather_transitions
ALTER TABLE game_weather_transitions ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_weather_transitions SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_weather_transitions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_weather_transitions ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_weather_transitions_touch_insert AFTER INSERT ON game_weather_transitions FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_weather_transitions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_weather_transitions_touch_update AFTER UPDATE ON game_weather_transitions FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_weather_transitions SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_weather_types
UPDATE game_weather_types SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE game_weather_types SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_weather_types SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_weather_types ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_weather_types_touch_insert AFTER INSERT ON game_weather_types FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_weather_types SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_weather_types_touch_update AFTER UPDATE ON game_weather_types FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_weather_types SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- game_worldinfo
ALTER TABLE game_worldinfo ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE game_worldinfo SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE game_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE game_worldinfo ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER game_worldinfo_touch_insert AFTER INSERT ON game_worldinfo FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE game_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER game_worldinfo_touch_update AFTER UPDATE ON game_worldinfo FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE game_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- memory_settings
UPDATE memory_settings SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE memory_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE memory_settings ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER memory_settings_touch_insert AFTER INSERT ON memory_settings FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE memory_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER memory_settings_touch_update AFTER UPDATE ON memory_settings FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE memory_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- message_bookmarks
ALTER TABLE message_bookmarks ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE message_bookmarks SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE message_bookmarks SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE message_bookmarks SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE message_bookmarks ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER message_bookmarks_touch_insert AFTER INSERT ON message_bookmarks FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE message_bookmarks SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER message_bookmarks_touch_update AFTER UPDATE ON message_bookmarks FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE message_bookmarks SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- message_collections
ALTER TABLE message_collections ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE message_collections SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE message_collections SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE message_collections SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE message_collections ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER message_collections_touch_insert AFTER INSERT ON message_collections FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE message_collections SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER message_collections_touch_update AFTER UPDATE ON message_collections FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE message_collections SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- message_tag_relations
ALTER TABLE message_tag_relations ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE message_tag_relations SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE message_tag_relations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE message_tag_relations ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER message_tag_relations_touch_insert AFTER INSERT ON message_tag_relations FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE message_tag_relations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER message_tag_relations_touch_update AFTER UPDATE ON message_tag_relations FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE message_tag_relations SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- message_tags
ALTER TABLE message_tags ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE message_tags SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE message_tags SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE message_tags ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER message_tags_touch_insert AFTER INSERT ON message_tags FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE message_tags SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER message_tags_touch_update AFTER UPDATE ON message_tags FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE message_tags SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- messages
UPDATE messages SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE messages SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE messages SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE messages ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER messages_touch_insert AFTER INSERT ON messages FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER messages_touch_update AFTER UPDATE ON messages FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE messages SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- model_prices
UPDATE model_prices SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE model_prices SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE model_prices ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER model_prices_touch_insert AFTER INSERT ON model_prices FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE model_prices SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER model_prices_touch_update AFTER UPDATE ON model_prices FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE model_prices SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- persona_worldinfo
ALTER TABLE persona_worldinfo ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE persona_worldinfo SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE persona_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE persona_worldinfo ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER persona_worldinfo_touch_insert AFTER INSERT ON persona_worldinfo FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE persona_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER persona_worldinfo_touch_update AFTER UPDATE ON persona_worldinfo FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE persona_worldinfo SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- personas
UPDATE personas SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE personas SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE personas SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE personas ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER personas_touch_insert AFTER INSERT ON personas FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE personas SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER personas_touch_update AFTER UPDATE ON personas FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE personas SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- phone_links
ALTER TABLE phone_links ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE phone_links SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE phone_links SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE phone_links SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE phone_links ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER phone_links_touch_insert AFTER INSERT ON phone_links FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE phone_links SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER phone_links_touch_update AFTER UPDATE ON phone_links FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE phone_links SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- proactive_settings
UPDATE proactive_settings SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE proactive_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE proactive_settings ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER proactive_settings_touch_insert AFTER INSERT ON proactive_settings FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE proactive_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER proactive_settings_touch_update AFTER UPDATE ON proactive_settings FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE proactive_settings SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- prompt_blocks
UPDATE prompt_blocks SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE prompt_blocks SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE prompt_blocks SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE prompt_blocks ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER prompt_blocks_touch_insert AFTER INSERT ON prompt_blocks FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE prompt_blocks SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER prompt_blocks_touch_update AFTER UPDATE ON prompt_blocks FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE prompt_blocks SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- prompt_presets
UPDATE prompt_presets SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE prompt_presets SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE prompt_presets SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE prompt_presets ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER prompt_presets_touch_insert AFTER INSERT ON prompt_presets FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE prompt_presets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER prompt_presets_touch_update AFTER UPDATE ON prompt_presets FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE prompt_presets SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- regex_scripts
UPDATE regex_scripts SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE regex_scripts SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE regex_scripts SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE regex_scripts ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER regex_scripts_touch_insert AFTER INSERT ON regex_scripts FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE regex_scripts SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER regex_scripts_touch_update AFTER UPDATE ON regex_scripts FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE regex_scripts SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- worldinfo_books
UPDATE worldinfo_books SET created_at = created_at * 1000 WHERE created_at IS NOT NULL AND created_at > 0 AND created_at < 100000000000;
UPDATE worldinfo_books SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE worldinfo_books SET updated_at = COALESCE(created_at, CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE worldinfo_books ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER worldinfo_books_touch_insert AFTER INSERT ON worldinfo_books FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE worldinfo_books SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER worldinfo_books_touch_update AFTER UPDATE ON worldinfo_books FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE worldinfo_books SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;

-- worldinfo_entries
ALTER TABLE worldinfo_entries ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
UPDATE worldinfo_entries SET updated_at = updated_at * 1000 WHERE updated_at IS NOT NULL AND updated_at > 0 AND updated_at < 100000000000;
UPDATE worldinfo_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE updated_at IS NULL OR updated_at = 0;
ALTER TABLE worldinfo_entries ADD COLUMN deleted_at INTEGER;
CREATE TRIGGER worldinfo_entries_touch_insert AFTER INSERT ON worldinfo_entries FOR EACH ROW WHEN NEW.updated_at IS NULL OR NEW.updated_at = 0 BEGIN UPDATE worldinfo_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
CREATE TRIGGER worldinfo_entries_touch_update AFTER UPDATE ON worldinfo_entries FOR EACH ROW WHEN NEW.updated_at IS OLD.updated_at BEGIN UPDATE worldinfo_entries SET updated_at = CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER) WHERE rowid = NEW.rowid; END;
