# 개요

## game_assets - 게임 에셋 파일 관리 테이블

```sql
CREATE TABLE IF NOT EXISTS game_assets (
  id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL REFERENCES game_projects(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  name TEXT NOT NULL,
  category TEXT NOT NULL CHECK (category IN ('voice','sfx','bgm','character','cg','background','font','other')),
  kind TEXT NOT NULL CHECK (kind IN ('image','audio','font','other')),
  file_path TEXT NOT NULL,
  settings_json TEXT,                       -- 볼륨, 루프, 페이드
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
  UNIQUE (game_id, key)
);

```

**해당 테이블의 하위 테이블 목록**
- 오디오 에셋
- 이미지 에셋
    - character_assets: 캐릭터 에셋. 감정별 분리(happy, sad, angry 등).
    - cg_assets: 특정 장면이나 이벤트에서 표시하는 cg 이미지
    - background_assets: 특정 위치마다의 배경 에셋 (존재 하지 않는 경우 기본 에셋으로 대체) 시간대에 따라 다른 배경을 표시할 수 있음. 시간대가 정의되지 않으면 기본 에셋

### 오디오 에셋

지금은 저장된 오디오 파일을 불러오는 기능만. 추후 외부 API와 연결해서 실시간 생성기능이 추가되면 좋겠다.(희망사항)

```sql
CREATE TABLE IF NOT EXISTS audio_assets (                  -- voice / sfx / bgm
  assets_id TEXT PRIMARY KEY REFERENCES game_assets(id) ON DELETE CASCADE,
  tag TEXT,                                  -- sfx: 소리 그룹 이름, bgm: 분위기 id
  game_character_id TEXT REFERENCES game_characters(id) ON DELETE CASCADE,  -- voice의 화자
  transcript TEXT                            -- voice의 대사 텍스트
);
```
예시 tag list: [물방울 소리, 새소리, 나뭇잎 소리...]

위 태그리스트만 ai에게 전송

tag 새 소리에서 새 소리\_1.mp3, 새 소리\_2.mp3 등이 있다면 랜덤으로 하나를 선택.
(끝에 숫자가 있는 경우에만)

현재 분위기: 기본 / 평화 / 경쾌 / 로맨스 / 살벌 / 전투 
이 중 하나만 텍스트로 받고, bgm을 출력

### 이미지 에셋 출력 규칙

| 분류              | ai 호출여부 |
| ----------------- | ----------- |
| character_assets  | X           |
| cg_assets         | O           |
| background_assets | X           |

#### character_assets - 캐릭터 이미지 에셋

```sql
CREATE TABLE IF NOT EXISTS character_assets (
  assets_id TEXT PRIMARY KEY REFERENCES game_assets(id) ON DELETE CASCADE,
  game_character_id TEXT NOT NULL REFERENCES game_characters(id) ON DELETE CASCADE,
  usage TEXT NOT NULL CHECK (usage IN ('standing','map')),
  emotion TEXT,                              -- standing은 필수, map은 NULL 또는 variant
  UNIQUE (game_character_id, usage, emotion)
);
```

#### cg_assets - cg 이미지 에셋

```sql
CREATE TABLE IF NOT EXISTS cg_assets (
  assets_id TEXT PRIMARY KEY REFERENCES game_assets(id) ON DELETE CASCADE,
  game_character_id TEXT REFERENCES game_characters(id) ON DELETE SET NULL,
  prompt TEXT, 
  negative_prompt TEXT, 
  seed INTEGER, 
  generation_params TEXT
);
```

cg_assets는 파일명과 설명을 보내는 방식과 ai에게는 이미지 태그로 받은 이후 nai 등에서 이미지로 출력하는 방식 두개를 고려중이야.

#### background_assets - 배경 이미지 에셋

```sql
CREATE TABLE IF NOT EXISTS background_assets (
  assets_id TEXT PRIMARY KEY REFERENCES game_assets(id) ON DELETE CASCADE,
  location_id TEXT REFERENCES game_locations(id) ON DELETE CASCADE,   -- NULL = 전체 기본
  time_id TEXT REFERENCES game_time_periods(id) ON DELETE SET NULL,
  weather_id TEXT REFERENCES game_weather_types(id) ON DELETE SET NULL
);
```

# 메모

- 유기적으로 데이터가 연결될 수 있게 신경쓰자.
- 쿼리문 짤 때 오타 주의.
