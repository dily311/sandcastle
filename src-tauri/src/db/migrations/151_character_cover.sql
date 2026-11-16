-- 캐릭터 페이지 위쪽 커버(배너) 이미지. 앱 데이터 characters/covers/에 복사한 파일 경로. 없으면 기본 그라데이션
ALTER TABLE characters ADD COLUMN cover TEXT;
