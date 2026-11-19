-- 커뮤니티 플러그인 (CLAUDE.md 6-D-1, ADR-012).
-- 설치된 것 = 앱 데이터의 plugins/<id>/ 폴더(manifest.json). 이 표는 기기마다의 상태만 둔다:
-- 켜짐(기본 꺼짐), 켤 때 사용자가 허락한 권한, 설치한 코드의 해시. 동기화하지 않는다(change_tracking::UNSYNCED_TABLES).
CREATE TABLE IF NOT EXISTS user_plugins (
  id TEXT PRIMARY KEY NOT NULL,
  -- manifest의 이름 (휴지통 목록에 보임)
  name TEXT NOT NULL DEFAULT '',
  version TEXT NOT NULL DEFAULT '',
  enabled INTEGER NOT NULL DEFAULT 0,
  -- JSON 배열: 켤 때 동의한 권한
  granted TEXT NOT NULL DEFAULT '[]',
  code_hash TEXT NOT NULL DEFAULT '',
  installed_at INTEGER NOT NULL DEFAULT 0,
  -- 메시지 후처리 방식 (CLAUDE.md 6-D-4, ADR-012): notes = 표시만(기본) / fix = 자동 고침.
  -- 플러그인이 글을 바꾸는 것은 사용자가 "자동 고침"을 고른 플러그인만. 기기마다 (user_plugins는 동기화 안 함).
  process_mode TEXT NOT NULL DEFAULT 'notes',
  -- 플러그인의 AI 호출 (CLAUDE.md 6-D-7, ADR-012): 사용자가 누른 명령 밖에서(훅·패널) 플러그인이 스스로 AI를 부르게 할지.
  -- 기본 꺼짐 — 꺼져 있으면 사용자가 누른 명령 안에서만, 패널은 호출마다 확인을 받는다. 기기마다 (user_plugins는 동기화 안 함)
  ai_auto INTEGER NOT NULL DEFAULT 0,
  -- 플러그인 판 폴더 (CLAUDE.md 1-F-1, ADR-012 갱신): 플러그인 a의 폴더는 plugins/a/<판>/ 들이고, 지금 쓰는 판 폴더 이름을 기기마다 기억한다.
  -- 비어 있으면(예전 행) 판 문자열과 같은 이름의 폴더, 그것도 없으면 가장 새 판. 기기마다 따로 (user_plugins와 같이 동기화 안 함).
  active_dir TEXT NOT NULL DEFAULT '',
  updated_at INTEGER NOT NULL DEFAULT 0
);
