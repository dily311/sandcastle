-- 읽기 모드 (5-E-6): 켜면 대화 화면에서 보내는 줄을 숨긴다. 대화마다 저장해 다시 열어도 유지
ALTER TABLE conversations ADD COLUMN read_mode INTEGER NOT NULL DEFAULT 0;
