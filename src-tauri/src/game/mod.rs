//! 게임 모드 전체를 여기 아래 한곳에 모은다.
//!
//!   game/
//!     manager.rs        — GameManager: 패키지 생성/로드/저장/삭제 (런타임 데이터는 안 건드림)
//!     package.rs         — .tgame zip import/export, 매니페스트 검증, 경로 탈출 방지
//!     protocol.rs         — game:// 커스텀 프로토콜 (에셋 서빙 + CSP)
//!     guards.rs            — current_game_id 등 커맨드 공용 검증 헬퍼
//!     commands.rs           — engine_move 등, 얇은 tauri command → GameRuntime 위임
//!     npc.rs                 — npc_say. GameRuntime과 별개 경로 (AI 대화는 상태 머신 밖)
//!     engine.rs               — 순수 함수: 가중치 랜덤/조건 평가/시간대 분할. DB도 async도 모름
//!     runtime/
//!       mod.rs                 — GameRuntime (dispatch/handle_command/emit/process_events)
//!       definition.rs           — GameDefinition: load() 시점에 한 번 긁어오는 정적 데이터 전부
//!       state.rs                  — GameState: 세션마다 바뀌는 상태 (위치/시간/날씨/플래그/인벤토리...)
//!       command.rs                 — GameCommand enum
//!       event.rs                    — GameEvent enum
//!       repository.rs                 — GameRepository trait + Sqlx 구현
//!    systems/
//!       mod.rs                       — GameSystem trait, 시스템 목록/실행 순서
//!       time.rs / map.rs / trigger.rs / encounter.rs / relationship.rs / scene.rs / inventory.rs

pub mod engine;
pub mod guard;
pub mod manager;
pub mod npc;
pub mod package;
pub mod protocol;
pub mod runtime;
pub mod journal;
pub mod session;
pub mod validate;

pub use manager::GameManager;
pub use runtime::GameRuntime;
