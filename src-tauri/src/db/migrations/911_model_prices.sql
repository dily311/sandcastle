-- =========================================================
-- 단가표 (CLAUDE.md 1-A-3, ADR-007)
-- ---------------------------------------------------------
-- 모델별 단가를 사용자가 적는다. 앱은 가격을 넣어 두지 않는다(가격은 바뀐다).
-- 금액은 사용량 × 단가로 계산한 "추정"이며 화면에도 그렇게 표시한다.
-- 비운 칸은 0으로 계산한다
-- =========================================================

CREATE TABLE IF NOT EXISTS model_prices (
  -- usage_log.model과 같은 문자열
  model TEXT PRIMARY KEY,
  -- 표시용 통화 기호 ("$", "₩" 등)
  currency TEXT NOT NULL DEFAULT '$',
  -- 입력·출력 100만 토큰당
  input_per_mtok REAL,
  output_per_mtok REAL,
  -- 음성 1000자당
  per_1k_chars REAL,
  -- 그림 1장당
  per_image REAL,
  updated_at INTEGER NOT NULL
);
