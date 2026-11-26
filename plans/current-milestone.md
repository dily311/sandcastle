# 현재 마일스톤

> 중장기 계획은 `plans/roadmap.md`, 끝난 마일스톤의 기록은 `docs/마일스톤 기록.md`, 작업 규칙은 `.claude/CLAUDE.md`.
> 이 파일에는 **지금 하고 있거나 곧 할 일**만 둔다. 마일스톤이 끝나면 기록을 `docs/마일스톤 기록.md`로 옮기고 여기서는 지운다.
> "결정 필요"는 사용자와 합의하기 전에 구현하지 않는다. 번호가 없는 일은 roadmap에 번호를 받은 뒤 시작한다 (규칙은 `.claude/CLAUDE.md`).

**기준일** 2026-11-26 · **버전** `0.4.0` · **브랜치** `dev` (main은 PR로만) · **마지막 완결 기록** M25 (`docs/마일스톤 기록.md`)

## 진행 중 마일스톤: M26 다국어 (5-C-2, 정식 릴리스 기준 Q1)

세부 항목과 완료 기준은 `plans/roadmap.md` 5-C-2-0~10.

### 완료된 것 (`feature/front` 병합, 2026-11-25~26)
- **[x] 5-C-2-1 기반**: `src/lib/utils/i18n.ts`(`ko`·`en`, `localStorage("sc.locale")`, `<html lang>` 동기화), 설정 > 일반 언어 즉시 전환(`GeneralSection.svelte`), `src/shared/time.ts` 다국어 상대·절대 시각, `src/lib/utils/i18n.test.ts`(`ko.json`·`en.json` 각 **3,756개 키** 일치·소스 내 모든 정적 `$_()` 키 존재·언어 전환·백엔드 변환 단위 테스트), E2E 스모크 언어 고정(`d8c6aa2`·`f29cf07`).
- **[x] 5-C-2-6 폰 화면**: `PhonePage`·`PhoneRoom`·`PhonePanel`·`PhoneSettings`·`PhoneAlbum`·`PhoneCamera`·`PhoneProfile`·`PhoneInstagram` 전 화면 다국어 전환 완료(`phone.svelte.ts` 기본 문구 2줄만 남음).
- **[~] 5-C-2-3~5·7·8 주요 화면 1차 전환**: 전체 Svelte 190개 중 `$_(` 사용 파일 **6개 → 133개**로 전환 완료(공용 UI·사이드바/타이틀바·설정 전 탭·캐릭터/페르소나/로어북/대화/메시지/패널/텍스트 카드·게임 목록/상세/VN 플레이/각본가/게임 편집기 전 섹션·홈·이미지/갤러리·대필·빠른 답장·위젯 음악/시메지).
- **[~] 5-C-2-9 백엔드 라벨 1차 변환**: `src/lib/utils/backendI18n.ts`로 백엔드 `ErrorCode` 기반 오류 메시지, 코어 플러그인 이름·설명, 작업 기능·상태 라벨을 프런트에서 현재 언어로 변환(`client.ts`·`taskLog.ts`·`PluginsSection.svelte`·`game/errors.ts`).

### 남은 것 (M26 완료까지)
- **[~] 5-C-2-0 설계 확정 및 ADR-032 기록** (합의 필요):
  - `feature/front`에서 먼저 구현된 방식 — ① `ko`+`en` ② `AppSetting.language` 대신 `localStorage("sc.locale")` 저장, 초기 언어는 브라우저 `en*`이면 `en`·그 밖은 `ko` ③ 영역별 분할·지연 로드 대신 단일 `src/locales/ko.json`·`en.json` 정적 등록 ④ 프런트 `backendI18n.ts` 변환 ⑤ AI 초안 ⑥ 번역 플러그인 기본 `targetLang`은 `"한국어"` 유지 — 을 그대로 확정할지, 원래 추천안(영역별 지연 로드·`AppSetting.language` 연동·비한국어 기본 `en`·`targetLang` 화면 언어 추종)으로 맞출지 결정 후 ADR-032 작성.
- **[ ] 5-C-2-2 하드코딩 한글 CI 검사**: 정적 `$_()` 키 누락 검사는 `i18n.test.ts`로 돌고 있으나, 소스 내 하드코딩 한글 줄 수를 세는 기준표 스크립트(`scripts/`)와 CI 단계는 아직 없음.
- **[ ] 5-C-2-3~8 남은 하드코딩 한글 정리** (시안 `src/design-preview/` 6개 제외 Svelte 41개·TS 48개):
  - 커뮤니티 플러그인 화면 전체(`src/features/community/*.svelte` 9개·`*.ts` 7개, `CommunityPluginSettings.svelte`)
  - 상태창 고정 표시(`StatusFloat`·`StatusTop`·`StatusWindow`·`status/vars.ts`)
  - 채팅·읽기 보조 화면(`ReadView` 탭 라벨·`SentPromptDialog`·`LintNote`·`UtilMacroNote`·`ChatView` 일부 역할/출처 이름·`PresetInline`) 및 TS(`library.svelte.ts`·`chatImport.svelte.ts`·`recovery.ts`·`textCard.ts` 일부 옵션·`lorebook.svelte.ts` 내보내기 설명)
  - 공용 UI·앱 틀(`AppShell` 부팅 문구·`DropLayer`·`AvatarFitDialog`·`ChipPicker`·`ContextMenu`·`SplitButton`·`Toasts` 기본 라벨) 및 TS(`accents.ts`·`backgrounds.ts`·`grays.ts`·`themes.ts`·`providers.ts`·`pluginSettingForm.svelte.ts`·`presetImport.ts`·`dialog.ts`·`jobs.svelte.ts`·`trash.ts`·`updateCheck.ts`·`androidDrive.ts`)
  - 슬래시 오류 문구(`slash/commands.ts`·`script.ts`), 게임 TS(`playUtils.ts`·`vnTheme.svelte.ts`·`runtimeStore.svelte.ts`·`game.svelte.ts`), 시메지 내부 기본 대사/라벨 일부(`simejiType.ts`·`WidgetGame.ts`·`PeerChatManager.ts`·`setFeautres.ts`)
- **[ ] 5-C-2-9 백엔드 원문 나머지**: 구체 인자가 필요한 검증 오류(`INVALID_INPUT` 등), 백엔드가 만든 작업 `label` 원문, OS 알림(`service/notify.rs` 기본 제목·본문), 가져오기 경고 목록.
- **[ ] 5-C-2-10 영어 검수와 배치**: 남은 한글을 0으로 줄인 뒤 영어 화면 전체 순회(긴 라벨 넘침·줄바꿈 점검), 사용자 검수 반영, `docs/` 번역 기여 안내 작성.

## 최근 완료 (v0.4.0 함께 반영 — `plans/roadmap.md` 5-G-17~23)

- [x] **5-G-17** 프런트엔드 디자인 일관성 개편 및 패널·이미지 뷰어 개선 (`a1f32ff`)
- [x] **5-G-18** 유틸 프롬프트에 대필(`ghost`)·폰 프로필(`phoneProfile`)·폰 SNS(`phoneSns`) 통합(총 10개 슬롯), 로어북 기본 스캔 깊이 `10 → 2`, 예산 슬라이더·폰 다크모드 개선 (`126b598`)
- [x] **5-G-19** 채팅 배경 스크림·반투명 UI 크롬·스크림 색 선택기·대화 오른쪽 패널 테마 탭 (`fd36e11`)
- [x] **5-G-20** 사이드바·대화 목록·캐릭터 상세 대화 정렬 통일 (`bcf9289`) 및 대화 패널 API 설정·페르소나 관리 바로가기 (`c6ac51e`)
- [x] **5-G-21** 게임 VN 테마 프리셋·화면 비율 프레이밍·게임 오른쪽 패널 테마 탭 (`914155f`)
- [x] **5-G-22** 위젯 시메지(마스코트, 기본 꺼짐) 개편·설정 화면 및 `TaskManager` AI 호출 규칙(`TaskType::Mascot`, `pairChatUseAi` 기본 꺼짐·`run_by = auto`) 연결 (`aa08ec7`·`2fa17d4`)
- [x] **5-G-23** 버전 `0.4.0` 및 `CHANGELOG.md` 갱신 (`568b05f`)

## 정식 릴리스 기준 (합의 전)

`plans/roadmap.md` "정식 릴리스(1.0) 기준" — 기능 기준 R1 폰 ↔ 대화 연동(5-H), R2 게임 템플릿 전환(4-B·4-C-1), 품질 기준 Q1 다국어(진행 중) · Q2 실환경 확인 · Q3 clippy·흔들리는 시험·CI · Q4 서명·릴리스 · Q5 형식 동결. 목록 확정과 5-H-0(무엇을 "메모리"로 건네나)이 결정 필요.

## 남은 것·후속 (번호 없는 작은 일 — 하려면 번호를 받음)

**1-F 안정화**
- SDK 객체의 `apiVersion`·기능 목록 노출, `docs/플러그인 만들기.md`의 SDK 범위 정책 절(1-F-1-1).
- 스키마 판이 다른 기기와의 실제 병합(양방향), 드라이브 이어 올리기 시험(1-F-3-4).
- 파일에서 가져오기·미리보기(`sync_exchange`)도 `merge_bundle`로(지금은 행을 한꺼번에 읽음).
- 편집기에서 꺼진 기능 탭 숨기기(4-B-1 일부), 실제 AI로 각본가 플랜 확인.

**품질** (정식 릴리스 기준 Q3)
- clippy 경고 **205**(기준 180): 기존 게임 코드 경고 11개(191)에 `feature/front`·시메지 병합으로 14개가 더 늘어남(`2fa17d4` 측정).
- 흔들리는 시험 — 4ms 쉬기로 밀리초 시각을 벌리는 동기화 시험들이 부하가 크면 전체 실행 중 가끔 실패(혼자 3번은 통과): `a_corrupted_file_in_the_folder_stops_before_the_database_changes_and_resumes_after_repair`, `first_reply_applies_edits_and_deletes_thanks_to_acks`(11-22), `same_row_edited_on_both_keeps_both`(11-23). 시계 주입으로 바꿀 것.

**5-G 소급 기록에서 남은 확인**
- 5-G-13 API·모델 따로 저장: ADR-020·028에 바뀐 점을 적어야 하는지.
- 5-G-14 배경 태그를 쓰지 않게 되면서 1-I-4(배경 목록 합치기) 코드가 아직 필요한지.
- 5-G-15 대화별 끄기 스위치 없앰: 옛 대화 meta의 `lint`·`rewriteOff`·`memoryOff`·`summaryOff` 값 처리, 3-B-5·ADR-015·016·017의 "대화별 덮어쓰기" 서술 갱신.
- 5-G-18 유틸 프롬프트 추가: `phoneProfile`·`phoneSns` 기본 지시문이 한국어로 들어감(5-E-21 ③ 영어 지시문 원칙과 맞출지, ADR-030 슬롯 목록 갱신).

**기능 후속**
- 5-D: 대필 전체를 실제 앱·실제 공급자에서 확인, 마지막 기록이 아직 답 못 받은 내 글일 때 처리, 여러 캐릭터 대화에서 모든 캐릭터 이름으로 자르기, 모바일 하단 시트 모양.
- 3-B-4: 사슬 전체를 실제 앱에서 확인, 규칙별 자동 재생성 결과를 실행 기록에서 보기.
- 3-D: 두 기기 사이 요약 동기화, 폰 대화 패널의 요약 구역, 요약 입력에 프롬프트용 정규식 통과.
- 3-B-5: 폰 엿보기·게임 대사에는 linter가 적용되지 않음.
- 3-C-5: "이번 보내기만 회상 건너뛰기", 컨텍스트 탭에서 회상한 대화로 이동, 회상 기록을 사용자 메시지에, 질의를 프롬프트용 정규식에 통과.
- 3-C: 커맨드를 거치지 않는 경로가 바꾼 글의 색인 즉시 맞추기, 검색 결과에서 오래된 메시지로 이동, 자동 색인, f16/int8 저장, 4000자 넘는 메시지의 뒷부분.
- 5-E-13: 게임 캐릭터 모션 이어 하기(남은 내용 확인 필요). 5-E-20-3: 미리보기에 폰 채팅·게임 대사가 없어 화면 확인 못 함.

## 확인하지 못한 것 (실제 환경 — 정식 릴리스 기준 Q2에서 한 번에)

- 공급자: 실제 OpenAI·Claude·Gemini·Vertex 호출(3-A는 가짜 서버로만), GPT-5·o 계열 `max_completion_tokens`, 실제 Gemini 스트리밍 체감(1-I-2), 다인봇 `이름: 대사` 형식(5-F-2), `<updateVar>` 태그(5-F-4-4), `/gen`·`/genraw`·`/inject`, 추론 강도, 한국어 대화에서 영어 지시문의 답 언어(5-E-21), 시메지 AI 대화·선물 응답(5-G-22).
- 실제 앱: 3-B linter 화면·순서, 3-B-4 사슬, 3-C 색인·검색·회상, 3-D 자동 요약, 5-D 대필 전체(`ChatManager::ghost`는 앱 핸들이 있어야 해서 단위 테스트가 없음), Windows 바탕화면 위젯 시메지·음악 플레이어(5-G-10·22), 5-G 항목별, 영어 화면 전체 순회(5-C-2-10).
- Android 실기기: 고른 글 메뉴(1-I-5), 터치로 끌기, 폰·게임 화면, 알림(소리 포함 5-G-16).
- 두 기기 동기화: 배경 목록(1-I-4), 드라이브 이어 받기(1-I-9), 기기 이름(5-E-16).
- SillyTavern: 내보낸 채팅·카드·프리셋·zip을 실제 SillyTavern에서 열기.

## 잊지 말 것

- 배포용 서명 키는 아직 정하지 않음 (M11·M15 "하지 못한 것", 정식 릴리스 기준 Q4). 실기기 Android 확인·GitHub E2E/릴리스 워크플로 실행은 사용자가 끝냄(2026-11-22). Drive 테스트 데이터도 사용자가 지움.
- 워크트리 `.claude/worktrees/front`(`feature/front`, `1a053cf`)는 `dev`에 병합 및 검토 수정(`2fa17d4`)까지 끝남 — 로컬에서 `pnpm test:run`을 돌리면 이 워크트리 안의 옛 시험(`errors.test.ts`)까지 같이 돌아 1건 실패하므로 워크트리를 지우거나 `vite.config.ts`에서 제외할 것. 합친 브랜치 `feature/front`·`claude/game-assets-storyplan-21f359`·`fix/mobile-release`·`feature/plugins`·`fix/release-ndk`는 정리 전까지 남아 있음.

## 사용자 개선점 (번호를 받으면 roadmap으로)

- 홈 페이지를 노션처럼 미리 정의된 위젯 블록으로 사용자가 자유롭게 꾸미기(지금처럼 심플한 것은 옵션으로 남김).
- 드라이브 동기화 때 사용자가 정한 비밀번호로 암호화해서 올리기(ADR-011 ⑤ "암호화는 선택·기본 꺼짐"과 이어짐).

## 메모

- (아이디어) chat 오른쪽 패널에 phone 페이지 내장. 지금 대화 중인 캐릭터와 user 페르소나 중에서만 폰 전환 가능하게. — 5-H(폰 ↔ 대화 연동)와 함께 볼 것.
