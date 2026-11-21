-- 대화 목록에 "마지막 대화 시각" (5-E-3)
-- updated_at은 이 기기에서 행이 바뀐 시각이라 동기화가 변경 표시로 쓴다. 가져온 대화처럼 오래된 메시지를 담은 대화도
-- 목록에서는 마지막 메시지 시각으로 보이고 정렬되도록, 뷰에 메시지의 가장 늦은 작성 시각을 더한다.
DROP VIEW IF EXISTS v_conversations_with_count;
CREATE VIEW v_conversations_with_count AS
SELECT c.*, COUNT(m.id) AS message_count, MAX(m.created_at) AS last_message_at
FROM conversations c
LEFT JOIN messages m ON c.id = m.conversation_id
GROUP BY c.id;
