
-- =========================================================
-- 채팅 DB
-- =========================================================

CREATE TABLE IF NOT EXISTS conv_settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  default_persona_id TEXT REFERENCES personas(id) ON DELETE SET NULL,
  updated_at INTEGER
);

-- ---------------------------------------------------------
--  캐릭터 & 페르소나
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS personas (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  avatar TEXT,
  description TEXT,
  tags TEXT,
  color TEXT,
  is_system_default INTEGER DEFAULT 0,
  created_at INTEGER,
  updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS characters (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  avatar TEXT,
  description TEXT,
  alternate_descriptions TEXT,
  first_message TEXT,
  alternate_first_message TEXT,
  example_dialogue TEXT,
  system_prompt_override TEXT,
  color TEXT,
  tags TEXT,
  category TEXT,
  default_persona_id TEXT REFERENCES personas(id) ON DELETE SET NULL,
  favorite INTEGER DEFAULT 0,
  created_at INTEGER,
  updated_at INTEGER
);

-- ---------------------------------------------------------
--  월드 인포
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS worldinfo_books (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  created_at INTEGER,
  updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS worldinfo_entries (
  id TEXT PRIMARY KEY,
  book_id TEXT NOT NULL REFERENCES worldinfo_books(id) ON DELETE CASCADE,
  keys TEXT,
  secondary_keys TEXT,
  content TEXT,
  enabled INTEGER DEFAULT 1,
  priority INTEGER DEFAULT 0,
  constant INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS persona_worldinfo (
  persona_id TEXT REFERENCES personas(id) ON DELETE CASCADE,
  book_id TEXT REFERENCES worldinfo_books(id) ON DELETE CASCADE,
  PRIMARY KEY (persona_id, book_id)
);

CREATE TABLE IF NOT EXISTS character_worldinfo (
  character_id TEXT REFERENCES characters(id) ON DELETE CASCADE,
  book_id TEXT REFERENCES worldinfo_books(id) ON DELETE CASCADE,
  PRIMARY KEY (character_id, book_id)
);

-- ---------------------------------------------------------
--  대화
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS conversations (
  id TEXT PRIMARY KEY,
  title TEXT,
  character_id TEXT REFERENCES characters(id) ON DELETE SET NULL,
  persona_id TEXT REFERENCES personas(id) ON DELETE SET NULL,
  world_info_ids TEXT,
  mode TEXT,
  meta TEXT,
  pinned INTEGER DEFAULT 0,
  created_at INTEGER,
  updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  role TEXT NOT NULL,
  name TEXT,
  content TEXT NOT NULL,
  character_id TEXT REFERENCES characters(id) ON DELETE SET NULL,
  meta TEXT,
  created_at INTEGER,
  updated_at INTEGER
);

-- =========================================================
-- 인덱스
-- =========================================================
CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id, created_at);
CREATE VIEW IF NOT EXISTS v_conversations_with_count AS SELECT c.*, COUNT(m.id) AS message_count FROM conversations c LEFT JOIN messages m ON c.id = m.conversation_id GROUP BY c.id;


-- =========================================================
-- 기본 설정
-- =========================================================
INSERT OR IGNORE INTO personas (id,name,avatar,description,tags,color,is_system_default,created_at,updated_at) VALUES ("default","user",NULL,NULL,NULL,NULL,0,strftime('%s','now'),strftime('%s','now'));
INSERT OR IGNORE INTO conv_settings (id, default_persona_id, updated_at) VALUES (1, "default", strftime('%s','now') * 1000);
