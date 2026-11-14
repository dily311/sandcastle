//! 작업 상태 전이 규칙.
//!
//! ```text
//!            ┌──────────── cancel ─────────────┐
//!            │                                 v
//!   [Pending] ──start──> [Running] ──ok──> [Completed]
//!      ^  │                  │ │
//!      │  │                  │ └──err──> [Failed] ──resume──┐
//!      │  │                  │                              │
//!      │  └──shutdown──┐     └──shutdown──> [Interrupted] ──┤
//!      │               v                         │          │
//!      │         [Interrupted]                   └─dismiss─> [Cancelled]
//!      └──────────────── resume ────────────────────────────┘
//! ```
//!
//! 이 모듈은 task 모듈 밖에서 보이지 않는다. 다른 Manager가 "이 전이가 되나?"를 묻고
//! 직접 상태를 써 넣는 경로 자체를 없애기 위해서다.
use crate::{
    error::{AppError, ErrorCode},
    task::task_models::TaskStatus,
};

pub(super) fn can_transition(from: TaskStatus, to: TaskStatus) -> bool {
    use TaskStatus::*;
    matches!(
        (from, to),
        (Pending, Running)
            | (Pending, Cancelled)
            | (Pending, Interrupted)
            | (Running, Completed)
            | (Running, Failed)
            | (Running, Cancelled)
            | (Running, Interrupted)
            // 재시도/재개
            | (Failed, Pending)
            | (Interrupted, Pending)
            // 복구 포기
            | (Interrupted, Cancelled)
    )
}

pub(super) fn validate_transition(from: TaskStatus, to: TaskStatus) -> Result<(), AppError> {
    if can_transition(from, to) {
        Ok(())
    } else {
        Err(AppError::Validation {
            code: ErrorCode::TaskInvalidTransition,
            message: format!("작업 상태를 {from}에서 {to}(으)로 바꿀 수 없습니다"),
            details: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use TaskStatus::*;

    #[test]
    fn terminal_states_have_no_exit() {
        for to in [Pending, Running, Completed, Failed, Cancelled, Interrupted] {
            assert!(!can_transition(Completed, to));
            assert!(!can_transition(Cancelled, to));
        }
    }

    #[test]
    fn running_cannot_go_back_to_pending() {
        assert!(!can_transition(Running, Pending));
        assert!(validate_transition(Running, Pending).is_err());
    }

    #[test]
    fn resume_paths() {
        assert!(can_transition(Interrupted, Pending));
        assert!(can_transition(Failed, Pending));
        assert!(can_transition(Interrupted, Cancelled));
        assert!(!can_transition(Failed, Running));
    }
}
