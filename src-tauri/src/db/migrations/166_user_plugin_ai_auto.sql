-- 플러그인의 AI 호출 (CLAUDE.md 6-D-7, ADR-012): 사용자가 누른 명령 밖에서(훅·패널) 플러그인이 스스로 AI를 부르게 할지.
-- 기본 꺼짐 — 꺼져 있으면 사용자가 누른 명령 안에서만, 패널은 호출마다 확인을 받는다. 기기마다 (user_plugins는 동기화 안 함)
ALTER TABLE user_plugins ADD COLUMN ai_auto INTEGER NOT NULL DEFAULT 0;
