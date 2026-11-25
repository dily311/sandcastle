-- 5-F-4-3 (ADR-029): 캐릭터의 대화 변수 정의 (JSON 배열 — 이름·설명·종류·범위·AI 읽기/쓰기). 없으면 NULL = 정의 없음.
-- 값은 대화 meta.variables에 그대로 둔다 (정의는 선택)
ALTER TABLE characters ADD COLUMN var_defs TEXT;
