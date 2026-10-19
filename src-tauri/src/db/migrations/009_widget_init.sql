
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

CREATE TABLE IF NOT EXISTS music_album_art_pos (
    music_src TEXT PRIMARY KEY NOT NULL REFERENCES music_meta(path) ON DELETE CASCADE,
    pos TEXT NOT NULL    
);

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

CREATE INDEX IF NOT EXISTS idx_playlist_entries_order ON playlist_entries(playlist_id, sort_order);

-- =========================================================
-- 기본 설정
-- =========================================================
INSERT OR IGNORE INTO playlist (id, title, sort_order, created_at, updated_at) VALUES ("default", "default", 0, strftime('%s','now') * 1000, strftime('%s','now') * 1000);
