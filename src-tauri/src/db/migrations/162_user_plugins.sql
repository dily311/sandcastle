-- 커뮤니티 플러그인 (CLAUDE.md 6-D-1, ADR-012).
-- 설치된 것 = 앱 데이터의 plugins/<id>/ 폴더(manifest.json). 이 표는 기기마다의 상태만 둔다:
-- 켜짐(기본 꺼짐), 켤 때 사용자가 허락한 권한, 설치한 코드의 해시. 동기화하지 않는다(change_tracking::UNSYNCED_TABLES).
CREATE TABLE user_plugins (
  id TEXT PRIMARY KEY NOT NULL,
  -- manifest의 이름 (휴지통 목록에 보임)
  name TEXT NOT NULL DEFAULT '',
  version TEXT NOT NULL DEFAULT '',
  enabled INTEGER NOT NULL DEFAULT 0,
  -- JSON 배열: 켤 때 동의한 권한
  granted TEXT NOT NULL DEFAULT '[]',
  code_hash TEXT NOT NULL DEFAULT '',
  installed_at INTEGER NOT NULL DEFAULT 0,
  updated_at INTEGER NOT NULL DEFAULT 0
);
