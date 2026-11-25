-- 5-E-29-1 (ADR-028): 대화 옵션은 자물쇠(이 대화) > 캐릭터 전용 > 전역 순서.
-- 전에는 새 일반 대화가 페르소나를 복사해 두어 전역(사이드바)을 바꿔도 따라오지 않았다.
-- 일반 대화의 페르소나가 지금의 캐릭터 기본(없으면 시스템 기본)과 같으면 비운다 — 보이는 값은 그대로이고 앞으로 전역을 따른다.
-- 다르면(사용자가 일부러 고른 것) 남긴다 = 잠김. 폰 대화는 폰 주인이라 그대로.
-- updated_at을 직접 바꿔 touch 트리거(920)가 지금 시각으로 올리지 않게 한다 (최근 대화 순서 유지).
UPDATE conversations
SET persona_id = NULL,
    updated_at = COALESCE(updated_at, 0) + 1
WHERE (mode IS NULL OR mode = 'chat')
  AND persona_id IS NOT NULL
  AND persona_id <> ''
  AND persona_id = COALESCE(
    NULLIF((SELECT c.default_persona_id FROM characters c WHERE c.id = conversations.character_id), ''),
    (SELECT s.default_persona_id FROM conv_settings s WHERE s.id = 1),
    'default'
  );
