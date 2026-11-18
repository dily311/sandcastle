# 샌드캐슬 — 작업 안내

- 목표: ai 채팅앱 구축
- 사용스택: svelte, tauri, pure css, pixi.js
- 플랫폼: 윈도우와 안드로이드
- 주요기능: ai 채팅 플랫폼
- 부가기능
    - 위젯 (데스크톱 전용)
    - phone/game 모드

## 문서 지도

이 파일에는 **변하지 않는 규칙**만 둔다. 계획·기록은 아래로 나눴다.

| 문서 | 내용 |
| --- | --- |
| `plans/roadmap.md` | 중장기 계획(1~6단계), 평가, 단계별 세부 계획(번호 = 커밋 단위, 예: `3-B-2`) |
| `plans/current-milestone.md` | 지금 하고 있거나 곧 할 일, 하다 만 것, 확인하지 못한 것 |
| `plans/idea.md` | 계획 단계가 아닌 아이디어 스케치 (구현 전에 roadmap에 번호를 받아야 함) |
| `docs/마일스톤 기록.md` | 끝난 마일스톤(M1~)의 결정·체크·계획과 다르게 한 것·하지 못한 것 |
| `docs/아키텍처 결정 기록 (ADR) — 프로젝트 전반.md` | 앱 전반의 결정과 그 근거 (ADR-001~) |
| `docs/아키텍처 결정 기록 (ADR) — AI 스토리 엔진 게임 모드.md` | 게임 모드의 결정과 근거 |
| `docs/` 그 밖 | Tech Spec, Android 점검표, 배포, 성능 측정, 플러그인 만들기 |

> 코드 주석·문서의 "CLAUDE.md 3-B" 같은 표기는 계획이 이 파일에 있던 때의 것이다. 번호(3-B 등)는 `plans/roadmap.md`의 번호와 같다.

## 중요하게 생각해야 하는 것

| 원칙 | 설명 |
| --- | --- |
| Lazy Initialization | 기능을 사용하기 전에는 초기화하지 않는다. |
| Explicit Execution | API 비용이나 외부 작업이 발생하면 실행 주체가 명확해야 한다. |
| Least Privilege | 기능은 필요한 데이터와 권한만 접근한다. |
| Reversibility | 가능하면 실행 취소, 복구, 원본 보존을 지원한다. |
| Observability | 사용자는 기능의 상태와 실행 결과를 확인할 수 있어야 한다. |

- 기능의 존재와 실행 단계를 반드시 분리
    - 기능이 설치되어 있어도 꺼져 있는 경우 리소스를 소비되게 하면 안됨
    - 새로 기능을 추가하는 경우에는 기본값은 반드시 off
- 데이터와 UI 상태 분리
- 사용자 동의 없는 리소스 소비 금지
    - API 등 지출이 필요한 단계는 사용자가 인식할 수 있어야 함. 예를 들어 채팅기능의 send 버튼처럼 사용자가 실행한 동작이 있어야 함. 선톡 기능의 경우 사용자가 옵션을 키지 않으면 실행되지 않도록 할 것
- 사용자 경험

## 작업 방식 (계획·기록)

- **번호 하나가 커밋 하나 크기**다. 작업은 `plans/roadmap.md`의 번호(예: `3-B-2`)로 지시하고 받는다. 번호를 나누거나 합쳐야 하면 roadmap을 먼저 고친다.
- **결정이 필요한 항목은 합의 전 구현 금지**. roadmap에 "결정 필요"라고 적힌 항목은 추천안이 적혀 있어도 사용자와 합의하기 전에 구현하지 않는다. 합의한 내용은 ADR에 적는다.
- **구현 위치와 완료 기준을 명시**한다. 모든 항목은 `번호 이름 — 무엇을. 어디: 파일. 완료: 기준` 형태로 적고, 완료 기준을 만족하는 테스트(또는 확인 방법)가 있어야 체크한다.
- **계획과 다르게 구현한 부분을 기록**한다. 이유와 함께 마일스톤 기록의 "계획과 다르게 한 것"에 적는다.
- **하지 못한 작업도 명시**한다. 못 한 것, 확인하지 못한 것(실제 공급자·실기기·실제 앱 등)을 "하지 못한 것"에 적고, 하지 않은 것을 한 것처럼 보고하지 않는다. 후속은 `plans/current-milestone.md`에 남긴다.
- **ADR을 통해 결정의 근거를 보존**한다. 설계·정책 결정(실행 자리, 기본값, 한계, 선택하지 않은 안)은 ADR에 번호를 붙여 적고(`ADR-0xx`), 나중에 바뀌면 지우지 않고 갱신·"추가"로 덧붙인다.
- **마무리 순서**: 구현 → 검증 → ADR(결정이 있었다면) → `plans/roadmap.md` 체크 → 끝난 마일스톤은 `docs/마일스톤 기록.md`에 기록하고 `plans/current-milestone.md`에서 정리 → 커밋.
- 설계·보안이 어려운 결정은 Opus 하위 에이전트에게 자문을 받고, 무엇을 받아들이고 무엇을 바꿨는지 ADR에 적는다.

## 브랜치·커밋

- **작업은 `dev`에서만** 한다. 기능 브랜치(`feature/*`, `fix/*`)는 `dev`에 합친다.
- **`main`에는 직접 커밋·push하지 않는다.** `main`은 보호되어 있어 PR 없이는 push할 수 없다. `dev` → `main`은 사용자가 요청할 때만 PR로 올린다.
- `git push --force`, 훅 건너뛰기(`--no-verify`)는 하지 않는다. 이미 push한 커밋을 고쳐 쓰지 않는다(새 커밋으로).
- 커밋 메시지는 한국어로 번호와 무엇을 했는지·확인한 것을 쓰고, 끝에 `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` 줄을 붙인다. PR 본문 끝에는 `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- 커밋·push·PR은 사용자가 요청했을 때만 한다. 비밀(키·토큰·`google-oauth.env` 등 git에 올리지 않기로 한 파일)은 절대 커밋하지 않는다.
- 합치기·지우기처럼 되돌리기 어려운 일은 대상을 확인한 뒤 한다.

## 구현 규칙

- **시작 전**: 이 파일의 원칙 표와 맡은 단계의 "현재"·"선행"을 읽는다. 관련 ADR을 읽는다.
- **새 기능 = 코어 플러그인**: `src-tauri/src/sys/plugins.rs`에 `Def`를 추가하고 `default_enabled: false`. 백엔드 진입점은 `plugins::require`로 꺼진 기능을 거절한다. 프런트는 `src/shared/stores/plugins.svelte.ts`로 켜짐을 읽어 메뉴·탭을 숨기고, 화면에 들어오면 `src/app/PluginOff.svelte`를 보여준다. 설정 > 코어 플러그인 목록(`PluginsSection.svelte`의 `GROUPS`)에도 넣어야 켤 수 있다. 꺼진 기능은 타이머·구독·데이터 읽기를 시작하지 않는다.
- **유료 호출**: 반드시 TaskManager 작업 안에서 부른다(`src-tauri/src/task/task_manager.rs`의 `submit` 또는 `run_inline`). 작업에 기능 id(`feature`), 실행 주체(`run_by`: user / auto), 표시 이름(`label`)을 남긴다. 공급자는 `AiService`를 거쳐 부른다(사용량이 자동 기록됨). 자동 실행은 별도 스위치(기본 off)가 켜졌을 때만, `run_by = auto`로. 실패해도 자동 재시도하지 않는다. 캐시에서 꺼낸 결과는 작업을 만들지 않는다. 시간 제한·취소는 `run_inline` 클로저 안에서 건다(밖에서 future를 버리면 작업이 Running으로 남는다).
- **비밀**: API 키·토큰은 `service/secret_store.rs`(OS keyring)에만 둔다. 설정 파일·DB·백업·동기화 묶음·로그·오류 메시지에 넣지 않는다.
- **되돌리기**: 지우기·덮어쓰기는 확인을 받고, 가능하면 원본을 남긴다(meta에 원문, 휴지통, 적용 전 백업).
- **코드 구조** (게임 ADR-017): `commands/`는 얇게, 로직은 `service/`·`game/`, DB 함수는 `get / get_all / upsert / delete + 테이블명`, 모델은 `X / XRow / XInput`. 병렬 조회는 `futures::try_join!`(모바일 빌드 제한), 네트워크는 rustls.
- **DB**: 새 마이그레이션 파일을 `src-tauri/src/db/migrations/`에 번호를 올려 추가한다. 이미 있는 마이그레이션 파일은 주석 한 글자도 고치지 않는다(백업이 체크섬을 비교함). 새 설정 칸은 `service/data/settings_scope.rs` 분류표에, 새 테이블은 `db/change_tracking.rs` 분류에 적는다(테스트가 강제).
- **IPC**: 커맨드·타입을 바꾸면 `cd src-tauri && cargo run --bin export-bindings`로 `src/ipc/bindings.ts`를 다시 만들어 같이 커밋한다(CI가 검사). 브라우저 미리보기에서 보이려면 `src/ipc/dev-mock/`에 가짜 응답도 추가한다.
- **화면**: `src/shared/ui`의 공용 컴포넌트를 먼저 쓴다. 폭·겹침 순서·색은 `tokens.css` 변수로(숫자 하드코딩 금지). 목록 화면은 `ListToolbar`·`SelectBar`·`ItemCard`. 새 화면의 글자는 `src/locales/`의 키로. 데이터 저장소와 화면 상태(열림·선택·정렬)를 한 store에 섞지 않는다. 데스크톱 전용 기능은 Android에서 숨긴다.
- **검증**: `pnpm check`, `pnpm test:run`, `cd src-tauri && cargo fmt --check && cargo test && cargo clippy --all-targets`(경고 수가 늘지 않게). 화면이 바뀌면 미리보기로 직접 확인한다. 테스트에서 실제 유료 API를 부르지 않는다(가짜 공급자).
- **보고는 정직하게**: 테스트가 실패했으면 출력과 함께 그대로 말하고, 건너뛴 단계·확인하지 못한 것을 숨기지 않는다. 확인한 것은 군더더기 없이 확인했다고 쓴다.
