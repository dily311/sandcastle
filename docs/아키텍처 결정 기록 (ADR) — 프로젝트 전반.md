# 아키텍처 결정 기록 (ADR) — 프로젝트 전반

> 기준: 지금까지의 설계 논의와 구현. 상태 표기: **Accepted**(결정·구현됨), **Proposed**(결정됨·구현 전), **Deferred**(보류).

## 0. 배경과 용어

- **제품**: Tauri v2(Windows/Android) 로컬 앱. 프론트 Svelte+TS, 백엔드 Rust, DB SQLite(sqlx). 서버 없음, API 키는 사용자 소유. 기능은 AI 채팅과 게임 모드(사용자들이 만든 게임을 가져와 실행). 윈도우 전용 위젯 기능.
- **IPC**: Tauri Command를 총칭.
- 

## ADR 목록

| ID | 제목 | 상태 |
| --- | --- | --- |
| 001 | 일반 대화와 게임모드 테이블 통합 | Proposed |

### ADR-001 일반 대화와 게임모드 테이블 통합

**상태** Proposed

**배경** 일반 대화와 게임모드 테이블의 유사성. 아키텍처 통일성을 위해서 두 테이블을 통합하고자 함.

**결정**

원 테이블 구조:
```
Conversation
 ├── Chat
 │    └── Messages
 │
 └── Game
      ├── GameSession
      ├── GameSave
      └── GameJournal
```

변경 후 테이블 구조:
```
Conversation
 ├── Template
 │    ├── Chat Template
 │    ├── VN Template
 │    ├── RPG Template
 │    └── Text Game Template
 │
 ├── Messages
 │
 └── GameSession (게임 템플릿에서만 생성)
      ├── GameSave
      └── GameJournal
```

- 한 계층에 통합: 대신 각 기능의 구분을 분명히 할 것.
- Conversation 세션은 공통 진입점으로 하고, GameSession은 템플릿에 따라 선택적 연결
- 이후 템플릿을 중간에 변경하는 경우에는 어떻게 하면 좋을까?

**결과**

- Chat Engine과 Game Engine의 책임소재 분명히. 대화의 공통 기능과 게임의 전용 기능을 구분하도록 할 것.
- chat 템플릿 생성.