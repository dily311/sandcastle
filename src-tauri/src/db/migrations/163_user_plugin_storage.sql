-- 커뮤니티 플러그인의 작은 저장소 (CLAUDE.md 6-D-2, ADR-012: `storage` 권한).
-- 플러그인마다 키-값(값은 JSON 글자). 플러그인을 지우면(휴지통) 같이 떠지고, 되살리면 같이 돌아온다(FK CASCADE).
-- 기기마다 따로 (동기화 안 함 — change_tracking::UNSYNCED_TABLES).
CREATE TABLE user_plugin_storage (
  plugin_id TEXT NOT NULL REFERENCES user_plugins(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  value TEXT NOT NULL,
  updated_at INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (plugin_id, key)
);
