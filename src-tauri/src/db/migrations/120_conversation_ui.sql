-- =========================================================
-- 대화 화면 지원
-- ---------------------------------------------------------
-- - conversation_folders: 캐릭터별 대화 폴더 (대화 상세 왼쪽 목록)
-- - conversations.folder_id / api_profile_id: 대화별 폴더·API (프리셋은 prompt_preset_id가 이미 있음)
--   비어 있으면 앱 설정의 선택값을 쓴다
-- - message_prompts: AI 응답을 만들 때 실제로 보낸 프롬프트 ("보낸 컨텍스트 보기")
-- =========================================================
CREATE TABLE IF NOT EXISTS conversation_folders (
  id TEXT PRIMARY KEY,
  character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_conversation_folders_char ON conversation_folders(character_id, sort_order);

ALTER TABLE conversations ADD COLUMN folder_id TEXT REFERENCES conversation_folders(id) ON DELETE SET NULL;
ALTER TABLE conversations ADD COLUMN api_profile_id TEXT REFERENCES api_profiles(id) ON DELETE SET NULL;

CREATE TABLE IF NOT EXISTS message_prompts (
  message_id TEXT PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
  prompt TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
