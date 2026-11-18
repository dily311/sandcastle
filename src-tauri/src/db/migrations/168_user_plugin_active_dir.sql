-- 플러그인 판 폴더 (CLAUDE.md 1-F-1, ADR-012 갱신): 플러그인 a의 폴더는 plugins/a/<판>/ 들이고, 지금 쓰는 판 폴더 이름을 기기마다 기억한다.
-- 비어 있으면(예전 행) 판 문자열과 같은 이름의 폴더, 그것도 없으면 가장 새 판. 기기마다 따로 (user_plugins와 같이 동기화 안 함).
ALTER TABLE user_plugins ADD COLUMN active_dir TEXT NOT NULL DEFAULT '';
