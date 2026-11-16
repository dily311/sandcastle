-- 공용 번역 캐시: (원문 해시, 언어) → 번역문. 채팅·폰·게임이 같이 쓴다 (코어 플러그인 '번역').
-- 같은 문장(게임 대본 줄 등)은 세션·분기·재플레이에서 다시 번역하지 않는다.
-- source_hash는 원문의 FNV-1a 64비트(16진수), 충돌을 막으려고 원문도 함께 둔다. 원문이 바뀌면 해시가 달라져 자연히 무효.
CREATE TABLE IF NOT EXISTS translation_cache (
  source_hash TEXT NOT NULL,
  lang TEXT NOT NULL,
  source TEXT NOT NULL,
  text TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  PRIMARY KEY (source_hash, lang)
);
