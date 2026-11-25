-- 로어북 항목 분류 (5-F-1, ADR-023): 캐릭터·장소·아이템·이벤트·세계, 사용자가 더 만들 수 있음. 빈 글 = 분류 없음
ALTER TABLE worldinfo_entries ADD COLUMN category TEXT NOT NULL DEFAULT '';
