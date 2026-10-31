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

