//! 게임 모드 도메인 로직. tauri command는 여기 없다 (commands/game_engine.rs에 있음) —
//! 이 아래는 전부 순수 도메인 코드다.
//!
//!   game/
//!     manager.rs        — GameManager: 패키지 생성/로드/저장/삭제/내보내기 (런타임 데이터는 안 건드림)
//!     package.rs         — .tgame zip import/export, 매니페스트 검증, 경로 탈출 방지
//!     protocol.rs         — game:// 커스텀 프로토콜 (에셋 서빙 + CSP)
//!     journal.rs           — 턴 결과(일지/이벤트 로그/세션 저장) 기록 공용 함수
//!     validate.rs          — 게임 정의 일관성 검사 (start_new_game이 Error면 시작을 막음)
//!     template.rs          — 템플릿(규칙/capabilities)과 번들된 템플릿 패키지(.tgame)
//!     session.rs           — 책갈피에서 분기(fork)한 새 세션 만들기
//!     guard.rs             — current_game_id 등 커맨드 공용 검증 헬퍼
//!     npc.rs                — npc_say의 실제 로직 (게임의 AI 대화. 얇은 wrapper는 commands/game_engine.rs)
//!     engine.rs               — 순수 함수: 가중치 랜덤/조건 평가/시간대 분할. DB도 async도 모름
//!     runtime/
//!       mod.rs                 — GameRuntime (dispatch/handle_command/emit/process_events)
//!       definition.rs           — GameDefinition: load() 시점에 한 번 긁어오는 정적 데이터 전부
//!       state.rs                  — GameState: 세션마다 바뀌는 상태 (위치/시간/날씨/플래그/인벤토리...)
//!       command.rs                 — GameCommand enum
//!       event.rs                    — GameEvent enum
//!       repository.rs                 — GameRepository trait + Sqlx 구현
//!       systems/
//!         mod.rs                       — GameSystem trait, 시스템 목록/실행 순서
//!         movement.rs / time.rs / map.rs / trigger.rs / encounter.rs / relationship.rs /
//!         dialogue.rs / scene.rs / inventory.rs
pub mod engine;
pub mod guard;
pub mod journal;
pub mod manager;
pub mod npc;
pub mod package;
pub mod protocol;
pub mod runtime;
pub mod session;
pub mod stage;
pub mod template;
pub mod transcript;
pub mod validate;

pub use manager::GameManager;
pub use runtime::GameRuntime;
