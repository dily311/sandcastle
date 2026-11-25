-- 5-F-4-5 (ADR-029 ④나): 캐릭터의 상태창 틀 (HTML + {{getvar::x}}). 변수로 채워 고정 자리에 그린다 — AI가 상태창 글을 쓰지 않아도 됨.
ALTER TABLE characters ADD COLUMN status_template TEXT;
