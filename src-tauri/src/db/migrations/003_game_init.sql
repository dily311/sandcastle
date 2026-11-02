-- =========================================================
-- 게임 모드
-- =========================================================

-- ---------------------------------------------------------
--  게임 프로젝트
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_projects (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT,
  -- 게임 프로젝트의 설정값
  template_id TEXT NOT NULL,        -- 사용한 템플릿 식별자
  version TEXT NOT NULL DEFAULT '1.0',
  api_version TEXT NOT NULL DEFAULT '1.0',
  cover_image TEXT,
  custom_css TEXT,           -- 커스텀 css
  start_scene_key TEXT,
  start_map_id TEXT,
  start_location_id TEXT,
  settings_json TEXT,        -- 템플릿별 추가 설정
  created_at INTEGER,
  updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS game_worldinfo (
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  book_id TEXT NOT NULL REFERENCES worldinfo_books(id) ON DELETE CASCADE,

  PRIMARY KEY (game_id, book_id)
);

-- ---------------------------------------------------------
-- 게임 변수
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_state_defs ( -- game_variables
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  description TEXT,
  key TEXT NOT NULL,                -- 예: affection_yuna
  type TEXT NOT NULL CHECK (type IN ('number', 'boolean', 'string', 'array')),
  default_value TEXT,               -- JSON으로 직렬화된 기본값
  config_json TEXT,                 -- 설정 / AI 변수 사용 여부, 한번에 변수 최대/최소 증감 기록
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (game_id, key)
);

-- ---------------------------------------------------------
-- 게임 캐릭터
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_characters (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  character_id TEXT REFERENCES characters(id) ON DELETE SET NULL, -- NULL이면 게임 전욜 캐릭터
  name TEXT NOT NULL,
  avatar TEXT,
  description TEXT,
  first_message TEXT,
  example_dialogue TEXT,
  system_prompt_override TEXT,
  
  -- AI NPC 행동 프로필
  behavior_type TEXT,
  tags TEXT,

  -- 게임에서만 사용하는 설정
  settings_json TEXT,

  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS game_character_worldinfo (
  game_character_id TEXT NOT NULL REFERENCES game_characters(id) ON DELETE CASCADE,
  book_id TEXT NOT NULL REFERENCES worldinfo_books(id) ON DELETE CASCADE,

  PRIMARY KEY (game_character_id, book_id)
);

CREATE TABLE IF NOT EXISTS game_character_spawn_locations (
  id TEXT PRIMARY KEY,
  game_character_id TEXT NOT NULL REFERENCES game_characters(id) ON DELETE CASCADE,
  location_id TEXT NOT NULL REFERENCES game_locations(id) ON DELETE CASCADE,
  -- 다른 장소와 비교하는 출현 가중치
  spawn_weight INTEGER NOT NULL DEFAULT 1 CHECK (spawn_weight > 0),
  -- 시간대, 날씨, 플래그 등의 추가 출현 조건
  condition_json TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (game_character_id, location_id)
);

-- ---------------------------------------------------------
-- 게임 에셋
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_assets (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  name TEXT NOT NULL,
  category TEXT NOT NULL CHECK (category IN ('voice','sfx','bgm','character','cg','background','font','other')),
  kind TEXT NOT NULL CHECK (kind IN ('image','audio','font','other')),
  file_path TEXT NOT NULL,
  settings_json TEXT,                       -- 볼륨, 루프, 페이드
  created_at INTEGER NOT NULL, 
  updated_at INTEGER NOT NULL,
  UNIQUE (game_id, key)
);

-- 오디오 에셋
CREATE TABLE IF NOT EXISTS audio_assets (                  -- voice / sfx / bgm
  assets_id TEXT PRIMARY KEY REFERENCES game_assets(id) ON DELETE CASCADE,
  tag TEXT,                                  -- sfx: 소리 그룹 이름, bgm: 분위기 id
  game_character_id TEXT REFERENCES game_characters(id) ON DELETE SET NULL,  -- voice의 화자
  transcript TEXT                            -- voice의 대사 텍스트
);

-- 이미지 에셋
CREATE TABLE IF NOT EXISTS character_assets (
  assets_id TEXT PRIMARY KEY REFERENCES game_assets(id) ON DELETE CASCADE,
  game_character_id TEXT REFERENCES game_characters(id) ON DELETE SET NULL,
  usage TEXT NOT NULL CHECK (usage IN ('standing','map')),
  emotion TEXT                              -- standing은 필수, map은 NULL 또는 variant
);

CREATE TABLE IF NOT EXISTS cg_assets (
  assets_id TEXT PRIMARY KEY REFERENCES game_assets(id) ON DELETE CASCADE,
  game_character_id TEXT REFERENCES game_characters(id) ON DELETE SET NULL,
  description TEXT,                          -- AI에게 보낼 설명
  prompt TEXT, 
  negative_prompt TEXT, 
  seed INTEGER, 
  generation_params TEXT,
  ai_usable INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS background_assets (
  assets_id TEXT PRIMARY KEY REFERENCES game_assets(id) ON DELETE CASCADE,
  location_id TEXT REFERENCES game_locations(id) ON DELETE SET NULL,
  time_id TEXT REFERENCES game_time_periods(id) ON DELETE SET NULL,
  weather_id TEXT REFERENCES game_weather_types(id) ON DELETE SET NULL
);
-- ---------------------------------------------------------
-- 게임 맵
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_maps (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  danger_level INTEGER NOT NULL DEFAULT 0,
  default_weather_pool_id TEXT, -- 기본 날씨 pool
  settings_json TEXT,  -- 맵 표시 세팅
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS game_map_zones (
  id TEXT PRIMARY KEY,
  map_id TEXT NOT NULL REFERENCES game_maps(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  tags TEXT, -- 예시: ["forest", "dark", "dangerous"]
  settings_json TEXT,  -- 맵 표시 세팅
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS game_locations (
  id TEXT PRIMARY KEY,
  zone_id TEXT NOT NULL REFERENCES game_map_zones(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  description TEXT,
  settings_json TEXT  -- 맵 표시 세팅
);

CREATE TABLE IF NOT EXISTS game_location_connections (
  from_id TEXT NOT NULL REFERENCES game_locations(id) ON DELETE CASCADE,
  to_id   TEXT NOT NULL REFERENCES game_locations(id) ON DELETE CASCADE,
  travel_seconds INTEGER NOT NULL,
  condition TEXT,
  settings_json TEXT,
  PRIMARY KEY (from_id, to_id)
);


-- ---------------------------------------------------------
-- 게임 시간
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_time_periods (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  start_minute INTEGER NOT NULL,
  end_minute INTEGER NOT NULL,
  UNIQUE (game_id, id)
);

-- ---------------------------------------------------------
-- 게임 날씨
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_weather_types (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  -- Encounter 확률에 미치는 배율
  encounter_rate_modifier REAL NOT NULL DEFAULT 1.0,
  -- 시야 배율
  visibility_modifier REAL NOT NULL DEFAULT 1.0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (game_id, id)
);

CREATE TABLE IF NOT EXISTS game_weather_pools (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (game_id, id)
);

CREATE TABLE IF NOT EXISTS game_weather_pool_entries (
  pool_id TEXT NOT NULL REFERENCES game_weather_pools(id) ON DELETE CASCADE,
  weather_id TEXT NOT NULL REFERENCES game_weather_types(id) ON DELETE CASCADE,
  -- 랜덤 선택 가중치
  weight INTEGER NOT NULL DEFAULT 1 CHECK (weight > 0),
  UNIQUE (pool_id, weather_id)
);

CREATE TABLE IF NOT EXISTS game_weather_transitions (
  from_weather_id TEXT NOT NULL REFERENCES game_weather_types(id) ON DELETE CASCADE,
  to_weather_id TEXT NOT NULL REFERENCES game_weather_types(id) ON DELETE CASCADE,
  map_id TEXT NOT NULL REFERENCES game_maps(id) ON DELETE CASCADE,
  probability REAL NOT NULL CHECK (probability >= 0.0 AND probability <= 1.0),
  PRIMARY KEY (from_weather_id, to_weather_id, map_id)
);

-- ---------------------------------------------------------
-- 게임 이벤트
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_events (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  event_type TEXT NOT NULL,                     -- 사용자가 정의하는 이벤트 타입
  trigger_type TEXT NOT NULL DEFAULT 'auto' CHECK (trigger_type IN ('manual','auto','interaction','location','scheduled','random')),
  repeat_policy TEXT NOT NULL DEFAULT 'once' CHECK (repeat_policy IN ('once','repeatable')),
  name TEXT NOT NULL,
  description TEXT NOT NULL,
  requires_npc INTEGER NOT NULL DEFAULT 0 CHECK (requires_npc IN (0, 1)),
  -- 이벤트별 추가설정
  config_json TEXT,
  effect_json TEXT,
  -- 동일 이벤트 재발생 방지 시간
  cooldown_seconds INTEGER NOT NULL DEFAULT 0 CHECK (cooldown_seconds >= 0),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS game_event_conditions (
  id TEXT PRIMARY KEY,
  event_id TEXT NOT NULL REFERENCES game_events(id) ON DELETE CASCADE,
  condition_type TEXT NOT NULL,
  condition_value TEXT
);

CREATE TABLE IF NOT EXISTS game_interactables (
  id TEXT PRIMARY KEY,
  location_id TEXT NOT NULL REFERENCES game_locations(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  event_id TEXT REFERENCES game_events(id) ON DELETE SET NULL
);


CREATE TABLE IF NOT EXISTS game_event_character_pool (
  event_id TEXT NOT NULL REFERENCES game_events(id) ON DELETE CASCADE,
  game_character_id TEXT NOT NULL REFERENCES game_characters(id) ON DELETE CASCADE,
  spawn_weight INTEGER NOT NULL DEFAULT 1 CHECK (spawn_weight > 0),
  PRIMARY KEY (event_id, game_character_id)
);

-- ---------------------------------------------------------
-- 게임 인카운트 테이블
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_encounter_tables (
  id TEXT PRIMARY KEY,
  map_id TEXT NOT NULL REFERENCES game_maps(id) ON DELETE CASCADE,
  -- NULL이면 Map 전체
  zone_id TEXT REFERENCES game_map_zones(id) ON DELETE CASCADE,
  location_id TEXT REFERENCES game_locations(id) ON DELETE CASCADE,
  -- 이동 시 Encounter Table 자체가 발동할 확률
  base_trigger_chance REAL NOT NULL DEFAULT 1.0 CHECK (base_trigger_chance >= 0.0 AND base_trigger_chance <= 1.0)
);

CREATE TABLE IF NOT EXISTS game_encounter_entries (
  id TEXT PRIMARY KEY,
  encounter_table_id TEXT NOT NULL REFERENCES game_encounter_tables(id) ON DELETE CASCADE,
  event_id TEXT NOT NULL REFERENCES game_events(id) ON DELETE CASCADE,
  -- 랜덤 가중치
  weight INTEGER NOT NULL DEFAULT 1 CHECK (weight > 0),
  -- NULL이면 모든 시간대
  time_period_id TEXT REFERENCES game_time_periods(id) ON DELETE CASCADE,
  -- NULL이면 모든 날씨
  weather_id TEXT REFERENCES game_weather_types(id)ON DELETE CASCADE
);

-- ---------------------------------------------------------
-- 게임 씬
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_scenes (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  scene_name TEXT NOT NULL,
  description TEXT,
  scene_key TEXT NOT NULL,          -- 예: yuna_intro
  is_ending INTEGER DEFAULT 0,
  ending_id TEXT,
  -- Scene 진입 시 실행되는 설정
  settings_json TEXT,
  script TEXT,

  -- Editor에서의 정렬 순서
  sort_order INTEGER DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (game_id, scene_key)
);

 -- branch 노드의 분기 목록을 담을 테이블 (choices와 비슷하지만 유저 입력 없이 조건만 평가됨)
CREATE TABLE IF NOT EXISTS game_branches (
  id TEXT PRIMARY KEY,
  scene_id TEXT NOT NULL REFERENCES game_scenes(id) ON DELETE CASCADE,
  condition TEXT,
  next_scene_id TEXT REFERENCES game_scenes(id) ON DELETE SET NULL,
  sort_order INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS game_choices (
  id TEXT PRIMARY KEY,
  scene_id TEXT NOT NULL REFERENCES game_scenes(id) ON DELETE CASCADE,
  text TEXT NOT NULL,
  next_scene_id TEXT REFERENCES game_scenes(id) ON DELETE SET NULL,
  -- 선택지가 표시되는 조건
  condition TEXT,
  effect TEXT,                      -- JSON: {"add": {"affection_yuna": 3}}
  sort_order INTEGER DEFAULT 0
);

-- ---------------------------------------------------------
-- 게임 엔딩
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_endings (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  ending_key TEXT NOT NULL,
  title TEXT NOT NULL,
  description TEXT,
  UNIQUE (game_id, ending_key)
);

-- ---------------------------------------------------------
-- 아이템
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_items (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  description TEXT,
  icon TEXT,                       -- 에셋 경로 등 (표시용)
  kind TEXT,                       -- 제작자가 정하는 열린 분류 (consumable/key/gift...). 엔진은 해석하지 않음
  max_stack INTEGER NOT NULL DEFAULT 99 CHECK (max_stack >= 1),
  is_unique INTEGER NOT NULL DEFAULT 0 CHECK (is_unique IN (0, 1)),   -- 하나만 소지 가능
  is_key INTEGER NOT NULL DEFAULT 0 CHECK (is_key IN (0, 1)),         -- 핵심 아이템
  consumable INTEGER NOT NULL DEFAULT 0 CHECK (consumable IN (0, 1)), -- 사용하면 하나 줄어듦
  use_effect TEXT,                 -- 이벤트 effect_json과 같은 형식. NULL이면 사용 불가 아이템
  use_condition TEXT,              -- Condition JSON. 사용할 수 있는 조건
  on_acquire_event_id TEXT REFERENCES game_events(id) ON DELETE SET NULL, -- 처음 입수할 때 발동할 manual 이벤트
  settings_json TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  CHECK (is_unique = 0 OR max_stack = 1)   -- 고유 아이템은 스택 1
);

-- ---------------------------------------------------------
-- 관계도
-- ---------------------------------------------------------
CREATE TABLE IF NOT EXISTS game_relationship_defs (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  from_id TEXT NOT NULL,   -- 'player' 또는 game_characters.id
  to_id TEXT NOT NULL,
  values_json TEXT,        -- {"affection": 10}
  tags TEXT,               -- ["자매"]
  UNIQUE (game_id, from_id, to_id)
);

-- ---------------------------------------------------------
-- Runtime (플레이 세션 & 로그)
-- ---------------------------------------------------------
-- game_saves 테이블과 달리 현재 실행 중인 상태
CREATE TABLE IF NOT EXISTS game_player_sessions (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  player_id TEXT REFERENCES personas(id) ON DELETE SET NULL, -- personas id와 연결
  current_scene_id TEXT REFERENCES game_scenes(id) ON DELETE SET NULL,
  current_map_id TEXT REFERENCES game_maps(id) ON DELETE SET NULL,
  current_zone_id TEXT REFERENCES game_map_zones(id) ON DELETE SET NULL,
  current_location_id TEXT REFERENCES game_locations(id) ON DELETE SET NULL,
  current_weather_id TEXT REFERENCES game_weather_types(id) ON DELETE SET NULL,
  game_time INTEGER NOT NULL DEFAULT 0,

  -- Runtime에서 사용하는 진행 상태
  -- JSON TEXT
  flags_json TEXT,
  pending_scenes_json TEXT, -- 씬 대기열
  inventory_json TEXT,       -- {"key_card": 1, "potion": 3}  (item_id → 수량)
  relationships_json TEXT,
  stage_json TEXT,

  -- 분기용
  parent_session_id TEXT REFERENCES game_player_sessions(id) ON DELETE SET NULL, -- 이전 세션 id를 가지고 있으면 분기로, 메시지와 대화 세션을 불러오는 역할을 한다.
  forked_from_seq INTEGER, -- 분기 순서

  -- 현재 진행 위치
  head_seq INTEGER NOT NULL,
  updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS game_saves (
  name TEXT,
  seq INTEGER NOT NULL,
  slot_number INTEGER NOT NULL,
  session_id TEXT NOT NULL REFERENCES game_player_sessions(id) ON DELETE CASCADE,
  current_scene_id TEXT REFERENCES game_scenes(id) ON DELETE SET NULL,
  current_map_id TEXT REFERENCES game_maps(id) ON DELETE SET NULL,
  current_zone_id TEXT REFERENCES game_map_zones(id) ON DELETE SET NULL,
  current_location_id TEXT REFERENCES game_locations(id) ON DELETE SET NULL,
  current_weather_id TEXT REFERENCES game_weather_types(id) ON DELETE SET NULL,
  game_time INTEGER NOT NULL DEFAULT 0,
  state_snapshot TEXT,
  updated_at INTEGER,
  UNIQUE (session_id, slot_number)
);

CREATE TABLE IF NOT EXISTS game_journal (
  session_id TEXT NOT NULL REFERENCES game_player_sessions(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL,
  kind TEXT NOT NULL,
  payload_json TEXT,
  game_time INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  UNIQUE (session_id, seq)
);

-- plans
CREATE TABLE IF NOT EXISTS game_story_plans (
  session_id TEXT NOT NULL REFERENCES game_player_sessions(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL,
  plan_json TEXT,
  reason TEXT,
  PRIMARY KEY (session_id, seq)
);

CREATE TABLE IF NOT EXISTS game_relationship_snapshots (
  session_id TEXT NOT NULL REFERENCES game_player_sessions(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL,
  relationships_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  PRIMARY KEY (session_id, seq)
);

CREATE TABLE IF NOT EXISTS game_character_state_snapshots (
  session_id TEXT NOT NULL REFERENCES game_player_sessions(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL,
  game_character_id TEXT NOT NULL REFERENCES game_characters(id) ON DELETE CASCADE,
  -- null이면 현재 맵에 등장 X
  current_location_id TEXT REFERENCES game_locations(id) ON DELETE SET NULL,
  -- active, absent, traveling, dead 등
  status TEXT NOT NULL DEFAULT 'active',
  state_json TEXT,
  updated_at INTEGER NOT NULL,

  PRIMARY KEY (session_id, seq, game_character_id)
);

-- event log
CREATE TABLE IF NOT EXISTS game_event_log (
  session_id TEXT NOT NULL REFERENCES game_player_sessions(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL, -- 순서용
  event_id TEXT NOT NULL REFERENCES game_events(id) ON DELETE CASCADE,
  weather_id TEXT REFERENCES game_weather_types(id) ON DELETE SET NULL,
  -- game_time 기준
  triggered_at INTEGER NOT NULL,
  -- 이벤트 실행 결과
  -- JSON TEXT
  result_json TEXT,
  PRIMARY KEY (session_id, seq)
);

CREATE TABLE IF NOT EXISTS game_event_log_characters (
  session_id TEXT NOT NULL REFERENCES game_player_sessions(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL, -- 순서용
  game_character_id TEXT NOT NULL REFERENCES game_characters(id) ON DELETE CASCADE,
  role TEXT,              -- participant, speaker, target 등
  sort_order INTEGER DEFAULT 0,
  PRIMARY KEY (session_id, seq, game_character_id),
  FOREIGN KEY (session_id, seq) REFERENCES game_event_log(session_id, seq) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS game_conversations (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  session_id TEXT NOT NULL REFERENCES game_player_sessions(id) ON DELETE CASCADE,
  branch_key TEXT NOT NULL DEFAULT '_default',
  -- 참여자 id를 정렬해서 ','로 이은 값. "이 참여자 조합의 대화"를 찾는 키.
  participants_key TEXT NOT NULL,
  title TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE (session_id, branch_key, participants_key)
);

CREATE TABLE IF NOT EXISTS game_messages (
  id TEXT PRIMARY KEY,
  seq INTEGER NOT NULL, -- 순서용
  conversation_id TEXT NOT NULL REFERENCES game_conversations(id) ON DELETE CASCADE,
  role TEXT NOT NULL,
  name TEXT, -- 스냅샷 역할
  content TEXT NOT NULL,
  character_id TEXT REFERENCES game_characters(id) ON DELETE SET NULL,
  meta TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER
);

CREATE TABLE IF NOT EXISTS game_conversation_participants (
  conversation_id TEXT NOT NULL REFERENCES game_conversations(id) ON DELETE CASCADE,
  game_character_id TEXT NOT NULL REFERENCES game_characters(id) ON DELETE CASCADE,
  sort_order INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (conversation_id, game_character_id)
);

CREATE TABLE IF NOT EXISTS game_scene_conversations (
  scene_id TEXT NOT NULL REFERENCES game_scenes(id) ON DELETE CASCADE,
  conversation_id TEXT NOT NULL REFERENCES game_conversations(id) ON DELETE CASCADE,
  sort_order INTEGER NOT NULL DEFAULT 0,

  PRIMARY KEY (scene_id, conversation_id)
);
-- =========================================================
-- 인덱스
-- =========================================================
--  project
CREATE INDEX IF NOT EXISTS idx_game_worldinfo ON game_worldinfo(game_id);
CREATE INDEX IF NOT EXISTS idx_game_state_defs_game ON game_state_defs(game_id);
CREATE INDEX IF NOT EXISTS idx_game_characters_project ON game_characters(game_id);
CREATE INDEX IF NOT EXISTS idx_game_character_worldinfo_character ON game_character_worldinfo(game_character_id);

-- scene
CREATE INDEX IF NOT EXISTS idx_game_scenes_game ON game_scenes(game_id);
CREATE INDEX IF NOT EXISTS idx_game_choices_node ON game_choices(scene_id);
CREATE INDEX IF NOT EXISTS idx_game_branches_node ON game_branches(scene_id);

-- character
CREATE INDEX IF NOT EXISTS idx_game_character ON game_characters(game_id);
CREATE INDEX IF NOT EXISTS idx_character_spawn_location ON game_character_spawn_locations(location_id);
CREATE INDEX IF NOT EXISTS idx_character_spawn_character ON game_character_spawn_locations(game_character_id);

-- assets
CREATE INDEX IF NOT EXISTS idx_game_assets_game ON game_assets(game_id);
CREATE INDEX IF NOT EXISTS idx_game_assets_game_category ON game_assets(game_id, category);
CREATE INDEX IF NOT EXISTS idx_audio_assets_character ON audio_assets(game_character_id);
CREATE INDEX IF NOT EXISTS idx_character_assets ON character_assets (game_character_id, usage, emotion);
CREATE INDEX IF NOT EXISTS idx_cg_assets_character ON cg_assets(game_character_id);
CREATE INDEX IF NOT EXISTS idx_background_assets ON background_assets (location_id, time_id, weather_id);
-- Map
CREATE INDEX IF NOT EXISTS idx_game_maps_game ON game_maps(game_id);
CREATE INDEX IF NOT EXISTS idx_map_zones_map_id ON game_map_zones(map_id);
CREATE INDEX IF NOT EXISTS idx_map_locations_zone_id ON game_locations(zone_id);
CREATE INDEX IF NOT EXISTS idx_map_locations_lockup ON game_location_connections(from_id, to_id);

-- time / weather
CREATE INDEX IF NOT EXISTS idx_time_periods_game ON game_time_periods(game_id);
CREATE INDEX IF NOT EXISTS idx_weather_types_game ON game_weather_types(game_id);
CREATE INDEX IF NOT EXISTS idx_weather_pools_game ON game_weather_pools(game_id);
CREATE INDEX IF NOT EXISTS idx_weather_pool_entries_weather ON game_weather_pool_entries(weather_id);
CREATE INDEX IF NOT EXISTS idx_weather_transitions_lookup ON game_weather_transitions(map_id, from_weather_id);

-- events
CREATE INDEX IF NOT EXISTS idx_events_game ON game_events(game_id);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON game_events(event_type);
CREATE INDEX IF NOT EXISTS idx_event_conditions_event_id ON game_event_conditions(event_id);
CREATE INDEX IF NOT EXISTS idx_event_interactables_location_id ON game_interactables(location_id);
CREATE INDEX IF NOT EXISTS idx_event_character_pool_event ON game_event_character_pool(event_id);
CREATE INDEX IF NOT EXISTS idx_event_character_pool_character ON game_event_character_pool(game_character_id);
CREATE INDEX IF NOT EXISTS idx_event_trigger_type ON game_events(game_id, trigger_type);
CREATE INDEX IF NOT EXISTS idx_event_log_session_event_id ON game_event_log(session_id, event_id);

-- encounters
CREATE INDEX IF NOT EXISTS idx_encounter_tables_map_zone ON game_encounter_tables(map_id, zone_id);
CREATE INDEX IF NOT EXISTS idx_encounter_entries_lookup ON game_encounter_entries(encounter_table_id, time_period_id, weather_id);
CREATE INDEX IF NOT EXISTS idx_encounter_entries_event ON game_encounter_entries(event_id);

-- item
CREATE INDEX IF NOT EXISTS idx_game_items_game ON game_items(game_id);

-- runtime
CREATE INDEX IF NOT EXISTS idx_player_sessions_game ON game_player_sessions(game_id);
CREATE INDEX IF NOT EXISTS idx_player_sessions_player_id ON game_player_sessions(player_id);
CREATE INDEX IF NOT EXISTS idx_event_log_session_time ON game_event_log(session_id, triggered_at);
CREATE INDEX IF NOT EXISTS idx_event_log_event ON game_event_log(event_id);
CREATE INDEX IF NOT EXISTS idx_event_log_characters_character_id ON game_event_log_characters(game_character_id);
CREATE INDEX IF NOT EXISTS idx_character_states_location ON game_character_state_snapshots(session_id, seq, current_location_id);

-- conversation
CREATE INDEX IF NOT EXISTS idx_game_messages_conversation ON game_messages(conversation_id, seq);
CREATE INDEX IF NOT EXISTS idx_game_conversation_participants ON game_conversation_participants(conversation_id, game_character_id);
CREATE INDEX IF NOT EXISTS idx_game_scene_conversations ON game_scene_conversations(scene_id, conversation_id);