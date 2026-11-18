# 기술 명세 (Tech Spec)

> 지금 코드가 어떻게 생겼는지를 한 곳에서 보는 문서다. **결정과 그 이유**는 ADR(`아키텍처 결정 기록 (ADR) — 프로젝트 전반.md`, 게임은 `… 게임 모드.md`)에, **작업 규칙**은 `.claude/CLAUDE.md`에, **계획·일정**은 `plans/`에 있다. 여기서는 번호(ADR-0xx)로 가리키기만 한다.

## 1. 개요

- AI 채팅 앱 (캐릭터와 대화하는 채팅, 메신저 같은 폰 모드, 이벤트가 있는 게임 모드). Windows(데스크톱)와 Android.
- 스택: **Tauri 2**(Rust 백엔드·네이티브 창) · **Svelte 5**(runes) · 순수 CSS(디자인 토큰, `src/shared/ui/tokens.css`) · **Pixi.js**(위젯 마스코트) · **SQLite + sqlx**.
- 버전은 `package.json` 한 곳(`pnpm version:set`이 Cargo·tauri 설정에 퍼뜨림, ADR-010). 변경 내역은 `CHANGELOG.md`.

## 2. 구조

```
Svelte (src/)  ──  IPC (Tauri 커맨드·이벤트·Channel, specta로 만든 src/ipc/bindings.ts)  ──  Rust (src-tauri/src/)
  features/ 화면별 폴더                                                                      commands/  얇은 IPC 진입점
  shared/   공용 UI·store·IPC 도우미                                                         service/   로직 (chat, ai, memory, summary, community, data, drive …)
  app/      셸·탐색·패널                                                                      db/        get / get_all / upsert / delete + 테이블명
  ipc/      bindings.ts, dev-mock/ (브라우저 미리보기용 가짜 응답)                            adapter/   공급자별 HTTP 형식
                                                                                           task/      TaskManager (모든 유료 호출의 작업화)
                                                                                           game/      게임 엔진
```

원칙
1. 외부 API 호출은 **Rust에서만**. API 키·토큰은 OS keyring(`service/secret_store.rs`)에만 있고 설정·DB·백업·동기화 묶음·로그에 넣지 않는다(ADR-005, 게임 ADR-003).
2. Svelte는 화면, Pixi.js는 캔버스 그림. 데이터 저장소와 화면 상태를 한 store에 섞지 않는다.
3. 새 기능은 코어 플러그인(기본 꺼짐). 꺼진 기능은 타이머·구독·데이터 읽기를 시작하지 않는다(Lazy Initialization).
4. 요금이 드는 호출은 반드시 **TaskManager 작업**(기능 id·`run_by`·이름)으로, 자동 실행은 별도 스위치가 켜졌을 때만, 실패해도 자동 재시도하지 않는다(ADR-006).

## 3. 데이터

| 위치 | 내용 | 백업 | 동기화 |
| --- | --- | --- | --- |
| `app.db` | 캐릭터·페르소나·로어북·대화·메시지·프리셋·정규식·게임·테마·작업·사용량 … (마이그레이션 `src-tauri/src/db/migrations/`, 기존 파일은 고치지 않음) | O | 대부분 항목별 병합 |
| `memory.db` | 메모리 검색의 벡터·chunk 표 (파생 데이터, ADR-015) | X | X |
| `settings.json` | 앱·테마·기능·플러그인 설정 (칸마다 같이 씀/기기 전용, `settings_scope.rs`) | O | 같이 쓰는 칸만 |
| 파일 폴더 | `characters/ games/ generated/ backgrounds/ themes/ fonts/ plugins/ trash/` | 대부분 O | 사용자 데이터 파일은 해시로 |
| OS keyring | API 키·드라이브 토큰·플러그인 비밀 | X | 절대 X |

- 모든 쓰기는 `updated_at`을 갱신하고(트리거), 지운 것은 `sync_tombstones`에 남는다. 큰 항목 7종은 지울 때 휴지통으로(ADR-011).
- 새 테이블은 `db/change_tracking.rs`에서 동기화 여부를 분류해야 하고(테스트가 강제), 새 설정 칸은 `settings_scope.rs`에 적어야 한다.
- 시작할 때 적용할 마이그레이션이 있으면 `migration-backups/`에 사본을 먼저 만든다(ADR-008).

## 4. 대화 한 번의 흐름 (`service/chat/chat_manager.rs`의 `turn`)

1. 대화·캐릭터·페르소나·프리셋·API 결정(대화 → 캐릭터 기본 → 앱 설정) → 컨텍스트(기록·월드인포) 구성
2. 작가 노트(메모는 AI에 보내지 않음, ADR-016) → 대화 요약 블록 → 커뮤니티 프롬프트 훅 → 자동 회상 블록(켜져 있을 때, ADR-015) → "지시와 함께 다시 생성"의 지시(ADR-017)
3. 프롬프트 조립(`service/prompt/`, 프리셋 슬롯·토큰 예산) → `AiService`(`service/ai/`)가 공급자 어댑터(`adapter/`: openai·claude·gemini·vertexai·nai)를 불러 스트리밍
4. 정규식(AiOutput) → `<image>`·`<cg>` 정리 → **linter**(ADR-014) → 저장(스와이프·meta) → 화면에서 번역·읽기·그림·커뮤니티 후처리
5. 저장 뒤 백그라운드: 요약(쌓였을 때만, ADR-016), 자동 재생성 예약(linter가 걸었을 때, ADR-017)

`AiService`는 도구 호출·구조화 출력·임베딩도 제공하고 연결별 지원 표(`adapter/capabilities.rs`)로 안 되는 것은 호출 전에 거절한다(ADR-013). 사용량·예산 한도(ADR-007)는 공급자를 부르기 직전에 확인한다.

## 5. 코어 플러그인

`src-tauri/src/sys/plugins.rs`의 목록이 원본이고 설정 > 코어 플러그인에서 켠다. 기본 켜짐은 폰·게임 모드·캐릭터 이미지·정규식뿐이다.

| id | 기능 | 기본 | ADR |
| --- | --- | --- | --- |
| `phone` `game` | 폰 모드, 게임 모드 | 켜짐 | 001·게임 |
| `chat.images` `regex` | 캐릭터 이미지(`<cg>`), 정규식 | 켜짐 | 005 |
| `html` | AI 답변의 HTML 블록 렌더링(격리 iframe) | 꺼짐 | 004 |
| `widget` | 바탕화면 위젯(데스크톱 전용) | 꺼짐 | 005 |
| `translate` `tts` `imagegen` `imageauto` | 번역, 음성 읽기, 이미지 생성, 자동 그림 | 꺼짐 | 005·006 |
| `sync` | 기기 사이 동기화 | 꺼짐 | 011 |
| `community` | 사용자가 설치한 플러그인 | 꺼짐 | 012 |
| `lint` | 메시지 linter | 꺼짐 | 014 |
| `memory` | 메모리 임베딩 검색·자동 회상 | 꺼짐 | 015 |
| `summary` | 메모리 확장: 대화 요약 | 꺼짐 | 016 |
| `rewrite` | 자동 재생성 | 꺼짐 | 017 |
| `ghostwrite` | 대필 | 꺼짐 | 018 |

## 6. 커뮤니티 플러그인

- 폴더 하나 = 플러그인 하나(`manifest.json` + 코드). 격리된 sandbox iframe에서 돌고 `window.sandcastle` RPC로 **허락한 권한**의 API만 쓴다(ADR-012). SDK 판은 manifest의 `apiVersion`, 앱이 지원하는 범위는 `service/community/manifest.rs`.
- 확장 지점: 메시지 후처리, 프롬프트 블록 추가, 화면(패널·명령), 데이터 읽기·쓰기 제안(승인 필요), AI 호출(작업·한도 안에서).
- 만드는 법은 `플러그인 만들기.md`, 예제는 `examples/plugins/`.

## 7. 동기화

기기마다 자기 묶음(`.scsync`)을 만들고 다른 기기 것을 항목 단위로 병합한다. 양쪽을 고치면 둘 다 남기고 사용자가 고른다. 저장소는 사용자가 고른 폴더, 구글 드라이브 앱 전용 폴더(Android 포함). API 키는 어떤 경우에도 묶음에 들어가지 않는다(키 누출 테스트가 묶음·합친 기기의 폴더 전체를 검사). 자세한 것은 ADR-011.

## 8. 게임 엔진

정의/상태 분리, 이벤트 구동·단일 writer, 선언형 조건·효과, AI 출력 태그 프로토콜, 템플릿. 플레이 세션 흐름은 `open_game` → `start_new_game`/`continue_game` → 사용자의 행동마다 `dispatch_command`(GameResponse 반환). 결정은 게임 ADR(001~019)에 있다.

## 9. 화면

- 공용 컴포넌트 `src/shared/ui/`, 색·폭·겹침 순서는 `tokens.css` 변수. 새 화면의 글은 `src/locales/`의 키(전면 다국어는 5-C 예정).
- 좁은 화면(`narrow`)과 Android: 안전 영역·키보드·누르는 대상 44px·꾹 눌러 메뉴, 데스크톱 전용 기능(위젯 등)은 Android에서 숨김(ADR-009).
- 브라우저 미리보기(`pnpm dev`)는 `src/ipc/dev-mock/`의 가짜 응답을 쓴다 — IPC를 바꾸면 거기에도 응답을 추가한다.

## 10. 빌드·배포

- CI: 프런트(check·test), Rust(fmt·clippy·test), Android(aarch64 clippy), E2E 스모크. 태그를 달면 릴리스 워크플로가 Windows 설치 파일과 APK로 초안 릴리스를 만든다(`배포.md`).
- 새 버전 확인은 사용자가 누를 때만(또는 켜 둔 경우 시작 때), 내려받기는 늘 사용자가 누른다.

## 11. 테스트와 품질

| 종류 | 방식 | 확인하는 것 |
| --- | --- | --- |
| UI | Vitest (jsdom) | store·도우미·컴포넌트 로직 |
| Rust 단위·DB | `cargo test` (임시 SQLite) | 로직과 저장·조회 |
| 공급자 | 가짜 공급자 서버(`adapter/testutil.rs`) | 요청 모양·응답 해석·예산·취소 — **테스트에서 실제 유료 API를 부르지 않는다** |
| E2E | WebdriverIO + msedgedriver(앱의 디버깅 포트에 붙음) | 실제 앱 시작 → 가져오기 → 메시지 → 백업 |

```
pnpm check               # 타입 검사
pnpm test:run            # 프런트 테스트
cd src-tauri
cargo fmt --check
cargo test
cargo clippy --all-targets
cargo run --bin export-bindings   # IPC를 바꿨을 때 src/ipc/bindings.ts 다시 만들기
pnpm e2e:build ; pnpm e2e         # E2E 스모크 (Windows, MSEDGEDRIVER 필요)
```

## 12. 알려진 위험과 제한

- 배포용 서명: Windows 코드 서명은 없음(SmartScreen 경고), Android 서명 키는 GitHub Secrets(ADR-010).
- 실제 공급자·실기기 확인이 부족한 영역(각 마일스톤 기록의 "하지 못한 것")은 `plans/current-milestone.md`에 모아 둔다.
- 커뮤니티 플러그인과 AI가 쓴 HTML은 격리 iframe에서 돌지만 외부 리소스 요청으로 신호를 보낼 수 있다(ADR-004·012의 한계).
