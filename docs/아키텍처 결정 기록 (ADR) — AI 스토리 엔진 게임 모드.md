# 아키텍처 결정 기록 (ADR) — AI 스토리 엔진 게임 모드

> 기준: 지금까지의 설계 논의와 구현. 상태 표기: **Accepted**(결정·구현됨), **Proposed**(결정됨·구현 전), **Deferred**(보류). 컴파일 검증은 사용자가 단계적으로 진행 중이며, 구현 여부는 각 ADR의 결과 항목에 적었다.

## 0. 배경과 용어

- **제품**: Tauri v2(Windows/Android) 로컬 앱. 프론트 Svelte+TS, 백엔드 Rust, DB SQLite(sqlx). 서버 없음, API 키는 사용자 소유. 기능은 AI 채팅과 게임 모드(사용자들이 만든 게임을 가져와 실행).
- **게임 코드**는 남이 만든 HTML/JS이며 iframe에서 재생한다.
- **세션**: 한 번의 플레이 전체(책 한 권). **책갈피(세이브)**: 세션의 한 시점(일지 seq + 상태 스냅샷).
- **정의(Definition)**: 제작자가 만든 정적 데이터. **상태(State)**: 플레이 중 바뀌는 데이터.
- **템플릿**: 엔진의 시스템 규칙(capabilities). **템플릿 패키지**: 새 게임의 시작 데이터(.tgame).
- **시스템**: 이벤트에 반응해 상태를 바꾸는 엔진 모듈(movement, time, events, scene 등).

## ADR 목록

| ID | 제목 | 상태 |
| --- | --- | --- |
| 001 | 신뢰 경계와 책임 분리 | Accepted |
| 002 | 게임 코드 격리 (iframe + game:// 프로토콜) | Accepted(SDK/브릿지는 Deferred) |
| 003 | API 키·LLM 호출은 Rust 전담, 게임은 프롬프트를 못 바꾼다 | Accepted |
| 004 | 게임 패키지(.tgame)와 import/export | Accepted |
| 005 | 정의/상태 분리와 로드 시 일괄 적재 | Accepted |
| 006 | 이벤트 구동 시스템과 단일 writer | Accepted |
| 007 | 선언형 조건·효과, 못 읽으면 막는다 | Accepted |
| 008 | 공간·시간 모델 | Accepted |
| 009 | 이벤트 종류·실행 순서 | Accepted |
| 010 | 세션·책갈피·일지·분기 | Accepted |
| 011 | 씬 모델 | Accepted |
| 012 | AI 대화 구조 | Accepted |
| 013 | AI 출력 태그 프로토콜 | Accepted |
| 014 | 인벤토리·관계·NPC 위치 | Accepted |
| 015 | 정의 편집·무결성·화면용 데이터 | Accepted |
| 016 | 템플릿과 템플릿 패키지 | Accepted |
| 017 | 코드 구조와 관례 | Accepted |
| 018 | 에셋 서빙과 연출 | Accepted("갱신" 절이 확정, 후속 있음) |
| 019 | StoryPlan(각본가) | Deferred — 구현이 미병합 브랜치에 있음(`ADR 미결 정리.md`) |

## ADR-001 신뢰 경계와 책임 분리

**상태** Accepted

**배경** 임포트한 게임 코드는 무엇이든 실행할 수 있다고 가정해야 한다. 동시에 로컬 앱이라 서버 쪽 방어선이 없다.

**결정**

- 세 계층으로 나눈다: **Rust(신뢰)** — 엔진, DB, LLM 호출, API 키, 패키지 검증 / **Svelte 호스트(반신뢰)** — UI, 브릿지(메시지 검증·전달) / **iframe(비신뢰)** — 게임 스크립트와 SDK.
- SDK는 개발 편의용이고 보안은 호스트와 Rust가 책임진다. 모든 검증은 Rust에서 다시 한다.
- 게임 식별자(game\_id, session\_id)는 iframe이 보내지 않는다. Rust가 현재 열린 게임(active\_archive)과 실행 중 런타임에서 직접 읽는다.

**결과**

- 호스트 창에만 허용할 커맨드(save\_package, delete\_game, import/export 등)와 게임이 부를 수 있는 커맨드(dispatch\_command, npc\_say, get\_game\_view 등)를 Tauri capabilities에서 나눠야 한다(후속).

## ADR-002 게임 코드 격리 (iframe + game:// 프로토콜)

**상태** Accepted — SDK와 호스트 브릿지는 보류(Deferred, 프로젝트 전반 ADR-003)

**결정**

- iframe은 `sandbox=allow-scripts`만 주고 `allow-same-origin`은 주지 않는다(불투명 origin → 부모 DOM·스토리지·Tauri 전역 접근 차단).
- 게임 파일은 커스텀 프로토콜 `game://localhost/<game_id>/<path>`로 서빙하고, CSP는 응답 헤더로 강제한다(default-src none, connect-src none, form-action none 등).
- 경로는 canonicalize 후 게임 폴더 하위인지 확인한다(경로 탈출·심볼릭 링크 방지).
- 통신은 postMessage 요청/응답만 쓰고, 호스트는 허용 목록 방식으로 메시지 타입과 페이로드를 검증하며 rate limit을 건다.

**결과**

- OS별 WebView(WebView2, WKWebView 등)의 CSP/sandbox 동작이 달라 플랫폼별 테스트가 필요하다.
- 무한 루프 같은 CPU 남용은 완전히 막기 어렵다(게임 재시작 버튼 상시 제공). 키 입력 UI는 호스트에만 둔다(피싱 방지).

## ADR-003 API 키·LLM 호출은 Rust 전담, 게임은 프롬프트를 못 바꾼다

**상태** Accepted

**결정**

- API 키는 keyring에 저장하고 프론트로 올리지 않는다. NPC 대화는 ChatManager.game\_chat이 프롬프트 조립·어댑터 호출을 맡는다.
- 게임이 넘기는 것은 입력 텍스트뿐이다. 모델·프로필은 앱 설정의 선택값을 공용으로 쓴다.
- 앱 고정 규칙(GAME\_SYSTEM\_RULES)을 항상 게임 데이터보다 앞에 둔다(임포트한 설명문이 규칙을 덮어쓰지 못하게). 응답 길이 상한(max\_tokens)은 앱이 고정한다.
- 게임별 분당 호출 제한(슬라이딩 윈도우)을 Rust에서 건다.

**결과**

- 토큰·비용 기준 예산은 아직 없다(후속). 임포트한 게임이 사용자 키로 호출을 늘리는 위험은 호출 제한으로만 막고 있다.

## ADR-004 게임 패키지(.tgame)와 import/export

**상태** Accepted

**결정**

- 포맷: zip — manifest.json, data/definition.json(PackageData: 각 테이블의 Input 타입 모음, camelCase), index.html, scripts/, assets/. 세이브·세션·일지·대화·API 키는 포함하지 않는다.
- import: 경로 탈출 방지, 크기 상한, 단일 트랜잭션. **모든 엔티티 id를 새 uuid로 재발급**하고 그 id를 가리키는 참조(문자열로 저장된 조건/효과 JSON, A>B 형태의 관계 키 포함)도 같이 치환한다. 월드인포 북은 새로 만들고 world\_info\_ids를 새 id로 다시 잇는다. 실패하면 만들다 만 폴더를 지운다.
- export: 폴더를 주지 않으면 다운로드 폴더에 저장하고, 같은 이름이 있으면 번호를 붙인다(실패 환경은 앱 데이터의 exports).

**이유** id가 테이블 전체에서 유일한 PK인데 game\_id만 새로 발급하면, 같은 패키지를 두 번 가져오거나 내가 내보낸 게임을 다시 가져올 때 기존 행을 덮어쓴다.

**결과**

- manifest.min\_engine\_version이 앱 버전보다 높거나 읽을 수 없으면 가져오지 않는다.
- 예전 snake\_case 최상위 키의 패키지는 가져올 수 없다. 가져온 게임의 월드인포 북은 사용자 북 목록에 추가되고, 게임 삭제 때 고아로 남는다(후속).

## ADR-005 정의/상태 분리와 로드 시 일괄 적재

**상태** Accepted

**결정**

- 플레이를 시작·이어갈 때 GameDefinition을 game\_id 기준 배치 조회로 **한 번에** 메모리에 올리고(futures::try\_join!), 이후 턴에는 DB를 읽지 않는다. 그래서 dispatch는 동기 함수다.
- 변하는 상태는 GameState(세션에 저장)이고, 순수 계산은 engine.rs(DB·async·Tauri를 모름, RNG 주입)에 둔다.
- 실행 중 런타임은 AppState.active\_runtime(tokio Mutex)에 하나만 둔다(락을 await 너머로 들고 가기 위해).

**결과**

- 정의를 편집해도 다음 로드부터 반영된다. 이어하기 때 repair\_state가 사라진 위치·씬·날씨·아이템·NPC를 안전한 값으로 되돌린다.
- 순수 함수 위주라 cargo test로 규칙을 검증하기 쉽다(시드 고정 RNG).

## ADR-006 이벤트 구동 시스템과 단일 writer

**상태** Accepted

**결정**

- 흐름: GameCommand → handle\_command(검증 + 씨앗 이벤트) → process\_events → 시스템들의 handle\_event. 새로 나온 이벤트는 다음 라운드의 입력이 되며 **최대 8라운드**까지만 전파한다(순환 방지).
- 시스템 순서: movement → time → map → trigger → npcs → events → relationship → dialogue → scene → inventory.
- **단일 writer**: 상태의 각 부분은 한 시스템만 쓴다(변수=trigger, 인벤토리=inventory, 관계=relationship, 씬=scene, NPC 위치=npcs, 위치=movement, 시간=time). 다른 곳은 **요청 이벤트**(VariableChanged, ItemChangeRequested, RelationChangeRequested, SceneRequested, EventRequested, TimeAdvanceRequested)만 내고, 쓰는 시스템이 규칙을 적용한 뒤 **결과 이벤트**(ItemAdded는 실제 수량 등)를 낸다.

**이유** 1단계 팬아웃은 변수 변경 → 트리거 → 효과 같은 연쇄를 못 받고, 무제한 재귀는 데이터 오류 하나로 앱이 멈춘다.

**결과** 효과가 상태에 반영되기까지 라운드가 늘어난다. 8라운드에서 끊기면 게임·세션 id, 남은 이벤트 수, 마지막 라운드에서 이벤트를 낸 시스템 이름을 로그로 남긴다.

## ADR-007 선언형 조건·효과, 못 읽으면 막는다

**상태** Accepted

**결정**

- 조건은 serde 태그 enum(Condition)으로 저장하고 DB에는 Condition 전체 JSON을 문자열로 둔다: flag\_gte, flag\_eq, min\_time, in\_location, in\_zone, at\_minute\_of\_day, has\_item, time\_period, weather, relation\_at\_least/at\_most, never.
- 효과 JSON 동사: set, add(정수는 정수로 유지), enter\_scene, fire\_event, give\_item, take\_item, relation. 새 조건·동사는 variant/verb 추가로 확장한다.
- **읽지 못한 조건은 Never(항상 거짓)** 로 대체한다. 빈 문자열만 조건 없음이다.

**이유** 조건 파싱 실패를 버리면 오염된 데이터가 오히려 이벤트·이동을 항상 통과시켰다.

**결과** validate가 효과·조건이 가리키는 씬·이벤트·아이템·캐릭터·축의 존재를 미리 검사한다.

## ADR-008 공간·시간 모델

**상태** Accepted

**결정**

- 계층: 맵 > 존(zone) > 위치(location). 이동은 location 단위(game\_location\_connections: 소요 초, 조건). MoveTo는 연결과 조건을 확인해 LocationChanged + TimeAdvanceRequested(소요 시간)를 낸다. ZoneChanged는 존이 바뀔 때만 추가로 나간다. 씬 진행 중에는 이동할 수 없다.
- game\_time은 **초 단위**로 통일한다. 시간대(time\_periods)는 start/end minute 범위로 판정하며 자정을 넘는 구간을 허용한다.
- 인카운터는 TimeAdvanced에 반응한다(이동이든 AdvanceTime이든 같은 경로).
- 맵의 danger\_level은 인카운터 확률 배율 1 + 레벨 x scale로 쓴다(settings\_json의 dangerEncounterScale, 기본 0.25). 시작 시각은 settings\_json의 startMinute.

## ADR-009 이벤트 종류·실행 순서

**상태** Accepted

**결정**

- game\_events.trigger\_type: manual / auto / interaction / location / scheduled / random. repeat\_policy: once / repeatable. event\_type은 제작자가 붙이는 열린 분류이며 엔진은 해석하지 않는다(프론트가 UI 선택에 쓴다). 발동 알림은 EventTriggered(event\_type, trigger\_type, npc\_id).
- 실행 순서 **interaction > location > scheduled > auto > random**. 라운드 구조(이동 직후 1라운드, TimeAdvanced가 나오는 2라운드)로 자연히 지켜진다. manual은 후보 풀에 없고 fire\_event 효과로만 발동한다.
- random은 같은 dispatch에서 다른 타입이 하나라도 발동했으면 굴리지 않는다. 같은 타입 안에서는 config\_json의 priority(내림차순) → id 순으로 정렬한다.
- once는 세션의 이벤트 로그 기준으로 지킨다. auto는 조건이 **거짓에서 참으로 바뀌는 순간에만** 발동하고(불러온 직후에는 참인 것을 발동 없이 표시), scheduled는 이번에 건너뛴 시간 구간에 그 시각이 포함되면 발동해 며칠을 건너뛰어도 한 번이다.

## ADR-010 세션·책갈피·일지·분기

**상태** Accepted

**결정**

- 세션(game\_player\_sessions)이 책 한 권이고, 책갈피(game\_saves)는 (session\_id, seq, slot\_number, 상태 스냅샷)이다. 세션 하나에 책갈피가 여러 개 붙는다. 같은 슬롯에 다시 저장하면 seq·위치·스냅샷이 모두 새 시점으로 바뀐다.
- 세션 안의 순서는 **하나의 seq**가 정한다. 일지(game\_journal), 메시지, 이벤트 로그가 같은 번호 공간을 쓰고, 런타임이 메모리에서 발급(next\_seq)하며 head\_seq를 세션에 저장한다(세션 하나는 런타임 하나가 쓰므로 안전하다).
- 한 턴의 기록(일지 + 이벤트 로그 + NPC/관계 변화량 + 세션 저장)은 journal::record가 **단일 트랜잭션**으로 쓴다. 새 게임의 첫 기록은 세션 행을 가장 먼저 쓴다(FK).
- 불러오기: 가장 최신 책갈피면 같은 세션을 이어가고, 오래된 책갈피면 **분기(fork)** 한다. 분기는 새 세션에 일지·이벤트 로그·관계/NPC/플랜 기록·대화(+참여자)·메시지를 seq까지 복사하고, 상태는 세션의 지금 값이 아니라 책갈피 스냅샷(engine)에서 복원한다. 원래 기록은 보존한다.
- 다시 보기는 읽기 전용이다(read\_journal, read\_conversations, read\_messages. AI 재호출 없음, 한 번에 1000개 상한). 변수 적용을 로드 때 재적용하지 않고, 적용 시점에 (키, 이전 값, 이후 값)을 메시지 meta에 남긴다.

**결과** 일지 성장 정책(요약·정리)은 미정이다. 분기 때 책갈피 이후에 만들어진 대화는 빈 대화로 따라온다.

## ADR-011 씬 모델

**상태** Accepted

**결정**

- 노드(game\_nodes)를 없애고 씬 + 선택지(+분기)로 단순화했다. 선택지의 next\_scene\_id가 비어 있으면 씬 종료다.
- 씬은 한 번에 하나만 열린다. 열려 있는 동안 요청된 씬은 대기열(pending\_scenes\_json)에서 기다리다 현재 씬이 끝나면 차례로 열린다. 대화 중에 씬이 열리면 먼저 DialogueEnded를 내고, 엔딩 씬이면 GameEnded를 낸다.
- 씬을 여닫는 건 scene 시스템뿐이다. 이벤트·선택지 효과의 enter\_scene은 SceneRequested만 낸다.
- Choose는 지금 열린 씬의 선택지이면서 표시 조건(condition)을 통과한 것만 받는다(fail-closed). RenderState.available\_choices로 프론트에 알린다. 시작 씬은 start\_scene\_key다.

## ADR-012 AI 대화 구조

**상태** Accepted

**결정**

- Talk/StopTalk로 대화창을 열고 닫는 북마크(active\_npc\_id, 세션에 저장하지 않음)만 엔진이 갖고, npc\_say는 열린 NPC와 일치할 때만 응답한다. 스폰 규칙이 있는 NPC는 같은 위치에 있어야 Talk할 수 있다.
- 대화 저장소는 게임 전용 테이블이다(game\_conversations: session\_id + branch\_key(=지금 씬) + participants\_key, 참여자 junction, game\_messages). 일반 채팅의 characters 테이블과 분리해서 분기 복사와 삭제 연쇄를 단순하게 했다. 같은 NPC라도 씬이 다르면 다른 대화다.
- npc\_say(npc.rs)가 ChatContext를 구성한다: 캐릭터, 페르소나, 월드인포(게임 + 캐릭터 + 페르소나), 최근 기록 20개, extra\_data(위치·시간·날씨·AI에게 보이는 변수·같은 자리의 관계). game\_chat은 raw 답변만 돌려주고 후처리와 저장은 npc.rs가 한다. 저장은 호출이 성공한 뒤이고 메시지 seq가 곧 일지 seq다.
- 페르소나는 새 게임을 시작할 때 세션에 고정한다(NULL이면 기본 페르소나로 복원).

**결과** NpcReply{text, events, state}로 응답해서 프론트가 엔진 커맨드와 같은 방식으로 이벤트를 처리한다.

## ADR-013 AI 출력 태그 프로토콜

**상태** Accepted

**결정**

- 태그: `<choice>`(선택지 버튼, meta.choices), `<updateVar>`(변수), `<updateRel>`(관계). 태그는 본문에서 벗겨 저장한다.
- 변수는 game\_state\_defs.config\_json의 {ai\_readable, ai\_writable, min, max, max\_step}를 따른다. 허용된 키만, 한 번에 max\_step까지, 응답당 최대 3태그/3변경이다. 관계는 NPC→플레이어(npc>player)만, 축 3개, ±5, 최대 3개다.
- 순서: 계획 세우기(상태 불변) → 메시지·일지 저장 → 적용. 적용 경로는 프론트가 부를 수 있는 GameCommand가 아니라 런타임 내부 메서드다. AI 출력은 신뢰하지 않는 입력으로 다룬다.

**결과** npc\_say·scene\_say는 응답을 스트리밍한다. 태그가 토큰 중간에서 잘려 오므로 game/stream\_filter.rs가 `<`부터 태그인지 판단될 때까지 붙잡고, 여는 태그 안의 내용은 닫는 태그까지 숨긴 글만 보낸다. 스트리밍 글은 기다리는 동안의 미리보기이고, 저장·연출은 완성된 응답을 파싱한 NpcReply를 쓴다.

## ADR-014 인벤토리·관계·NPC 위치

**상태** Accepted

**결정**

- 인벤토리: 정의 game\_items(max\_stack, is\_unique, is\_key, consumable, use\_effect, use\_condition, on\_acquire\_event\_id\[manual\]). 상태는 세션의 inventory\_json. 규칙은 inventory 시스템만 적용하고(스택 상한, 고유 규칙) ItemAdded는 실제 들어온 수량을 싣는다. DropItem은 핵심 아이템을 막는다. HasItem 조건으로 열쇠와 문을 만든다.
- 관계: 고정 축 affection/trust/tension, 범위 -100\~100. 키 `A>B`는 A가 B에게 느끼는 것이다. 초기값은 game\_relationship\_defs, 현재 값은 세션의 relationships\_json, 변화량만 game\_relationship\_snapshots에 쌓는다.
- NPC 위치: game\_character\_spawn\_locations(가중치 + 조건). 게임 시작과 시간대가 바뀔 때만 다시 굴린다(이동마다 굴리지 않는다). 결과는 NpcMoved로 알리고 game\_character\_state\_snapshots에 변화량만 쌓는다. 이벤트로 등장한 NPC는 플레이어 위치에 기록한다. 스폰 규칙이 없는 캐릭터는 위치를 추적하지 않고 항상 말을 걸 수 있다.

## ADR-015 정의 편집·무결성·화면용 데이터

**상태** Accepted

**결정**

- save\_package: **id 보존 upsert + 데이터에 없는 행만 삭제**, 한 트랜잭션. 통째로 지우고 다시 쓰지 않는다(세션·세이브·로그가 정의 행의 id를 FK로 참조한다). 이벤트 조건만 이벤트별로 교체한다.
- validate(순수 함수): Error가 있으면 start\_new\_game을 막고, Warning은 알려준다(발동하지 않는 이벤트, 조건 없이 매번 발동하는 이벤트, 참조 누락 등).
- 편집용 전체 데이터(get\_package\_data)와 플레이 화면용으로 추린 데이터(get\_game\_view)를 분리한다. 화면용에는 이벤트 조건·효과, NPC 풀, 인카운터 테이블, 이동 조건, 선택지 효과, 캐릭터 프롬프트 필드, AI 접근 설정을 넣지 않는다. 현재 값은 RenderState(flags, inventory, relationships, present\_npcs, available\_choices)로 준다.
- delete\_game은 DB를 먼저 지우고 실행 중인 게임은 막는다. 플레이 기록이 같이 지워지므로 UI에서 확인을 받아야 한다.

## ADR-016 템플릿과 템플릿 패키지

**상태** Accepted

**결정**

- **템플릿**은 엔진 규칙이다(id, name, apiVersion, templateVersion, capabilities{choices, freeText, inventory, map, aiNarration, location, time}). capabilities는 지금은 선언일 뿐 엔진이 강제하지 않고 get\_game\_view로 프론트에 알린다(지도·시간 끄기는 후속).
- **템플릿 패키지**는 시작 데이터(.tgame)다. 앱에 include\_bytes!로 포함하고 사용자 템플릿은 지원하지 않는다. create\_game(title, template\_id)은 번들 패키지를 import(id 재발급)하고 제목과 template\_id를 덮어쓴다.
- 첫 템플릿은 Dating Sim(기능 전체를 시험할 수 있는 샘플): 5개 시간대, 날씨 3종, 위치 6곳, NPC 3명, 이벤트 6종류, 씬·엔딩, 아이템·변수. 패키지의 index.html과 scripts는 SDK 확정 전 자리표시자이며 생성 스크립트로 다시 만든다.

## ADR-017 코드 구조와 관례

**상태** Accepted

**결정**

- 폴더: commands/(얇은 tauri 커맨드), game/(도메인: runtime, systems, manager, package, template, validate, session, journal, npc), db/, models/, service/(chat, ai). DB 함수는 get/get\_all/upsert/delete + 테이블명, 모델은 X/XRow/XInput. 정의 조회는 game\_id 기준 배치로 한다.
- 비동기: active\_runtime은 tokio Mutex, 나머지는 std Mutex. 병렬 조회는 futures::try\_join!(tokio::try\_join!은 모바일 빌드 제한 때문에 쓰지 않는다). reqwest와 tungstenite는 rustls. rand 0.10 API(random, random\_range, make\_rng).
- 검증: 순수 함수 단위 테스트, 번들 템플릿 패키지 역직렬화 테스트, 단계적 cargo check.

## ADR-018 에셋 서빙과 연출

**상태** Accepted — 아래 "ADR-018 갱신"이 이 항목을 확정·대체했다 (2026-11-18 감사에서 Proposed를 정정)

**결정**

- game\_assets 확장: key(게임 안에서 유일), category(background, cg, standing, map\_sprite, voice, bgm, sfx, font, other), game\_character\_id(standing/map\_sprite/voice의 주인), variant(포즈 이름), description(AI가 언제 쓰는지 판단하는 한 줄), ai\_usable, settings\_json(볼륨, 루프, 스탠딩 위치 등). 세션에 stage\_json(현재 배경·BGM·CG·서 있는 캐릭터와 포즈)을 둬서 이어하기와 분기에서 같은 화면을 복원한다.
- 서빙: 파일은 games/\<id>/assets/ 아래에 두고 DB의 file\_path는 그 안의 상대경로다. iframe은 상대경로를 그대로 쓴다. protocol.rs에 Range 요청(206)과 확장자 허용 목록(png, jpg, webp, gif, mp3, ogg, wav, m4a, opus, woff2), 파일별 크기 상한을 추가한다. SVG는 제외한다. file\_path는 절대경로와 `..`을 거절하고, 에셋 행을 패키지(PackageData)와 get\_game\_view에 포함한다.
- 연출은 하나의 이벤트(StageCue)로 통일하고 만드는 곳이 셋이다: 씬·위치의 settings\_json, 이벤트·선택지 효과의 stage 동사, AI 인라인 태그.
- AI 태그는 `<pose>`(지금 말하는 캐릭터의 스탠딩만), `<sfx>`, `<bgm>`, `<bg>`, `<cg>`다. 에셋 **키**로 지시하고 ai\_usable인 것만 허용하며 모르는 키는 버린다. 응답당 상한을 두고(bgm과 bg는 한 번), 태그를 **본문 위치를 보존한 타임라인(segments)** 으로 파싱해 메시지 meta에 저장한다(다시 볼 때 같은 연출이 재생된다).
- **음성(voice)은 AI 태그에 포함하지 않는다.** 나중에 음성 생성 API를 연결할 수 있도록 남겨두고, 지금은 사용자가 추가한 음성 파일만 관리하며 엔진·이벤트가 재생한다.
- 맵 스프라이트는 AI와 무관하게 get\_game\_view의 에셋 목록과 present\_npcs로 프론트가 그린다. 스탠딩이 여러 명이면 위치(left/center/right)를 에셋 설정으로 정한다.

## ADR-019 StoryPlan(각본가)

**상태** Deferred — dev에는 없고 구현이 브랜치 `claude/game-assets-storyplan`에 있다(병합 결정은 `ADR 미결 정리.md`, roadmap 4-D)

**결정**

- AI가 각본가, 엔진이 무대다. 각본가는 NPC 대화와 **별개의 호출**이고 체크포인트(세션 시작, 씬 종료, 하루 경계, 이벤트 발동 후, N턴마다)에서만 부른다. 플랜은 구조화 데이터(전제, 단계와 상태, 열린 복선, 확정된 사실, 요약)이고 NPC 대화에는 활성 단계와 지켜야 할 사실만 잘라 넣는다.
- 플랜은 조언일 뿐 엔진 상태를 직접 바꾸지 않는다. 제작자가 표시한 manual 이벤트와 씬을 「무대에 올리라」고 제안하는 것만 가능하고, fire\_event와 같은 검증을 통과해야 한다. 출력은 신뢰하지 않는 입력으로 다루고(스키마, 크기 상한, 존재하는 id만) 프롬프트에서는 앱 고정 규칙보다 낮은 우선순위로 둔다.
- 저장은 추가 전용 game\_story\_plans(session\_id, seq)라서 분기 때 seq까지 복사된다. 기본은 꺼짐이고 호출 상한은 npc\_say와 같은 제한 안에 둔다. 인벤토리와 관계 이후에 구현한다.

## 플레이 화면의 입력 흐름

**상태** Accepted (2026-11-16)

- 씬이 열려 있으면(RenderState.currentSceneId) 대사창 입력과 AI가 제안한 선택지(`<choice>`)는 씬 입력(sceneSay)으로 간다. 씬 동안은 1:1 대화(talk)가 막히므로 오른쪽 패널의 "말 걸기"도 꺼진다.
- 씬 밖에서 NPC와 대화 중이면(activeNpcId) 같은 입력이 npcSay로 가고, 대화 동안 씬 선택지는 숨긴다.
- RenderState.sceneChat이 거짓이면(엔딩 씬, AI 이어가기가 꺼진 씬) 씬 입력창과 "장면 끝내기"를 보여주지 않는다. "장면 끝내기"는 endScene 커맨드다.
- 씬 선택지를 고르면 대사창이든 오른쪽 패널이든 같은 dispatch(choose)를 거친다. 응답의 ChoiceMade.continuesScene이 참이면 선택지 문구로 sceneSay를 이어 불러, 선택지 결과 대사 뒤에 AI 서술을 붙인다.

## zone 날씨·턴 시점·지난 대사

**상태** Accepted (2026-11-16)

- **날씨는 zone 단위**다. 세션의 zone\_weather\_json(마이그레이션 130)에 zone별 {weather, day, period}를 기억하고, 같은 zone·같은 날·같은 시간대 안에서는 시간이 흘러도(같은 zone 안에서 이동해도) 날씨를 다시 굴리지 않는다. 시간대나 날이 바뀌었거나 그 시점에 처음 들어간 zone이면 그 zone의 마지막 날씨(없으면 지금 날씨)에서 맵의 전이 확률로 굴린다. ADR-008의 "TimeAdvanced마다 굴림"을 대체한다.
- **턴 시점**: 턴이 끝날 때마다 일지에 상태 스냅샷(kind=state)을 남긴다(journal::record, NPC 대화 턴은 journal::checkpoint). 이 seq가 분기할 수 있는 시점이고, fork\_at(session, seq)은 책갈피 분기와 같은 복사 규칙으로 새 세션을 만든다. list\_turn\_points는 시점마다 그 턴의 마지막 대사(없으면 행동)를 요약으로 준다. 이 기능 전에 진행한 턴은 스냅샷이 없어서 목록에 없다.
- **지난 대사**는 오른쪽 패널이 아니라 무대 위 창(L)이다. read\_current\_transcript가 범위를 정한다: 씬이 열려 있으면 그 씬이 마지막으로 열린 뒤의 대본과 그 씬의 대화, 씬 밖 NPC 대화 중이면 그 대화를 시작한(dialogueStarted) 뒤의 메시지, 둘 다 아니면 최근 기록.
- 플레이 화면: 연출만 있는 줄(배경 전환·캐릭터 내리기)은 앞뒤 대사 줄에 합쳐 "…" 줄로 보이지 않게 한다. 얻은 아이템은 대사창에 "~을 얻었다"로 이어서 보인다. 할 일이 없으면 대사창에 이동·대화·살펴보기 목록을 바로 보여준다. 효과음·음성 연출은 그 줄이 처음 보일 때 한 번 재생한다(소리 끄기 버튼이 함께 적용).

## 게임 오류 코드

**상태** Accepted (2026-11-16)

게임 커맨드의 거절은 코드로 구분한다(`src-tauri/src/error/game_error.rs`). 프론트는 `src/features/game/errors.ts`로 코드별 알림을 고른다.

| 코드 | 언제 | details |
| --- | --- | --- |
| GAME\_NOT\_OPENED | open\_game을 부르지 않음 | |
| GAME\_NO\_SESSION | 진행 중인 세션(런타임)이 없음 | |
| GAME\_SESSION\_CHANGED | AI를 기다리는 동안 세션이 바뀜 (응답은 버림) | |
| GAME\_DATA\_INVALID | 게임 데이터 문제로 시작 불가 | diagnostics: 진단 오류 목록 |
| GAME\_ACTION\_BLOCKED | 규칙상 지금 할 수 없는 행동 | reason |
| GAME\_INVALID\_STATE | 지금 상태와 맞지 않는 요청 (지난 플레이의 메시지 고치기, 열린 게임 지우기 등) | |

reason(BlockReason): conditionNotMet, sceneActive, notPresent, noRoute, noActiveDialogue, noOpenScene, endingScene, choiceNotInScene, choiceUnavailable, sceneChatDisabled, itemNotOwned, itemNotUsable, keyItem.

없는 id(캐릭터·선택지·아이템 등)나 잘못된 값(음수 시간)은 규칙이 아니라 입력 문제라 INVALID\_INPUT으로 둔다. 화면은 GAME\_ACTION\_BLOCKED를 실패가 아닌 경고로, 세션이 닫힌 경우는 게임 화면으로 돌아가는 버튼과 함께 알린다.

## 미결·후속 작업

> 2026-11-18 코드와 대조해 정리했다(`ADR 미결 정리.md`). 번호는 `plans/roadmap.md`.

- **iframe SDK·호스트 postMessage 브릿지**, 호스트 전용 커맨드와 게임이 부를 수 있는 커맨드의 capabilities 분리 — 보류, 6-C.
- **에셋**: 서빙·정의·검증·view·StageCue·태그 파서·프롬프트는 구현됨. 남음: 에셋 추가/삭제 커맨드(브랜치 `claude/game-assets-storyplan`에 있음)와 프론트 재생기 — 4-A.
- **capabilities 강제**(지도·시간 끄기): 브랜치에 구현이 있음, 병합 시 해결 — 4-B-1. chat 전용 템플릿 4-C-1, 사용자 템플릿은 요청이 있을 때(6-C).
- 호출 예산: 구현됨(게임 대사도 `AiService`를 거쳐 ADR-007 한도가 적용됨). **일지 성장 정책** 남음 — 4-A.
- **AI의 아이템 지급**(결정 필요: 지급 조건)·**게임 삭제 시 월드인포 북 정리**(확인 필요) 남음 — 4-A. NPC끼리의 관계 변화(시뮬레이션)·관계 tags 변경 효과는 보류(요청이 있을 때, 호출 비용 때문에 ADR이 먼저).
- 이전 형식(snake\_case) 패키지 호환은 폐기 — 필요가 생기면 format\_version으로 구분한다.

## ADR-018 갱신 — 에셋·연출 결정 확정

**상태** Accepted — 서빙, 데이터, 파서, 무대 상태는 구현됨. 프론트 재생기와 에셋 추가/삭제 커맨드는 후속. (위 ADR-018의 Proposed 내용을 이 항목이 대체한다.)

**스키마**

- game\_assets(key, category, kind, file\_path, settings\_json) + 종류별 하위 테이블. 하위 테이블의 PK는 assets\_id다.
  - audio\_assets: tag(효과음은 소리 그룹 이름, BGM은 분위기 id), game\_character\_id(음성의 화자), transcript
  - character\_assets: game\_character\_id, usage(standing/map), emotion(스탠딩은 필수)
  - cg\_assets: description(AI에게 보낼 설명), ai\_usable, 생성 메타(prompt/seed 등)
  - background\_assets: location/time/weather(NULL이면 그 조건은 전체)
- 하위 테이블 FK는 SET NULL이고, 에셋 정리는 코드(save\_package)가 한다. FK 때문에 지워진 location의 배경이 「전체 기본 배경」으로 바뀌는 걸 막기 위해, 지워지는 location, 시간대, 날씨, 캐릭터에 묶인 에셋은 에셋째로 먼저 지우고 파일도 같이 지운다.
- 세션에 stage\_json, 씬에 script 컬럼을 둔다.

**선택 규칙**

- 같은 조건의 중복 에셋은 **랜덤**으로 고른다(배경, 스탠딩, 효과음, BGM 모두). 뽑힌 결과는 기록에 박혀서 다시 볼 때 같은 파일이 나온다.
- 배경은 위치 > 시간대 > 날씨 순으로 일치하는 조건이 많은 후보가 우선이고 최하위는 전체 기본이다. 지금 보이는 배경이 새 조건의 최적 후보 안에 있으면 바꾸지 않는다.

**문법** (제작자 대본과 AI 응답이 같은 문법을 쓰고, 허용 범위만 다르다)

- 한 줄이 한 노드다. `이름:` 접두가 캐릭터 이름이면 대사, 아니면 나레이션이다. 태그는 줄 어디에 있든 그 줄을 보여주기 직전에 실행되고, 태그만 있는 줄은 다음 줄에 합친다. 프론트는 긴 줄을 글자 수로 나눠 출력한다.
- 태그: `<bgm>분위기</bgm>`, `<sfx>소리 이름</sfx>`, `<cg>키</cg>`/`<cg>off</cg>`, `<voice>키</voice>`(제작자 전용), `<effects>이름[:값]</effects>`, `<emotion::캐릭터>감정</emotion>`, `<motion::캐릭터>동작</motion>`. 배경은 태그가 아니라 위치, 시간대, 날씨로 엔진이 자동 선택한다.
- 기본 제공 목록(엔진 상수): BGM 분위기 6개(default, peaceful, upbeat, romance, tense, battle), 화면 효과 10개(shake, flash, blackout, fade\_in, fireworks, petals, snow, rain, fog, clear), 모션 8개(jump, bounce, shake, nod, sway, shrink, slide\_in, slide\_out).
- 풀 수 없는 키, 허용되지 않은 태그, 상한을 넘는 태그는 조용히 버린다.

**AI 연출 규칙**

- AI는 파일이 아니라 이름만 본다. 목록은 같은 위치의 인물(최대 4명, 설명 200자), 인물별 표정(스탠딩 감정명), 효과음 그룹, BGM 분위기, ai\_usable인 CG(키+설명), 화면 효과, 모션이다.
- 화자와 연출 대상은 **대화 상대 + 같은 위치의 NPC**만 가능하다. 플레이어 대사와 이 자리에 없는 인물의 대사는 버린다. 응답당 연출 6개, BGM 1개, CG 1개가 상한이다. 음성(voice)은 AI 태그에 포함하지 않는다(나중에 음성 생성 API를 연결할 가능성을 위해 남겨둔다).
- 저장되는 content는 태그를 벗기고 ` 이름:  ` 접두를 되살린 평문이고, 줄 목록(연출 포함)은 메시지 meta.lines에 남는다. `이름:` 접두가 하나도 없는 응답은 전부 대화 상대의 대사로 취급한다.

**무대 상태와 이벤트**

- stage\_json: {background, bgm:{mood, asset}, cg, standings:{캐릭터id:{emotion, asset}}}. stage 시스템이 유일한 writer이고, 위치가 바뀌면 CG와 스탠딩을 내리고 씬이 끝나면 CG를 내린다(탐험 중에 CG가 남지 않게).
- 이벤트: SceneScript(씬이 열릴 때 대본의 줄 목록), StageLines(연출만 있는 줄: 배경 전환, 효과 stage). 이벤트 효과 동사 `stage`는 대본 문법으로 바뀌어 같은 파서와 검증을 탄다. NpcReply.lines가 AI 응답의 줄 목록이다.

**서빙**

- 파일은 games/\<id>/assets/ 아래, DB의 file\_path는 상대경로다. protocol.rs는 Range(206, 한 번에 최대 4MB)와 확장자 허용 목록(SVG 제외), 종류별 크기 상한을 둔다. file\_path는 절대경로와 `..`을 거절한다. validate는 키 문자, 카테고리와 종류 일치, 고아 에셋, 대본의 쓸 수 없는 태그, 파일 존재를 검사한다.

**후속**

- 프론트 재생기(타자기, 연출 실행, 효과/모션 구현), 에셋 추가/삭제 커맨드(파일 복사와 행 생성), RenderState의 stage를 에셋 참조로 풀어서 내려주기, 스트리밍 시 태그 버퍼링.
- 외부 이미지 생성 API로 CG 만들기(Deferred), 음성 생성 API 연결(Deferred).

## ADR-016 갱신 — capabilities 강제 (지도·시간 끄기)

**상태** Accepted

- 실제로 쓰는 기능 = 템플릿 capabilities에서 게임 설정 `settings_json.capabilities`가 **끈 것**을 뺀 값(게임 데이터가 템플릿이 끈 기능을 켤 수는 없다). `GameDefinition::capabilities()` 하나를 커맨드·화면용 데이터·검증·AI 프롬프트가 같이 쓴다.
- map 또는 location이 꺼지면 MoveTo가, location이 꺼지면 Interact가, time이 꺼지면 AdvanceTime이 `GAME_ACTION_BLOCKED`(details.reason = `capabilityDisabled`)로 거절된다. time이 꺼진 게임은 이동해도 시간이 흐르지 않고 시간 시스템도 요청을 무시한다.
- 꺼진 기능의 값은 get_game_view(maps/zones/connections, locations/interactables, timePeriods/weatherTypes는 빈 배열)와 RenderState(mapId/zoneId/locationId, gameTime/timePeriodId/weatherId는 null, availableMoves는 빈 배열), NPC/씬 프롬프트에서 빠진다. view에는 실제 값 `capabilities`가 추가된다.
- validate: 발동할 수 없는 이벤트(time 꺼짐 + scheduled/random, 이동 꺼짐 + location, location 꺼짐 + interaction)는 Error, 시작 값에 고정되는 시간/위치 조건과 만날 수 없는 NPC는 Warning.

## ADR-018 갱신 — 에셋 추가/삭제 커맨드

**상태** Accepted

- add_game_asset: 파일 형식은 내용(infer)으로 판정하고 저장 확장자도 판정 결과로 정한다. 서빙 허용 목록·크기 상한(protocol)과 같은 기준이고, mime_guess로 한 번 더 확인한다. 저장 경로는 `assets/<category>/<uuid>.<ext>`, 폴더가 게임 폴더 밖이면 ARCHIVE_UNSAFE_PATH. 복사 후 다시 판정하고, DB(트랜잭션)가 실패하면 복사한 파일을 지운다.
- remove_game_asset: 씬 대본·이벤트/선택지/아이템 효과의 stage(cg/voice 키, 그룹의 마지막 sfx/bgm, 캐릭터+감정의 마지막 스탠딩), 표지/아바타/아이콘 경로가 쓰는 에셋은 거절(details.reason = `assetInUse`, references). force면 지우고 진단을 돌려준다. DB 커밋 후 파일을 지운다.

## ADR-019 갱신 — StoryPlan 구현

**상태** Accepted

- 켜기: `settings_json.storyPlan = {enabled, everyNTurns(기본 10)}`. 제작자 표시: 이벤트 `config_json.planUsable`(manual만), 씬 `settings_json.planUsable`.
- 체크포인트(세션 시작/이어하기, 씬 종료, 하루 경계, random 이외 이벤트 발동, N턴)는 런타임이 RenderState.storyPlanDue로 표시만 하고, refresh_story_plan이 AiService.generate_once로 부른다(사용자 프리셋은 쓰지 않음, 앱 고정 규칙이 맨 앞). AI 대기 중에는 런타임 락을 잡지 않는다.
- 출력은 크기 상한·스키마·길이/개수 상한·`<` `>` 제거·planUsable id만 남긴 뒤 game_story_plans(session_id, seq)에 저장한다. 제안을 올리는 stage_story_suggestion은 fire_event/enter_scene과 같은 검증을 다시 하고, 같은 플랜의 같은 제안은 한 번만 올린다.
- NPC/씬 대화에는 활성 단계와 확정된 사실만 캐릭터 설정 끝에 붙인다. 호출 상한은 npc_say의 게임별 분당 제한을 같이 쓴다.
- 한 번의 짧은 호출이라 TaskManager 작업으로 만들지 않았다(취소/진행 표시가 필요해지면 GameDialogue Executor로 옮긴다).
