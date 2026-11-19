-- =========================================================
-- 채팅 DB
-- =========================================================
CREATE TABLE IF NOT EXISTS conv_settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  default_persona_id TEXT REFERENCES personas(id) ON DELETE SET NULL,
  default_prompt_preset_id TEXT REFERENCES prompt_presets(id) ON DELETE SET NULL,
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
  default_prompt_preset_id TEXT REFERENCES prompt_presets(id) ON DELETE SET NULL,
  favorite INTEGER DEFAULT 0,
  creator_notes TEXT,
  -- 캐릭터 페이지 위쪽 커버(배너) 이미지. 앱 데이터 characters/covers/에 복사한 파일 경로. 없으면 기본 그라데이션
  cover TEXT,
  -- 커버 이미지에서 보일 세로 위치 (0 = 맨 위, 100 = 맨 아래, background-position-y %)
  -- cover_position으로 변경 필요 (x/y 값 및 확대를 했는지도 저장하려면)
  cover_y REAL NOT NULL DEFAULT 50,
  created_at INTEGER,
  updated_at INTEGER
);

-- ---------------------------------------------------------
-- 채팅용 캐릭터 이미지
-- ---------------------------------------------------------
-- - 라이브러리 캐릭터(characters)에 붙는 이미지. 게임 에셋(game_assets)과는 별개.
-- - chat/phone 대화에서 모델에게 키 목록만 보내고, 모델은 <cg>키</cg>로 고른다.
-- - 파일은 앱 데이터 폴더 기준 상대 경로로 저장 (내보내기/가져오기 시 그대로 옮길 수 있게)
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS character_images (
  id TEXT PRIMARY KEY,
  character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  -- 모델이 <cg>키</cg>로 쓰는 이름 (캐릭터 안에서 대소문자 무시 유일)
  key TEXT NOT NULL,
  -- 앱 데이터 폴더 기준 상대 경로
  file_path TEXT NOT NULL,
  thumb_path TEXT,
  -- 끄면 모델에게 보내지 않음
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
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
  comment TEXT NOT NULL DEFAULT '',
  enabled INTEGER DEFAULT 1,
  priority INTEGER DEFAULT 0,
  constant INTEGER DEFAULT 0,
  updated_at INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS persona_worldinfo (
  persona_id TEXT REFERENCES personas(id) ON DELETE CASCADE,
  book_id TEXT REFERENCES worldinfo_books(id) ON DELETE CASCADE,
  updated_at INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (persona_id, book_id)
);

CREATE TABLE IF NOT EXISTS character_worldinfo (
  character_id TEXT REFERENCES characters(id) ON DELETE CASCADE,
  book_id TEXT REFERENCES worldinfo_books(id) ON DELETE CASCADE,
  updated_at INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (character_id, book_id)
);

-- ---------------------------------------------------------
--  대화
-- ---------------------------------------------------------
-- conversation_folders: 캐릭터별 대화 폴더 (대화 상세 왼쪽 목록)
CREATE TABLE IF NOT EXISTS conversation_folders (
  id TEXT PRIMARY KEY,
  character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS conversations (
  id TEXT PRIMARY KEY,
  title TEXT,
  character_id TEXT REFERENCES characters(id) ON DELETE SET NULL,
  persona_id TEXT REFERENCES personas(id) ON DELETE SET NULL,
  prompt_preset_id TEXT REFERENCES prompt_presets(id) ON DELETE SET NULL,
  world_info_ids TEXT,
  mode TEXT,
  meta TEXT,
  -- 대화별 모델 (ADR-020, 1-H-2)
  -- NULL = 이 대화의 API 프로필의 기본 모델을 따라간다 (프로필의 기본 모델을 바꾸면 같이 바뀜).
  -- 사용자가 대화 설정에서 모델을 고르면 그 이름이 여기 저장된다. 대화의 API를 바꾸면 NULL로 돌아간다.
  model TEXT,
  pinned INTEGER DEFAULT 0,
  images_enabled INTEGER NOT NULL DEFAULT 0,
  -- folder_id / api_profile_id: 대화별 폴더·API (프리셋은 prompt_preset_id가 이미 있음)
  -- 비어 있으면 앱 설정의 선택값을 쓴다
  folder_id TEXT REFERENCES conversation_folders(id) ON DELETE SET NULL,
  api_profile_id TEXT REFERENCES api_profiles(id) ON DELETE SET NULL,
  created_at INTEGER,
  updated_at INTEGER
);

-- ---------------------------------------------------------
--  메시지 테이블
-- ---------------------------------------------------------
-- UI표시용 테이블 / raw 답변은 task.sql을 참조 
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL,
  role TEXT NOT NULL,
  name TEXT,
  content TEXT NOT NULL,
  character_id TEXT REFERENCES characters(id) ON DELETE SET NULL,
  meta TEXT,
  created_at INTEGER,
  updated_at INTEGER,
  UNIQUE(conversation_id, seq)
);

-- message_prompts: AI 응답을 만들 때 실제로 보낸 프롬프트 ("보낸 컨텍스트 보기")
CREATE TABLE IF NOT EXISTS message_prompts (
  message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
  prompt TEXT NOT NULL,
  created_at INTEGER NOT NULL
);


-- 메시지 즐겨찾기
CREATE TABLE IF NOT EXISTS message_bookmarks (
  id TEXT PRIMARY KEY,
  message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  note TEXT,
  created_at INTEGER NOT NULL
);

-- 태그
CREATE TABLE IF NOT EXISTS message_tags (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  updated_at INTEGER NOT NULL DEFAULT 0
);

-- 메시지와 태그 연결
CREATE TABLE IF NOT EXISTS message_tag_relations (
  message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  tag_id TEXT NOT NULL,
  updated_at INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (message_id, tag_id)
);

-- 사용자 컬렉션
CREATE TABLE IF NOT EXISTS message_collections (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL DEFAULT 0
);

-- 컬렉션에 포함된 메시지
CREATE TABLE IF NOT EXISTS collection_messages (
  collection_id TEXT NOT NULL REFERENCES message_collections(id) ON DELETE CASCADE,
  message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  sort_order INTEGER NOT NULL,
  updated_at INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (collection_id, message_id)
);

-- =========================================================
-- 인덱스
-- =========================================================
CREATE INDEX IF NOT EXISTS idx_conversation_folders_char ON conversation_folders(character_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id, created_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_character_images_key ON character_images(character_id, key COLLATE NOCASE);

-- =========================================================
-- 뷰
-- =========================================================
CREATE VIEW IF NOT EXISTS v_conversations_with_count AS SELECT c.*, COUNT(m.id) AS message_count FROM conversations c LEFT JOIN messages m ON c.id = m.conversation_id GROUP BY c.id;

-- =========================================================
-- 기본 설정
-- =========================================================
INSERT OR IGNORE INTO personas (id,name,avatar,description,tags,color,is_system_default,created_at,updated_at) VALUES ("default","user",NULL,NULL,NULL,NULL,0,strftime('%s','now'),strftime('%s','now'));
INSERT OR IGNORE INTO conv_settings (id, default_persona_id, updated_at) VALUES (1, "default", strftime('%s','now') * 1000);