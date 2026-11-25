-- API 키 연결 확인 여부 (연결 테스트 성공 또는 실제 AI 응답 수신 시 1로 갱신)
ALTER TABLE api_profiles ADD COLUMN verified INTEGER NOT NULL DEFAULT 0;
