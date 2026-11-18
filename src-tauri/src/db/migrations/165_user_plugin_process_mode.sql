-- 메시지 후처리 방식 (CLAUDE.md 6-D-4, ADR-012): notes = 표시만(기본) / fix = 자동 고침.
-- 플러그인이 글을 바꾸는 것은 사용자가 "자동 고침"을 고른 플러그인만. 기기마다 (user_plugins는 동기화 안 함).
ALTER TABLE user_plugins ADD COLUMN process_mode TEXT NOT NULL DEFAULT 'notes';
