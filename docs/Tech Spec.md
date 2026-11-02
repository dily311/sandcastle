# 개요

## 시스템 아키텍처 및 기술 스택

### 기술 스택 버전 및 역할 분담:
- Core / System: Tauri 2.0 (Rust IPC, 네이티브 윈도우 제어)
- Frontend UI: Svelte 5 (Runes 기반 반응형 UI, 오버레이 컨트롤)
- Rendering Engine: Pixi.js (하드웨어 가속 캔버스 렌더링, 애니메이션 루프)
- Database: SQLite + sqlx (로컬 영구 저장소, 파일 기반 마이그레이션)

### 전체 아키텍처

```
[Svelte Application] (Chat UI · Game UI · Editor · Settings · Widgets)
⇅
[Tauri IPC Layer] Commands · Events · Channels
⇅
[Rust Core]

- Chat Engine (Prompt · Context · AI Provider)
- Game Engine (Runtime · State · Scene · Event)
- Asset Manager (Images · Audio · Package)
- Data Layer (SQLite · Files · Migrations)
⇅
[External AI Providers/외부 API 연결]
- OpenAI-compatible API · Local AI · Embedding

---

[PixiJS Renderer]
- Game Canvas · Animation · Map · Character · Effects
- GameRuntime의 화면 표현을 담당
```

1. 외부 API 호출은 **반드시** rust 단에서만
2. svelte5와 Pixi.js는 각 역할이 다름/ svelte5는 UI 표시, Pixi.js는 이미지 에셋 표시 관리
3. SQLite를 중심으로 데이터 관리


## DB 설계

자세한 db 구조는 `docs/db` 폴더를 확인할 것


### 서비스 프로필

API 연결 프로필을 정의

### 게임 엔진

#### 게임 엔진 플레이 세션 흐름

1. start_new_game or continue_game 호출 (GameResponse/렌더링 데이터)
2. getGameView 호출 (정적 데이터)
3. 프론트에서 표시
4. 유저의 행동에 따라 dispatch_command 호출 (GameResponse 반환)

##### 데이터 예시

랜더링 데이터:

```json
{
    "events": [
        {
            "type": "game_started"
        },
        {
            "type": "npc_moved",
            "game_character_id": "3494af25-4350-4223-884f-5d729dca2641",
            "from": null,
            "to": null
        },
        {
            "type": "npc_moved",
            "game_character_id": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
            "from": null,
            "to": "69e1694f-e70e-471c-bf24-b026a92172f0"
        },
        {
            "type": "npc_moved",
            "game_character_id": "d24beaaa-687f-4029-a0ed-af9e31f77626",
            "from": null,
            "to": null
        },
        {
            "type": "stage_lines",
            "lines": [
                {
                    "kind": "direction",
                    "speaker": null,
                    "text": "",
                    "cues": [
                        {
                            "type": "background",
                            "asset": {
                                "id": "04de1e8d-96a7-4d01-a911-18886127b974",
                                "key": "bg_class",
                                "path": "assets/background/bg_class.png",
                                "settings": null
                            }
                        }
                    ]
                }
            ]
        },
        {
            "type": "scene_script",
            "scene_id": "9dd48c11-c807-4690-847c-e8c872617601",
            "lines": [
                {
                    "kind": "narration",
                    "speaker": null,
                    "text": "새 학기의 첫날 아침, 교실에는 벚꽃이 흩날린다.",
                    "cues": [
                        {
                            "type": "bgm",
                            "mood": "peaceful",
                            "asset": {
                                "id": "719f6a1c-6a68-495c-8fb4-ac5dacbbcdaa",
                                "key": "bgm.peaceful",
                                "path": "assets/audio/bgm_peaceful.wav",
                                "settings": {
                                    "loop": true,
                                    "volume": 0.5
                                }
                            }
                        }
                    ]
                },
                {
                    "kind": "narration",
                    "speaker": null,
                    "text": "창밖에서 새 지저귀는 소리가 들려온다.",
                    "cues": [
                        {
                            "type": "sfx",
                            "tag": "새 소리",
                            "asset": {
                                "id": "c4b5c63c-0874-4b1d-989a-73866f4b65be",
                                "key": "sfx.bird.2",
                                "path": "assets/audio/bird_2.wav",
                                "settings": {
                                    "volume": 0.6
                                }
                            }
                        }
                    ]
                },
                {
                    "kind": "dialogue",
                    "speaker": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                    "text": "아, 안녕! 오늘도 일찍 왔네?",
                    "cues": [
                        {
                            "type": "standing",
                            "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                            "emotion": "happy",
                            "asset": {
                                "id": "271df3d4-9124-41d0-a4ec-aeab8f92e0a3",
                                "key": "yuna_happy_2",
                                "path": "assets/standing/yuna_happy_2.png",
                                "settings": null
                            }
                        },
                        {
                            "type": "voice",
                            "asset": {
                                "id": "dc6dd2a1-9b77-42be-a4ea-2e2bc5eb558d",
                                "key": "yuna_greet",
                                "path": "assets/audio/voice_yuna_greet.wav",
                                "settings": {
                                    "volume": 0.9
                                }
                            }
                        }
                    ]
                },
                {
                    "kind": "dialogue",
                    "speaker": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                    "text": "올해도 같은 반이라서 다행이야.",
                    "cues": [
                        {
                            "type": "standing",
                            "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                            "emotion": "shy",
                            "asset": {
                                "id": "5939f5f0-b442-4907-a367-91391d3326de",
                                "key": "yuna_shy",
                                "path": "assets/standing/yuna_shy.png",
                                "settings": null
                            }
                        },
                        {
                            "type": "motion",
                            "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                            "name": "nod"
                        }
                    ]
                },
                {
                    "kind": "dialogue",
                    "speaker": "3494af25-4350-4223-884f-5d729dca2641",
                    "text": "어이, 유나! 같이 가자!",
                    "cues": [
                        {
                            "type": "sfx",
                            "tag": "문 소리",
                            "asset": {
                                "id": "008c53b6-d7f1-4263-ad5e-8cc6b57ea651",
                                "key": "sfx.door.2",
                                "path": "assets/audio/door_2.wav",
                                "settings": {
                                    "volume": 0.8
                                }
                            }
                        },
                        {
                            "type": "standing",
                            "characterId": "3494af25-4350-4223-884f-5d729dca2641",
                            "emotion": "happy",
                            "asset": {
                                "id": "45c40bce-f5d6-4270-9d19-e3b4d2f41927",
                                "key": "mina_happy",
                                "path": "assets/standing/mina_happy.png",
                                "settings": null
                            }
                        },
                        {
                            "type": "standing",
                            "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                            "emotion": "surprised",
                            "asset": {
                                "id": "eb32f656-9335-471f-8ee3-1c8786fd205f",
                                "key": "yuna_surprised",
                                "path": "assets/standing/yuna_surprised.png",
                                "settings": null
                            }
                        },
                        {
                            "type": "motion",
                            "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                            "name": "jump"
                        },
                        {
                            "type": "effect",
                            "name": "flash",
                            "param": null
                        }
                    ]
                },
                {
                    "kind": "dialogue",
                    "speaker": "3494af25-4350-4223-884f-5d729dca2641",
                    "text": "어, 이 친구는 누구야?",
                    "cues": [
                        {
                            "type": "standing",
                            "characterId": "3494af25-4350-4223-884f-5d729dca2641",
                            "emotion": "normal",
                            "asset": {
                                "id": "d238f7e2-a3e7-4fcd-a5d8-cd8d6922bc05",
                                "key": "mina_normal",
                                "path": "assets/standing/mina_normal.png",
                                "settings": null
                            }
                        }
                    ]
                }
            ]
        }
    ],
    "state": {
        "sessionId": "53e28b3e-fd4c-4eeb-98ec-b16f7fcff69a",
        "locationId": "8a7698b0-aee9-4650-bea5-79c8614f57fc",
        "mapId": "46b8209d-4868-4d3d-bdd4-10b6b2550961",
        "zoneId": "ba6786a9-20a8-449f-93a6-7b0759a3ae1d",
        "gameTime": 0,
        "weatherId": "5caf5a44-314c-4890-b55b-40616ac0b4ef",
        "timePeriodId": "b3830634-2799-46a7-a082-9732dc61344b",
        "currentSceneId": "9dd48c11-c807-4690-847c-e8c872617601",
        "activeNpcId": null,
        "availableChoices": [],
        "flags": {
            "confidence": 10,
            "energy": 100,
            "festival_seen": false,
            "met_yuna": false,
            "money": 50,
            "rooftop_visited": false
        },
        "inventory": {},
        "relationships": {
            "3494af25-4350-4223-884f-5d729dca2641>bf4bc3eb-6fd1-47e8-9c37-429b1e167980": {
                "tags": [
                    "소꿉친구",
                    "라이벌"
                ],
                "tension": 20
            },
            "bf4bc3eb-6fd1-47e8-9c37-429b1e167980>3494af25-4350-4223-884f-5d729dca2641": {
                "affection": 15,
                "tags": [
                    "소꿉친구"
                ],
                "tension": 10
            },
            "bf4bc3eb-6fd1-47e8-9c37-429b1e167980>player": {
                "affection": 0,
                "tags": [],
                "trust": 5
            },
            "d24beaaa-687f-4029-a0ed-af9e31f77626>player": {
                "affection": 0,
                "tags": [],
                "trust": 0
            }
        },
        "stage": {
            "background": {
                "id": "04de1e8d-96a7-4d01-a911-18886127b974",
                "key": "bg_class",
                "path": "assets/background/bg_class.png",
                "settings": null
            },
            "bgm": {
                "asset": {
                    "id": "719f6a1c-6a68-495c-8fb4-ac5dacbbcdaa",
                    "key": "bgm.peaceful",
                    "path": "assets/audio/bgm_peaceful.wav",
                    "settings": {
                        "loop": true,
                        "volume": 0.5
                    }
                },
                "mood": "peaceful"
            },
            "standings": {
                "3494af25-4350-4223-884f-5d729dca2641": {
                    "asset": {
                        "id": "d238f7e2-a3e7-4fcd-a5d8-cd8d6922bc05",
                        "key": "mina_normal",
                        "path": "assets/standing/mina_normal.png",
                        "settings": null
                    },
                    "emotion": "normal"
                },
                "bf4bc3eb-6fd1-47e8-9c37-429b1e167980": {
                    "asset": {
                        "id": "eb32f656-9335-471f-8ee3-1c8786fd205f",
                        "key": "yuna_surprised",
                        "path": "assets/standing/yuna_surprised.png",
                        "settings": null
                    },
                    "emotion": "surprised"
                }
            }
        },
        "presentNpcs": []
    }
}
```

정적 데이터: 

```json
{
    "assets": {
        "audio": [
            {
                "category": "sfx",
                "characterId": null,
                "id": "a15003d9-7ec8-4a5c-b5a9-a25b413da057",
                "key": "sfx.door.1",
                "path": "assets/audio/door_1.wav",
                "settings": {
                    "volume": 0.8
                },
                "tag": "문 소리",
                "transcript": null
            },
            {
                "category": "sfx",
                "characterId": null,
                "id": "008c53b6-d7f1-4263-ad5e-8cc6b57ea651",
                "key": "sfx.door.2",
                "path": "assets/audio/door_2.wav",
                "settings": {
                    "volume": 0.8
                },
                "tag": "문 소리",
                "transcript": null
            },
            {
                "category": "sfx",
                "characterId": null,
                "id": "ae3efa11-bb8f-4f96-8197-33cf864ec54b",
                "key": "sfx.bird.1",
                "path": "assets/audio/bird_1.wav",
                "settings": {
                    "volume": 0.6
                },
                "tag": "새 소리",
                "transcript": null
            },
            {
                "category": "sfx",
                "characterId": null,
                "id": "c4b5c63c-0874-4b1d-989a-73866f4b65be",
                "key": "sfx.bird.2",
                "path": "assets/audio/bird_2.wav",
                "settings": {
                    "volume": 0.6
                },
                "tag": "새 소리",
                "transcript": null
            },
            {
                "category": "sfx",
                "characterId": null,
                "id": "5163806a-9686-4de5-8ae1-69b0030b47a7",
                "key": "sfx.bell",
                "path": "assets/audio/bell.wav",
                "settings": {
                    "volume": 0.7
                },
                "tag": "종소리",
                "transcript": null
            },
            {
                "category": "bgm",
                "characterId": null,
                "id": "fd524db7-df00-434b-9377-98553ea7f5d0",
                "key": "bgm.default",
                "path": "assets/audio/bgm_default.wav",
                "settings": {
                    "loop": true,
                    "volume": 0.5
                },
                "tag": "default",
                "transcript": null
            },
            {
                "category": "bgm",
                "characterId": null,
                "id": "719f6a1c-6a68-495c-8fb4-ac5dacbbcdaa",
                "key": "bgm.peaceful",
                "path": "assets/audio/bgm_peaceful.wav",
                "settings": {
                    "loop": true,
                    "volume": 0.5
                },
                "tag": "peaceful",
                "transcript": null
            },
            {
                "category": "bgm",
                "characterId": null,
                "id": "1bdbd697-84ca-4e3d-9b36-09a51c0a20f5",
                "key": "bgm.romance",
                "path": "assets/audio/bgm_romance.wav",
                "settings": {
                    "loop": true,
                    "volume": 0.5
                },
                "tag": "romance",
                "transcript": null
            },
            {
                "category": "bgm",
                "characterId": null,
                "id": "bac06c2f-718e-4006-b9f4-bc823af77085",
                "key": "bgm.tense",
                "path": "assets/audio/bgm_tense.wav",
                "settings": {
                    "loop": true,
                    "volume": 0.5
                },
                "tag": "tense",
                "transcript": null
            },
            {
                "category": "voice",
                "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                "id": "dc6dd2a1-9b77-42be-a4ea-2e2bc5eb558d",
                "key": "yuna_greet",
                "path": "assets/audio/voice_yuna_greet.wav",
                "settings": {
                    "volume": 0.9
                },
                "tag": null,
                "transcript": "아, 안녕! 오늘도 일찍 왔네?"
            }
        ],
        "backgrounds": [
            {
                "id": "944593e0-7c5f-4da1-b462-7344df2e394d",
                "key": "bg_default",
                "locationId": null,
                "path": "assets/background/bg_default.png",
                "settings": null,
                "timeId": null,
                "weatherId": null
            },
            {
                "id": "f575a9d8-c775-4ea7-8455-c710af583e80",
                "key": "bg_gate",
                "locationId": "bb0b4b4d-6994-41c9-be16-e82cfba05455",
                "path": "assets/background/bg_gate.png",
                "settings": null,
                "timeId": null,
                "weatherId": null
            },
            {
                "id": "e31190c2-7409-4216-8375-83b60d14048e",
                "key": "bg_ground",
                "locationId": "42f56e50-8a98-41bb-ac78-eaed0ed21617",
                "path": "assets/background/bg_ground.png",
                "settings": null,
                "timeId": null,
                "weatherId": null
            },
            {
                "id": "04de1e8d-96a7-4d01-a911-18886127b974",
                "key": "bg_class",
                "locationId": "8a7698b0-aee9-4650-bea5-79c8614f57fc",
                "path": "assets/background/bg_class.png",
                "settings": null,
                "timeId": null,
                "weatherId": null
            },
            {
                "id": "ca5c09b0-bf81-4f6d-bd8b-f6cbca1988d6",
                "key": "bg_class_evening",
                "locationId": "8a7698b0-aee9-4650-bea5-79c8614f57fc",
                "path": "assets/background/bg_class_evening.png",
                "settings": null,
                "timeId": "229f88d8-67d9-4b18-979f-45a674a7b716",
                "weatherId": null
            },
            {
                "id": "2a8ae871-d3ad-499f-acda-10a90f8db424",
                "key": "bg_class_rain",
                "locationId": "8a7698b0-aee9-4650-bea5-79c8614f57fc",
                "path": "assets/background/bg_class_rain.png",
                "settings": null,
                "timeId": null,
                "weatherId": "6e18dc53-0061-4c11-a89c-60bc0256c12f"
            },
            {
                "id": "a1fa122e-fb38-4599-86a1-4f3749301fc1",
                "key": "bg_library_a",
                "locationId": "90e79e62-ba6d-4496-a5f1-e66e4e6a81a4",
                "path": "assets/background/bg_library_a.png",
                "settings": null,
                "timeId": null,
                "weatherId": null
            },
            {
                "id": "82af577f-1741-49c4-bd47-0b5a21a78546",
                "key": "bg_library_b",
                "locationId": "90e79e62-ba6d-4496-a5f1-e66e4e6a81a4",
                "path": "assets/background/bg_library_b.png",
                "settings": null,
                "timeId": null,
                "weatherId": null
            },
            {
                "id": "892eb1d2-f4f8-475c-b5b5-66d6757d3243",
                "key": "bg_roof",
                "locationId": "69e1694f-e70e-471c-bf24-b026a92172f0",
                "path": "assets/background/bg_roof.png",
                "settings": null,
                "timeId": null,
                "weatherId": null
            },
            {
                "id": "bc725ca8-a9f8-4517-81bd-cd0660b9daf8",
                "key": "bg_roof_night",
                "locationId": "69e1694f-e70e-471c-bf24-b026a92172f0",
                "path": "assets/background/bg_roof_night.png",
                "settings": null,
                "timeId": "b3830634-2799-46a7-a082-9732dc61344b",
                "weatherId": null
            },
            {
                "id": "efbae3f6-5c55-4ba3-a044-861a455c63d9",
                "key": "bg_cafe",
                "locationId": "1faa481b-0078-48c1-b651-121bdfffe47f",
                "path": "assets/background/bg_cafe.png",
                "settings": null,
                "timeId": null,
                "weatherId": null
            }
        ],
        "cgs": [
            {
                "characterId": null,
                "description": "문화제 밤하늘에 터지는 불꽃놀이. 두 사람이 함께 올려다보는 장면에 쓴다.",
                "id": "54ba6d5f-6683-4f2b-92ed-8bdb0d44a50f",
                "key": "festival_fireworks",
                "path": "assets/cg/festival_fireworks.png",
                "settings": null
            },
            {
                "characterId": null,
                "description": "해피 엔딩의 고백 장면. 엔딩에서만 쓴다.",
                "id": "89b99c0c-884d-4cc4-afb9-8f8667d04c1e",
                "key": "confession",
                "path": "assets/cg/confession.png",
                "settings": null
            }
        ],
        "characters": [
            {
                "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                "emotion": "normal",
                "id": "6447a355-5248-441c-852e-dbe4d3906e45",
                "key": "yuna_normal",
                "path": "assets/standing/yuna_normal.png",
                "settings": null,
                "usage": "standing"
            },
            {
                "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                "emotion": "happy",
                "id": "b11ad563-8b3e-4b8d-b186-da25f2316dcc",
                "key": "yuna_happy",
                "path": "assets/standing/yuna_happy.png",
                "settings": null,
                "usage": "standing"
            },
            {
                "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                "emotion": "surprised",
                "id": "eb32f656-9335-471f-8ee3-1c8786fd205f",
                "key": "yuna_surprised",
                "path": "assets/standing/yuna_surprised.png",
                "settings": null,
                "usage": "standing"
            },
            {
                "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                "emotion": "shy",
                "id": "5939f5f0-b442-4907-a367-91391d3326de",
                "key": "yuna_shy",
                "path": "assets/standing/yuna_shy.png",
                "settings": null,
                "usage": "standing"
            },
            {
                "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                "emotion": "angry",
                "id": "2918f199-536d-4f6d-8ebe-db911d751788",
                "key": "yuna_angry",
                "path": "assets/standing/yuna_angry.png",
                "settings": null,
                "usage": "standing"
            },
            {
                "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                "emotion": "happy",
                "id": "271df3d4-9124-41d0-a4ec-aeab8f92e0a3",
                "key": "yuna_happy_2",
                "path": "assets/standing/yuna_happy_2.png",
                "settings": null,
                "usage": "standing"
            },
            {
                "characterId": "d24beaaa-687f-4029-a0ed-af9e31f77626",
                "emotion": "normal",
                "id": "ae093368-2b84-4bbe-9cff-ab6ed77415cf",
                "key": "haru_normal",
                "path": "assets/standing/haru_normal.png",
                "settings": null,
                "usage": "standing"
            },
            {
                "characterId": "d24beaaa-687f-4029-a0ed-af9e31f77626",
                "emotion": "happy",
                "id": "04d5bd4d-e56b-48b8-a5ee-306630ceb4fb",
                "key": "haru_happy",
                "path": "assets/standing/haru_happy.png",
                "settings": null,
                "usage": "standing"
            },
            {
                "characterId": "3494af25-4350-4223-884f-5d729dca2641",
                "emotion": "normal",
                "id": "d238f7e2-a3e7-4fcd-a5d8-cd8d6922bc05",
                "key": "mina_normal",
                "path": "assets/standing/mina_normal.png",
                "settings": null,
                "usage": "standing"
            },
            {
                "characterId": "3494af25-4350-4223-884f-5d729dca2641",
                "emotion": "happy",
                "id": "45c40bce-f5d6-4270-9d19-e3b4d2f41927",
                "key": "mina_happy",
                "path": "assets/standing/mina_happy.png",
                "settings": null,
                "usage": "standing"
            },
            {
                "characterId": "3494af25-4350-4223-884f-5d729dca2641",
                "emotion": "angry",
                "id": "aef90fd6-3a24-405e-847b-ce28b63c321b",
                "key": "mina_angry",
                "path": "assets/standing/mina_angry.png",
                "settings": null,
                "usage": "standing"
            },
            {
                "characterId": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
                "emotion": null,
                "id": "4ae4f7f7-ba16-463f-90a0-0642a0be0bd9",
                "key": "map_yuna",
                "path": "assets/map/map_yuna.png",
                "settings": null,
                "usage": "map"
            },
            {
                "characterId": "d24beaaa-687f-4029-a0ed-af9e31f77626",
                "emotion": null,
                "id": "98d8c51d-4be1-44b4-9c94-fb43a9750540",
                "key": "map_haru",
                "path": "assets/map/map_haru.png",
                "settings": null,
                "usage": "map"
            },
            {
                "characterId": "3494af25-4350-4223-884f-5d729dca2641",
                "emotion": null,
                "id": "07f10f64-040a-42ca-918d-767f83c83898",
                "key": "map_mina",
                "path": "assets/map/map_mina.png",
                "settings": null,
                "usage": "map"
            }
        ],
        "other": []
    },
    "characters": [
        {
            "avatar": null,
            "description": "같은 반 반장. 밝고 활발하며 누구에게나 친절하지만, 속마음은 잘 드러내지 않는다. 플레이어에게 호기심이 있다.",
            "firstMessage": "아, 안녕! 오늘도 일찍 왔네?",
            "id": "bf4bc3eb-6fd1-47e8-9c37-429b1e167980",
            "name": "유나",
            "settings": {
                "color": "#f48fb1"
            },
            "tags": "[\"classmate\",\"heroine\"]"
        },
        {
            "avatar": null,
            "description": "도서부의 한 학년 선배. 말수가 적고 차분하다. 책 이야기를 할 때만 눈이 반짝인다.",
            "firstMessage": "...조용히 해 줄래? 아, 미안. 책 찾으러 왔어?",
            "id": "d24beaaa-687f-4029-a0ed-af9e31f77626",
            "name": "하루",
            "settings": {
                "color": "#90caf9"
            },
            "tags": "[\"senior\",\"quiet\"]"
        },
        {
            "avatar": null,
            "description": "카페에서 아르바이트하는 유나의 소꿉친구. 장난기 많고 솔직하다. 유나와는 은근한 라이벌.",
            "firstMessage": "어서 와! 오늘은 뭐 마실래?",
            "id": "3494af25-4350-4223-884f-5d729dca2641",
            "name": "미나",
            "settings": {
                "color": "#ffcc80"
            },
            "tags": "[\"friend\",\"rival\"]"
        }
    ],
    "connections": [
        {
            "fromId": "bb0b4b4d-6994-41c9-be16-e82cfba05455",
            "settings": {},
            "toId": "1faa481b-0078-48c1-b651-121bdfffe47f",
            "travelSeconds": 900
        },
        {
            "fromId": "bb0b4b4d-6994-41c9-be16-e82cfba05455",
            "settings": {},
            "toId": "42f56e50-8a98-41bb-ac78-eaed0ed21617",
            "travelSeconds": 300
        },
        {
            "fromId": "bb0b4b4d-6994-41c9-be16-e82cfba05455",
            "settings": {},
            "toId": "8a7698b0-aee9-4650-bea5-79c8614f57fc",
            "travelSeconds": 600
        },
        {
            "fromId": "42f56e50-8a98-41bb-ac78-eaed0ed21617",
            "settings": {},
            "toId": "bb0b4b4d-6994-41c9-be16-e82cfba05455",
            "travelSeconds": 300
        },
        {
            "fromId": "8a7698b0-aee9-4650-bea5-79c8614f57fc",
            "settings": {},
            "toId": "69e1694f-e70e-471c-bf24-b026a92172f0",
            "travelSeconds": 240
        },
        {
            "fromId": "8a7698b0-aee9-4650-bea5-79c8614f57fc",
            "settings": {},
            "toId": "90e79e62-ba6d-4496-a5f1-e66e4e6a81a4",
            "travelSeconds": 180
        },
        {
            "fromId": "8a7698b0-aee9-4650-bea5-79c8614f57fc",
            "settings": {},
            "toId": "bb0b4b4d-6994-41c9-be16-e82cfba05455",
            "travelSeconds": 600
        },
        {
            "fromId": "90e79e62-ba6d-4496-a5f1-e66e4e6a81a4",
            "settings": {},
            "toId": "8a7698b0-aee9-4650-bea5-79c8614f57fc",
            "travelSeconds": 180
        },
        {
            "fromId": "69e1694f-e70e-471c-bf24-b026a92172f0",
            "settings": {},
            "toId": "8a7698b0-aee9-4650-bea5-79c8614f57fc",
            "travelSeconds": 240
        },
        {
            "fromId": "1faa481b-0078-48c1-b651-121bdfffe47f",
            "settings": {},
            "toId": "bb0b4b4d-6994-41c9-be16-e82cfba05455",
            "travelSeconds": 900
        }
    ],
    "endings": [
        {
            "description": "소중한 친구로 남은 결말.",
            "endingKey": "friends",
            "id": "20403fa8-b1d4-4713-ad5f-92aa3d605444",
            "title": "친구로서"
        },
        {
            "description": "마음이 이어진 결말.",
            "endingKey": "good",
            "id": "68281fff-b2b2-4ab9-b981-0646485fad6a",
            "title": "해피 엔딩"
        }
    ],
    "game": {
        "coverImage": null,
        "customCss": null,
        "description": "하루하루를 보내며 인연을 만들어 가는 학원 연애 시뮬레이션. 엔진의 모든 기능을 시험해 볼 수 있는 기본 템플릿 패키지.",
        "id": "5f7e42c7-058a-4c71-9193-506637d9c503",
        "settings": "{\"startWeatherId\":\"5caf5a44-314c-4890-b55b-40616ac0b4ef\",\"startMinute\":480,\"dangerEncounterScale\":0.25}",
        "title": "c"
    },
    "interactables": [
        {
            "id": "aa800b1a-625e-4e9d-858c-5e1c71a7522b",
            "locationId": "90e79e62-ba6d-4496-a5f1-e66e4e6a81a4",
            "name": "오래된 책장"
        },
        {
            "id": "6e3caeef-8f35-456c-8d0a-b7f4b9d2e95f",
            "locationId": "bb0b4b4d-6994-41c9-be16-e82cfba05455",
            "name": "자판기"
        }
    ],
    "items": [
        {
            "consumable": false,
            "description": "언젠가 전하고 싶은 편지.",
            "icon": null,
            "id": "14dffe33-1c57-4d7d-867b-e649df4119bc",
            "isKey": true,
            "isUnique": true,
            "kind": "key",
            "maxStack": 1,
            "name": "러브레터",
            "settings": {},
            "usable": false
        },
        {
            "consumable": false,
            "description": "옥상 문을 여는 열쇠.",
            "icon": null,
            "id": "f538e037-09eb-4ad6-988d-3bac1016a997",
            "isKey": true,
            "isUnique": true,
            "kind": "key",
            "maxStack": 1,
            "name": "옥상 열쇠",
            "settings": {},
            "usable": false
        },
        {
            "consumable": true,
            "description": "달콤한 초콜릿. 선물하기 좋다.",
            "icon": null,
            "id": "bd35eb4b-2db5-4366-b4e2-d87f06721af1",
            "isKey": false,
            "isUnique": false,
            "kind": "gift",
            "maxStack": 10,
            "name": "초콜릿",
            "settings": {},
            "usable": true
        },
        {
            "consumable": true,
            "description": "자판기에서 뽑은 커피.",
            "icon": null,
            "id": "d7a0d797-5b70-4b87-bdb5-a0733eb048f5",
            "isKey": false,
            "isUnique": false,
            "kind": "consumable",
            "maxStack": 10,
            "name": "캔커피",
            "settings": {},
            "usable": true
        }
    ],
    "locations": [
        {
            "description": "학교의 정문. 등하교하는 학생들로 붐빈다.",
            "id": "bb0b4b4d-6994-41c9-be16-e82cfba05455",
            "name": "정문",
            "settings": {},
            "zoneId": "155e4168-459e-4648-9d19-4db170bbb0be"
        },
        {
            "description": "넓은 운동장. 동아리 활동 소리가 들린다.",
            "id": "42f56e50-8a98-41bb-ac78-eaed0ed21617",
            "name": "운동장",
            "settings": {},
            "zoneId": "155e4168-459e-4648-9d19-4db170bbb0be"
        },
        {
            "description": "새 학기의 교실. 창가 자리에 햇빛이 든다.",
            "id": "8a7698b0-aee9-4650-bea5-79c8614f57fc",
            "name": "교실 2-A",
            "settings": {},
            "zoneId": "ba6786a9-20a8-449f-93a6-7b0759a3ae1d"
        },
        {
            "description": "조용한 도서관. 오래된 책 냄새가 난다.",
            "id": "90e79e62-ba6d-4496-a5f1-e66e4e6a81a4",
            "name": "도서관",
            "settings": {},
            "zoneId": "ba6786a9-20a8-449f-93a6-7b0759a3ae1d"
        },
        {
            "description": "평소에는 잠겨 있는 옥상. 바람이 시원하다.",
            "id": "69e1694f-e70e-471c-bf24-b026a92172f0",
            "name": "옥상",
            "settings": {},
            "zoneId": "ba6786a9-20a8-449f-93a6-7b0759a3ae1d"
        },
        {
            "description": "학교 근처의 아늑한 카페.",
            "id": "1faa481b-0078-48c1-b651-121bdfffe47f",
            "name": "카페",
            "settings": {},
            "zoneId": "ff2ef641-261b-4b50-a3d6-e3fdb2d179e5"
        }
    ],
    "maps": [
        {
            "dangerLevel": 1,
            "id": "46b8209d-4868-4d3d-bdd4-10b6b2550961",
            "name": "선라이즈 고등학교",
            "settings": {
                "theme": "school"
            }
        }
    ],
    "relationshipAxes": [
        "affection",
        "trust",
        "tension"
    ],
    "scenes": [
        {
            "choices": [],
            "description": "새 학기의 첫날 아침, 교실. 창밖으로 벚꽃이 흩날린다. 새로운 반, 새로운 얼굴들 속에서 한 사람이 눈에 들어온다.",
            "id": "9dd48c11-c807-4690-847c-e8c872617601",
            "isEnding": false,
            "name": "새 학기의 첫날",
            "sceneKey": "intro",
            "settings": null
        },
        {
            "choices": [],
            "description": "잠겨 있던 문 너머로 탁 트인 하늘이 펼쳐진다. 도시가 한눈에 내려다보인다.",
            "id": "7011493d-5b48-4ee5-b661-a31f8b477adf",
            "isEnding": false,
            "name": "옥상의 풍경",
            "sceneKey": "rooftop_view",
            "settings": null
        },
        {
            "choices": [],
            "description": "점심 방송에서 이번 주말 문화제 소식이 들린다. 같이 갈 사람이 떠오른다.",
            "id": "e880ac93-b734-4d32-97bd-3f6e86597679",
            "isEnding": false,
            "name": "문화제 소식",
            "sceneKey": "festival",
            "settings": null
        },
        {
            "choices": [],
            "description": "유나는 환하게 웃으며 고개를 끄덕인다. \"약속이다!\"",
            "id": "86b3d880-168b-4990-a27f-c865afca1e45",
            "isEnding": false,
            "name": "문화제 약속",
            "sceneKey": "festival_together",
            "settings": null
        },
        {
            "choices": [],
            "description": "어느새 유나와 자연스럽게 이야기하는 사이가 되었다. 마음을 전할 때일지도 모른다.",
            "id": "ca84afdf-d401-43d2-b8aa-36024d11d49d",
            "isEnding": false,
            "name": "가까워진 거리",
            "sceneKey": "yuna_close",
            "settings": null
        },
        {
            "choices": [],
            "description": "벚꽃이 흩날리는 하굣길, 두 사람은 나란히 걸었다.",
            "id": "cdea27ce-790f-4836-a317-df5a47c75898",
            "isEnding": true,
            "name": "해피 엔딩",
            "sceneKey": "good_ending",
            "settings": null
        },
        {
            "choices": [],
            "description": "서로를 소중히 여기는 친구로 남은 두 사람의 이야기.",
            "id": "d8f9aad7-34c2-4e48-8671-25236306c890",
            "isEnding": true,
            "name": "친구로서",
            "sceneKey": "friends_ending",
            "settings": null
        }
    ],
    "stageCatalog": {
        "bgmMoods": [
            "default",
            "peaceful",
            "upbeat",
            "romance",
            "tense",
            "battle"
        ],
        "motions": [
            "jump",
            "bounce",
            "shake",
            "nod",
            "sway",
            "shrink",
            "slide_in",
            "slide_out"
        ],
        "screenEffects": [
            "shake",
            "flash",
            "blackout",
            "fade_in",
            "fireworks",
            "petals",
            "snow",
            "rain",
            "fog",
            "clear"
        ]
    },
    "template": {
        "apiVersion": "1",
        "capabilities": {
            "aiNarration": true,
            "choices": true,
            "freeText": true,
            "inventory": true,
            "location": true,
            "map": true,
            "time": true
        },
        "id": "dating-sim",
        "name": "Dating Sim",
        "templateVersion": "1.1.0"
    },
    "timePeriods": [
        {
            "endMinute": 480,
            "id": "75e83a1f-fd07-4303-8dc3-2139e61134f1",
            "name": "새벽",
            "startMinute": 300
        },
        {
            "endMinute": 720,
            "id": "73fa91a3-f958-4691-b4a0-0ed0b7e3be7e",
            "name": "아침",
            "startMinute": 480
        },
        {
            "endMinute": 1080,
            "id": "313f3028-5e30-479b-aea0-ce2ffe9b543b",
            "name": "낮",
            "startMinute": 720
        },
        {
            "endMinute": 1260,
            "id": "229f88d8-67d9-4b18-979f-45a674a7b716",
            "name": "저녁",
            "startMinute": 1080
        },
        {
            "endMinute": 300,
            "id": "b3830634-2799-46a7-a082-9732dc61344b",
            "name": "밤",
            "startMinute": 1260
        }
    ],
    "variables": [
        {
            "default": 10,
            "description": "자신감",
            "key": "confidence",
            "max": 100,
            "min": 0,
            "type": "number"
        },
        {
            "default": 100,
            "description": "체력",
            "key": "energy",
            "max": 100,
            "min": 0,
            "type": "number"
        },
        {
            "default": false,
            "description": "문화제 방송을 들었는가",
            "key": "festival_seen",
            "max": null,
            "min": null,
            "type": "boolean"
        },
        {
            "default": false,
            "description": "유나와 만났는가",
            "key": "met_yuna",
            "max": null,
            "min": null,
            "type": "boolean"
        },
        {
            "default": 50,
            "description": "용돈",
            "key": "money",
            "max": 99999,
            "min": 0,
            "type": "number"
        },
        {
            "default": false,
            "description": "옥상에 가 보았는가",
            "key": "rooftop_visited",
            "max": null,
            "min": null,
            "type": "boolean"
        }
    ],
    "weatherTypes": [
        {
            "id": "5caf5a44-314c-4890-b55b-40616ac0b4ef",
            "name": "맑음"
        },
        {
            "id": "fe6c2565-3053-4228-800f-82ab9bf8725c",
            "name": "흐림"
        },
        {
            "id": "6e18dc53-0061-4c11-a89c-60bc0256c12f",
            "name": "비"
        }
    ],
    "zones": [
        {
            "id": "155e4168-459e-4648-9d19-4db170bbb0be",
            "mapId": "46b8209d-4868-4d3d-bdd4-10b6b2550961",
            "name": "교정",
            "settings": {},
            "tags": [
                "outdoor"
            ]
        },
        {
            "id": "ba6786a9-20a8-449f-93a6-7b0759a3ae1d",
            "mapId": "46b8209d-4868-4d3d-bdd4-10b6b2550961",
            "name": "교실동",
            "settings": {},
            "tags": [
                "indoor"
            ]
        },
        {
            "id": "ff2ef641-261b-4b50-a3d6-e3fdb2d179e5",
            "mapId": "46b8209d-4868-4d3d-bdd4-10b6b2550961",
            "name": "시내",
            "settings": {},
            "tags": [
                "outdoor"
            ]
        }
    ]
}
```

#### Assets의 단위

##### 이미지 에셋

- Background CG
- Event CG
- Character Standing
- Map Character


##### 오디오 에셋

- Voice: 캐릭터 대사에 연결되는 음성파일
- BGM: 상황이나 장면에 따라 재생되는 배경음악
- Sound Effect: 버튼, 발걸음, 문 여는 소리 등 효과음

### 위젯

## IPC 설계

## 렌더링 및 UI 설계

UI에서는 Svelte5를 사용한다.

게임 엔진에서 그래픽을 다룰 때는 Pixi.js를 사용.

## 테스트 및 품질 보증

| 테스트       | 방식               | 확인하는 것                                 |
| ------------ | ------------------ | ------------------------------------------- |
| UI           | Vitest             | 화면이 올바르게 표시되고 입력에 반응하는가? |
| Rust 단위    | cargo test         | 로직이 예상대로 계산되는가?                 |
| DB           | SQLite             | 데이터가 정확하게 저장되고 조회되는가?      |
| Tauri 커맨드 | Mock + Integration | 프론트엔드와 Rust 사이의 계약이 맞는가?     |
| E2E          | WebdriverIO        | 실제 앱에서 전체 기능이 작동하는가?         |

### UI 테스트 - Svelte5 + Vitest

| 도구                           | 역할                    |
| ------------------------------ | ----------------------- |
| Vitest                         | 테스트 실행             |
| @testing-library/svelte Svelte | 컴포넌트 렌더링 및 조작 |
| @testing-library/user-event    | 사용자 입력 시뮬레이션  |
| jsdom                          | 브라우저 DOM 환경 제공  |

### 테스트 명령어 정의

#### 프론트엔드 테스트

npm run test:run

#### Rust 테스트

cd src-tauri
cargo test

##### Rust 코드 스타일 검사

cargo fmt --check

##### Rust 정적 분석

cargo clippy

## 마일스톤 일정

- [~] Phase1: 기본 아키텍처 구축
    - [ ] conv 테이블과 game 테이블의 통합작업
    - [ ] UI 구축
    - [ ] 게임 에셋 매니저 구축
    - [ ] 다국어 환경 기반 마련
- [ ] Phase2: DB 통합테스트 구성
- [ ] Phase3: 외부 API 연결 작업
- [ ] Phase4: 모바일 UI작업
- [ ] Phase5: 게임 엔진 템플릿 세분화


## 잠재적 문제

- 프로그램 인증서 발급
- 유지보수 (납품하면 끝이 아니니까 어렵네)
    - 해킹 가능성?
    - 보안 및 스크립트 격리

