-- =========================================================
-- 정규식 치환 스크립트
-- ---------------------------------------------------------
-- SillyTavern regex 확장과 같은 개념을 쓴다 (가져오기 시 1:1 대응).
-- - placement: 어떤 텍스트에 적용할지 (user_input / ai_output / world_info)
-- - target:   어디에 반영할지
--     stored  - 저장되는 메시지 자체를 바꿈 (ST 기본)
--     prompt  - AI에게 보내는 프롬프트에서만 바꿈 (ST promptOnly)
--     display - 화면 표시에서만 바꿈 (ST markdownOnly)
-- - scope: global / character / preset (scope_id = character_id / preset_id)
-- =========================================================
CREATE TABLE IF NOT EXISTS regex_scripts (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  -- "/pattern/flags" (JS 표기) 또는 플래그 없는 패턴
  find_regex TEXT NOT NULL,
  -- $1, {{match}}, {{char}}, {{user}} 사용 가능
  replace_string TEXT NOT NULL DEFAULT '',
  -- 캡처 그룹을 넣기 전에 지울 문자열 목록
  trim_strings TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(trim_strings)),
  placement TEXT NOT NULL DEFAULT '["ai_output"]' CHECK (json_valid(placement)),
  target TEXT NOT NULL DEFAULT 'stored' CHECK (target IN ('stored', 'prompt', 'display')),
  scope TEXT NOT NULL DEFAULT 'global' CHECK (scope IN ('global', 'character', 'preset')),
  scope_id TEXT,
  -- find_regex 안의 {{char}}/{{user}} 치환: none / raw / escaped
  substitute_macros TEXT NOT NULL DEFAULT 'none' CHECK (substitute_macros IN ('none', 'raw', 'escaped')),
  -- 사용자가 메시지를 수정했을 때도 다시 적용할지
  run_on_edit INTEGER NOT NULL DEFAULT 1 CHECK (run_on_edit IN (0, 1)),
  -- 적용할 메시지 깊이 범위 (0 = 가장 최근 메시지, NULL = 제한 없음)
  min_depth INTEGER CHECK (min_depth IS NULL OR min_depth >= 0),
  max_depth INTEGER CHECK (max_depth IS NULL OR max_depth >= 0),
  enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
  sort_order INTEGER NOT NULL DEFAULT 0,
  -- native / sillytavern
  source_format TEXT NOT NULL DEFAULT 'native',
  -- 미지원 필드 보존
  extra_json TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_regex_scripts_scope ON regex_scripts(scope, scope_id, sort_order);
