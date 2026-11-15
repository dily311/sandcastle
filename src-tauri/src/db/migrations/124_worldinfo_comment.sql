-- 로어북 항목 제목 (SillyTavern의 comment). 목록에서 항목을 알아보는 이름
ALTER TABLE worldinfo_entries ADD COLUMN comment TEXT NOT NULL DEFAULT '';
