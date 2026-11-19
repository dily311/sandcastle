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

-- audio_assets (assets_id)
CREATE TRIGGER audio_assets_tombstone AFTER DELETE ON audio_assets FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('audio_assets', json_array(OLD.assets_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER audio_assets_untombstone AFTER INSERT ON audio_assets FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'audio_assets' AND key = json_array(NEW.assets_id); END;

-- background_assets (assets_id)
CREATE TRIGGER background_assets_tombstone AFTER DELETE ON background_assets FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('background_assets', json_array(OLD.assets_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER background_assets_untombstone AFTER INSERT ON background_assets FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'background_assets' AND key = json_array(NEW.assets_id); END;

-- budget_limits (api_profile_id)
CREATE TRIGGER budget_limits_tombstone AFTER DELETE ON budget_limits FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('budget_limits', json_array(OLD.api_profile_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER budget_limits_untombstone AFTER INSERT ON budget_limits FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'budget_limits' AND key = json_array(NEW.api_profile_id); END;

-- cg_assets (assets_id)
CREATE TRIGGER cg_assets_tombstone AFTER DELETE ON cg_assets FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('cg_assets', json_array(OLD.assets_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER cg_assets_untombstone AFTER INSERT ON cg_assets FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'cg_assets' AND key = json_array(NEW.assets_id); END;

-- character_assets (assets_id)
CREATE TRIGGER character_assets_tombstone AFTER DELETE ON character_assets FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('character_assets', json_array(OLD.assets_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER character_assets_untombstone AFTER INSERT ON character_assets FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'character_assets' AND key = json_array(NEW.assets_id); END;

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

-- game_assets (id)
CREATE TRIGGER game_assets_tombstone AFTER DELETE ON game_assets FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_assets', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_assets_untombstone AFTER INSERT ON game_assets FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_assets' AND key = json_array(NEW.id); END;

-- game_branches (id)
CREATE TRIGGER game_branches_tombstone AFTER DELETE ON game_branches FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_branches', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_branches_untombstone AFTER INSERT ON game_branches FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_branches' AND key = json_array(NEW.id); END;

-- game_character_spawn_locations (id)
CREATE TRIGGER game_character_spawn_locations_tombstone AFTER DELETE ON game_character_spawn_locations FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_character_spawn_locations', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_character_spawn_locations_untombstone AFTER INSERT ON game_character_spawn_locations FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_character_spawn_locations' AND key = json_array(NEW.id); END;

-- game_character_state_snapshots (session_id, seq, game_character_id)
CREATE TRIGGER game_character_state_snapshots_tombstone AFTER DELETE ON game_character_state_snapshots FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_character_state_snapshots', json_array(OLD.session_id, OLD.seq, OLD.game_character_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_character_state_snapshots_untombstone AFTER INSERT ON game_character_state_snapshots FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_character_state_snapshots' AND key = json_array(NEW.session_id, NEW.seq, NEW.game_character_id); END;

-- game_character_worldinfo (game_character_id, book_id)
CREATE TRIGGER game_character_worldinfo_tombstone AFTER DELETE ON game_character_worldinfo FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_character_worldinfo', json_array(OLD.game_character_id, OLD.book_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_character_worldinfo_untombstone AFTER INSERT ON game_character_worldinfo FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_character_worldinfo' AND key = json_array(NEW.game_character_id, NEW.book_id); END;

-- game_characters (id)
CREATE TRIGGER game_characters_tombstone AFTER DELETE ON game_characters FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_characters', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_characters_untombstone AFTER INSERT ON game_characters FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_characters' AND key = json_array(NEW.id); END;

-- game_choices (id)
CREATE TRIGGER game_choices_tombstone AFTER DELETE ON game_choices FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_choices', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_choices_untombstone AFTER INSERT ON game_choices FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_choices' AND key = json_array(NEW.id); END;

-- game_conversation_participants (conversation_id, game_character_id)
CREATE TRIGGER game_conversation_participants_tombstone AFTER DELETE ON game_conversation_participants FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_conversation_participants', json_array(OLD.conversation_id, OLD.game_character_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_conversation_participants_untombstone AFTER INSERT ON game_conversation_participants FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_conversation_participants' AND key = json_array(NEW.conversation_id, NEW.game_character_id); END;

-- game_conversations (id)
CREATE TRIGGER game_conversations_tombstone AFTER DELETE ON game_conversations FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_conversations', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_conversations_untombstone AFTER INSERT ON game_conversations FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_conversations' AND key = json_array(NEW.id); END;

-- game_encounter_entries (id)
CREATE TRIGGER game_encounter_entries_tombstone AFTER DELETE ON game_encounter_entries FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_encounter_entries', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_encounter_entries_untombstone AFTER INSERT ON game_encounter_entries FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_encounter_entries' AND key = json_array(NEW.id); END;

-- game_encounter_tables (id)
CREATE TRIGGER game_encounter_tables_tombstone AFTER DELETE ON game_encounter_tables FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_encounter_tables', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_encounter_tables_untombstone AFTER INSERT ON game_encounter_tables FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_encounter_tables' AND key = json_array(NEW.id); END;

-- game_endings (id)
CREATE TRIGGER game_endings_tombstone AFTER DELETE ON game_endings FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_endings', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_endings_untombstone AFTER INSERT ON game_endings FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_endings' AND key = json_array(NEW.id); END;

-- game_event_character_pool (event_id, game_character_id)
CREATE TRIGGER game_event_character_pool_tombstone AFTER DELETE ON game_event_character_pool FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_event_character_pool', json_array(OLD.event_id, OLD.game_character_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_event_character_pool_untombstone AFTER INSERT ON game_event_character_pool FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_event_character_pool' AND key = json_array(NEW.event_id, NEW.game_character_id); END;

-- game_event_conditions (id)
CREATE TRIGGER game_event_conditions_tombstone AFTER DELETE ON game_event_conditions FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_event_conditions', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_event_conditions_untombstone AFTER INSERT ON game_event_conditions FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_event_conditions' AND key = json_array(NEW.id); END;

-- game_event_log (session_id, seq)
CREATE TRIGGER game_event_log_tombstone AFTER DELETE ON game_event_log FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_event_log', json_array(OLD.session_id, OLD.seq), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_event_log_untombstone AFTER INSERT ON game_event_log FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_event_log' AND key = json_array(NEW.session_id, NEW.seq); END;

-- game_event_log_characters (session_id, seq, game_character_id)
CREATE TRIGGER game_event_log_characters_tombstone AFTER DELETE ON game_event_log_characters FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_event_log_characters', json_array(OLD.session_id, OLD.seq, OLD.game_character_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_event_log_characters_untombstone AFTER INSERT ON game_event_log_characters FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_event_log_characters' AND key = json_array(NEW.session_id, NEW.seq, NEW.game_character_id); END;

-- game_events (id)
CREATE TRIGGER game_events_tombstone AFTER DELETE ON game_events FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_events', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_events_untombstone AFTER INSERT ON game_events FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_events' AND key = json_array(NEW.id); END;

-- game_interactables (id)
CREATE TRIGGER game_interactables_tombstone AFTER DELETE ON game_interactables FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_interactables', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_interactables_untombstone AFTER INSERT ON game_interactables FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_interactables' AND key = json_array(NEW.id); END;

-- game_items (id)
CREATE TRIGGER game_items_tombstone AFTER DELETE ON game_items FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_items', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_items_untombstone AFTER INSERT ON game_items FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_items' AND key = json_array(NEW.id); END;

-- game_journal (session_id, seq)
CREATE TRIGGER game_journal_tombstone AFTER DELETE ON game_journal FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_journal', json_array(OLD.session_id, OLD.seq), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_journal_untombstone AFTER INSERT ON game_journal FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_journal' AND key = json_array(NEW.session_id, NEW.seq); END;

-- game_location_connections (from_id, to_id)
CREATE TRIGGER game_location_connections_tombstone AFTER DELETE ON game_location_connections FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_location_connections', json_array(OLD.from_id, OLD.to_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_location_connections_untombstone AFTER INSERT ON game_location_connections FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_location_connections' AND key = json_array(NEW.from_id, NEW.to_id); END;

-- game_locations (id)
CREATE TRIGGER game_locations_tombstone AFTER DELETE ON game_locations FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_locations', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_locations_untombstone AFTER INSERT ON game_locations FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_locations' AND key = json_array(NEW.id); END;

-- game_map_zones (id)
CREATE TRIGGER game_map_zones_tombstone AFTER DELETE ON game_map_zones FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_map_zones', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_map_zones_untombstone AFTER INSERT ON game_map_zones FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_map_zones' AND key = json_array(NEW.id); END;

-- game_maps (id)
CREATE TRIGGER game_maps_tombstone AFTER DELETE ON game_maps FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_maps', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_maps_untombstone AFTER INSERT ON game_maps FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_maps' AND key = json_array(NEW.id); END;

-- game_messages (id)
CREATE TRIGGER game_messages_tombstone AFTER DELETE ON game_messages FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_messages', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_messages_untombstone AFTER INSERT ON game_messages FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_messages' AND key = json_array(NEW.id); END;

-- game_player_sessions (id)
CREATE TRIGGER game_player_sessions_tombstone AFTER DELETE ON game_player_sessions FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_player_sessions', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_player_sessions_untombstone AFTER INSERT ON game_player_sessions FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_player_sessions' AND key = json_array(NEW.id); END;

-- game_projects (id)
CREATE TRIGGER game_projects_tombstone AFTER DELETE ON game_projects FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_projects', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_projects_untombstone AFTER INSERT ON game_projects FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_projects' AND key = json_array(NEW.id); END;

-- game_relationship_defs (id)
CREATE TRIGGER game_relationship_defs_tombstone AFTER DELETE ON game_relationship_defs FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_relationship_defs', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_relationship_defs_untombstone AFTER INSERT ON game_relationship_defs FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_relationship_defs' AND key = json_array(NEW.id); END;

-- game_relationship_snapshots (session_id, seq)
CREATE TRIGGER game_relationship_snapshots_tombstone AFTER DELETE ON game_relationship_snapshots FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_relationship_snapshots', json_array(OLD.session_id, OLD.seq), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_relationship_snapshots_untombstone AFTER INSERT ON game_relationship_snapshots FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_relationship_snapshots' AND key = json_array(NEW.session_id, NEW.seq); END;

-- game_saves (session_id, slot_number)
CREATE TRIGGER game_saves_tombstone AFTER DELETE ON game_saves FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_saves', json_array(OLD.session_id, OLD.slot_number), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_saves_untombstone AFTER INSERT ON game_saves FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_saves' AND key = json_array(NEW.session_id, NEW.slot_number); END;

-- game_scene_conversations (scene_id, conversation_id)
CREATE TRIGGER game_scene_conversations_tombstone AFTER DELETE ON game_scene_conversations FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_scene_conversations', json_array(OLD.scene_id, OLD.conversation_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_scene_conversations_untombstone AFTER INSERT ON game_scene_conversations FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_scene_conversations' AND key = json_array(NEW.scene_id, NEW.conversation_id); END;

-- game_scenes (id)
CREATE TRIGGER game_scenes_tombstone AFTER DELETE ON game_scenes FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_scenes', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_scenes_untombstone AFTER INSERT ON game_scenes FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_scenes' AND key = json_array(NEW.id); END;

-- game_session_options (session_id)
CREATE TRIGGER game_session_options_tombstone AFTER DELETE ON game_session_options FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_session_options', json_array(OLD.session_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_session_options_untombstone AFTER INSERT ON game_session_options FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_session_options' AND key = json_array(NEW.session_id); END;

-- game_state_defs (id)
CREATE TRIGGER game_state_defs_tombstone AFTER DELETE ON game_state_defs FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_state_defs', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_state_defs_untombstone AFTER INSERT ON game_state_defs FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_state_defs' AND key = json_array(NEW.id); END;

-- game_story_plans (session_id, seq)
CREATE TRIGGER game_story_plans_tombstone AFTER DELETE ON game_story_plans FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_story_plans', json_array(OLD.session_id, OLD.seq), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_story_plans_untombstone AFTER INSERT ON game_story_plans FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_story_plans' AND key = json_array(NEW.session_id, NEW.seq); END;

-- game_time_periods (id)
CREATE TRIGGER game_time_periods_tombstone AFTER DELETE ON game_time_periods FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_time_periods', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_time_periods_untombstone AFTER INSERT ON game_time_periods FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_time_periods' AND key = json_array(NEW.id); END;

-- game_weather_pool_entries (pool_id, weather_id)
CREATE TRIGGER game_weather_pool_entries_tombstone AFTER DELETE ON game_weather_pool_entries FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_weather_pool_entries', json_array(OLD.pool_id, OLD.weather_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_weather_pool_entries_untombstone AFTER INSERT ON game_weather_pool_entries FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_weather_pool_entries' AND key = json_array(NEW.pool_id, NEW.weather_id); END;

-- game_weather_pools (id)
CREATE TRIGGER game_weather_pools_tombstone AFTER DELETE ON game_weather_pools FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_weather_pools', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_weather_pools_untombstone AFTER INSERT ON game_weather_pools FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_weather_pools' AND key = json_array(NEW.id); END;

-- game_weather_transitions (from_weather_id, to_weather_id, map_id)
CREATE TRIGGER game_weather_transitions_tombstone AFTER DELETE ON game_weather_transitions FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_weather_transitions', json_array(OLD.from_weather_id, OLD.to_weather_id, OLD.map_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_weather_transitions_untombstone AFTER INSERT ON game_weather_transitions FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_weather_transitions' AND key = json_array(NEW.from_weather_id, NEW.to_weather_id, NEW.map_id); END;

-- game_weather_types (id)
CREATE TRIGGER game_weather_types_tombstone AFTER DELETE ON game_weather_types FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_weather_types', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_weather_types_untombstone AFTER INSERT ON game_weather_types FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_weather_types' AND key = json_array(NEW.id); END;

-- game_worldinfo (game_id, book_id)
CREATE TRIGGER game_worldinfo_tombstone AFTER DELETE ON game_worldinfo FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('game_worldinfo', json_array(OLD.game_id, OLD.book_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER game_worldinfo_untombstone AFTER INSERT ON game_worldinfo FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'game_worldinfo' AND key = json_array(NEW.game_id, NEW.book_id); END;

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

-- phone_links (owner_kind, owner_id, contact_kind, contact_id)
CREATE TRIGGER phone_links_tombstone AFTER DELETE ON phone_links FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('phone_links', json_array(OLD.owner_kind, OLD.owner_id, OLD.contact_kind, OLD.contact_id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER phone_links_untombstone AFTER INSERT ON phone_links FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'phone_links' AND key = json_array(NEW.owner_kind, NEW.owner_id, NEW.contact_kind, NEW.contact_id); END;

-- proactive_settings (id)
CREATE TRIGGER proactive_settings_tombstone AFTER DELETE ON proactive_settings FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('proactive_settings', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER proactive_settings_untombstone AFTER INSERT ON proactive_settings FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'proactive_settings' AND key = json_array(NEW.id); END;

-- prompt_blocks (id)
CREATE TRIGGER prompt_blocks_tombstone AFTER DELETE ON prompt_blocks FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('prompt_blocks', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER prompt_blocks_untombstone AFTER INSERT ON prompt_blocks FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'prompt_blocks' AND key = json_array(NEW.id); END;

-- prompt_presets (id)
CREATE TRIGGER prompt_presets_tombstone AFTER DELETE ON prompt_presets FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('prompt_presets', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER prompt_presets_untombstone AFTER INSERT ON prompt_presets FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'prompt_presets' AND key = json_array(NEW.id); END;

-- regex_scripts (id)
CREATE TRIGGER regex_scripts_tombstone AFTER DELETE ON regex_scripts FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('regex_scripts', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER regex_scripts_untombstone AFTER INSERT ON regex_scripts FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'regex_scripts' AND key = json_array(NEW.id); END;

-- worldinfo_books (id)
CREATE TRIGGER worldinfo_books_tombstone AFTER DELETE ON worldinfo_books FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('worldinfo_books', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER worldinfo_books_untombstone AFTER INSERT ON worldinfo_books FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'worldinfo_books' AND key = json_array(NEW.id); END;

-- worldinfo_entries (id)
CREATE TRIGGER worldinfo_entries_tombstone AFTER DELETE ON worldinfo_entries FOR EACH ROW BEGIN INSERT OR REPLACE INTO sync_tombstones (tbl, key, deleted_at) VALUES ('worldinfo_entries', json_array(OLD.id), CAST((julianday('now') - 2440587.5) * 86400000 AS INTEGER)); END;
CREATE TRIGGER worldinfo_entries_untombstone AFTER INSERT ON worldinfo_entries FOR EACH ROW BEGIN DELETE FROM sync_tombstones WHERE tbl = 'worldinfo_entries' AND key = json_array(NEW.id); END;
