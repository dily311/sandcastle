-- =========================================================
-- 예산 한도를 "한 달 금액(달러)" 하나로 (ADR-007 추가)
-- ---------------------------------------------------------
-- - 호출마다 그때의 단가로 계산한 금액(cost_usd)을 같이 기록한다. 나중에 단가를 고쳐도 지난 기록은 안 바뀐다.
--   cost_usd가 NULL이면 계산하지 않은 줄(음성·그림·이 기능 이전의 줄). 단가를 몰라 계산하지 못한 호출은 unpriced = 1.
-- - budget_limits에는 monthly_usd를 더한다. 예전 칸(하루·한 달 토큰, 음성 글자, 그림 장 수)은 더 이상 쓰지 않지만
--   지우지 않는다(새 한도를 정할 때 비워 "이전 한도" 안내가 사라진다).
-- =========================================================

ALTER TABLE usage_log ADD COLUMN cost_usd REAL;
ALTER TABLE usage_log ADD COLUMN unpriced INTEGER NOT NULL DEFAULT 0;
ALTER TABLE usage_daily ADD COLUMN cost_usd REAL;
ALTER TABLE usage_daily ADD COLUMN unpriced_calls INTEGER NOT NULL DEFAULT 0;
ALTER TABLE budget_limits ADD COLUMN monthly_usd REAL;
