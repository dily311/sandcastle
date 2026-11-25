-- 5-F-7-1 (ADR-030 추가): 대화 유틸 지시문은 프리셋 블록의 매크로 자리에만 들어간다. 기본 프리셋에 그 자리를 하나 둔다
-- (캐릭터 이미지·다인봇·자동 그림·AI 변수 — 기능이 꺼져 있으면 빈 글이라 빠짐). 대화 기록 앞에.
INSERT OR IGNORE INTO prompt_blocks (id, preset_id, identifier, block_type, name, role, content, enabled, position, depth, sort_order, triggers_json, created_at, updated_at)
SELECT 'default-utils', 'default', 'utils', 'text', 'Utility Prompts', 'system',
 '{{img_inprompt}}
{{multi_char}}
{{auto_image}}
{{vars}}',
 1, 'relative', NULL, 5, '[]', strftime('%s','now') * 1000, strftime('%s','now') * 1000
WHERE EXISTS (SELECT 1 FROM prompt_presets WHERE id = 'default');

-- 기록·리마인더를 한 칸씩 뒤로 (사용자가 순서를 바꿨으면 그대로 둔다)
UPDATE prompt_blocks SET sort_order = 7 WHERE id = 'default-post' AND sort_order = 6;
UPDATE prompt_blocks SET sort_order = 6 WHERE id = 'default-history' AND sort_order = 5;
