//! 지금 실행 중인 작업 (tokio task-local).
//!
//! TaskManager가 작업을 실행할 때 이 값을 깔아 두면, 그 안에서 불리는 AI 호출·음성·이미지 코드가
//! 인자를 따로 받지 않고도 "어느 작업·기능의 호출인지"를 알 수 있다 (ResourceManager가 사용량에 붙임).
//! 작업 밖에서 불리면 None (기록은 작업 없이 남는다).
use std::future::Future;

use super::task_models::RunBy;

#[derive(Debug, Clone)]
pub struct CurrentTask {
    pub task_id: String,
    pub feature: Option<String>,
    /// 누가 시작했나 (예산 한도: auto는 항상 막음)
    pub run_by: RunBy,
}

tokio::task_local! {
    static CURRENT: CurrentTask;
}

/// 지금 실행 중인 작업 (없으면 None)
pub fn current() -> Option<CurrentTask> {
    CURRENT.try_with(Clone::clone).ok()
}

/// fut를 이 작업 안에서 실행한다
pub(crate) async fn scope<F: Future>(task: CurrentTask, fut: F) -> F::Output {
    CURRENT.scope(task, fut).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn current_is_visible_only_inside_scope() {
        assert!(current().is_none());
        let id = scope(
            CurrentTask {
                task_id: "t1".into(),
                feature: Some("translate".into()),
                run_by: RunBy::User,
            },
            async { current().map(|c| c.task_id) },
        )
        .await;
        assert_eq!(id.as_deref(), Some("t1"));
        assert!(current().is_none());
    }
}
