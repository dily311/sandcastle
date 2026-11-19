-- 폰 화면: 페르소나(폰 주인)마다 따로 가진 연락처
-- - 캐릭터나 페르소나가 지워지면 연락처도 지워짐
-- - 대화 자체는 conversations(mode = 'phone')에 있음
CREATE TABLE IF NOT EXISTS phone_contacts (
  persona_id   TEXT NOT NULL REFERENCES personas(id) ON DELETE CASCADE,
  character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  -- 연락처에 보이는 상태 메시지
  status       TEXT NOT NULL DEFAULT '',
  sort_order   INTEGER NOT NULL DEFAULT 0,
  created_at   INTEGER NOT NULL,
  PRIMARY KEY (persona_id, character_id)
);
