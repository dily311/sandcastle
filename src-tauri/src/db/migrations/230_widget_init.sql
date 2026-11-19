-- =========================================================
-- 위젯 모드에서 사용되는 DB
-- ---------------------------------------------------------
-- 설명
-- 1. 시메지: 유저가 임포트한 캐릭터 정보 저장
-- 2. 음악 플레이어: 캐싱된 음악 메타 데이터 저장
-- ---------------------------------------------------------
-- 테이블 추가원칙
-- 1. pos 데이터는 DB 저장 XXXXXX
-- =========================================================

-- ---------------------------------------------------------
-- 시메지
-- ---------------------------------------------------------
-- 캐릭터 관리 테이블
CREATE TABLE IF NOT EXISTS widget_characters (
    id TEXT PRIMARY KEY,
    name TEXT,
    definition_path TEXT,
    enable INTEGER NOT NULL DEFAULT 0,
    display_order INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS widget_character_settings (
    character_id TEXT PRIMARY KEY NOT NULL REFERENCES widget_characters(id) ON DELETE CASCADE,
    scale INTEGER NOT NULL DEFAULT 0,
    speed INTEGER NOT NULL DEFAULT 0,
    opacity	INTEGER NOT NULL DEFAULT 0,
    behavior_settings TEXT,
    position_settings TEXT
);

-- ---------------------------------------------------------
-- 음악 플레이어
-- ---------------------------------------------------------
-- 캐싱된 음악 메타 데이터
-- 데이터 내보내기 기능을 구현할 경우 src는 절대경로가 아닌 상대 경로로 저장해야 함
CREATE TABLE IF NOT EXISTS music_meta (
    path TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    artist TEXT,
    album TEXT,
    album_title TEXT,
    album_artist TEXT,
    track_number TEXT,
    genre TEXT,
    comment TEXT,
    album_art TEXT,
    lyrics TEXT,
    duration REAL NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

-- 앨범아트 표시 위치
CREATE TABLE IF NOT EXISTS music_album_art_pos (
    music_src TEXT PRIMARY KEY NOT NULL REFERENCES music_meta(path) ON DELETE CASCADE,
    pos TEXT NOT NULL    
);

-- 유저가 생성한 플레이 리스트
CREATE TABLE IF NOT EXISTS playlist (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    cover_image TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS playlist_entries (
    playlist_id TEXT NOT NULL REFERENCES playlist(id) ON DELETE CASCADE,
    music_src TEXT NOT NULL REFERENCES music_meta(path) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0,

    PRIMARY KEY (playlist_id, music_src)
);

-- =========================================================
-- 인덱스
-- =========================================================
CREATE INDEX IF NOT EXISTS idx_playlist_entries_order ON playlist_entries(playlist_id, sort_order);

-- =========================================================
-- 기본 설정
-- =========================================================
INSERT OR IGNORE INTO playlist (id, title, sort_order, created_at, updated_at) VALUES ("default", "default", 0, strftime('%s','now') * 1000, strftime('%s','now') * 1000);
