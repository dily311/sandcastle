use crate::{
    error::AppError,
    task::{
        task_event::TaskEventEmitter,
        task_models::{CreateTaskRequest, TaskInfo},
        task_repository::TaskRepository,
        task_runner::TaskRunner,
    },
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tauri_plugin_background_service::Notifier;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

pub struct TaskManager {
    repository: Arc<TaskRepository>,
    runner: Arc<TaskRunner>,
    emitter: TaskEventEmitter,

    running_tasks: Mutex<HashMap<String, RunningTask>>,
    semaphore: Arc<Semaphore>,
}

pub struct RunningTask {
    pub task_id: String,
    pub attempt_id: String,
    pub cancel_token: CancellationToken,
}

impl TaskManager {
    pub fn new(
        repository: Arc<TaskRepository>,
        runner: Arc<TaskRunner>,
        emitter: TaskEventEmitter,
    ) -> Self {
        Self {
            repository,
            runner,
            emitter,
            running_tasks: Mutex::new(HashMap::new()),
            semaphore: Arc::new(Semaphore::new(2)),
        }
    }

    /// 작업 생성
    pub async fn create_task(&self, request: CreateTaskRequest) -> Result<TaskInfo, AppError> {
        todo!()
    }

    /// 작업 시작: 실행 준비 후 Worker 전달
    pub async fn start_task(&self, task_id: &str) -> Result<(), AppError> {
        todo!()
    }

    /// 작업 실행: 대기 중인 작업을 가져와 실행하는 루프
    pub async fn run_worker(&self, shutdown: CancellationToken) -> Result<(), AppError> {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            tokio::select! {
                // Cooperative cancellation — always include this branch.
                // 서비스 종료 신호가 오면 실행 중인 모든 작업의 토큰 취소
                _ = shutdown.cancelled() => {
                    let mut tasks = self.running_tasks.lock().await;
                    for (_, handle) in tasks.drain() {
                        handle.cancel_token.cancel();
                    }
                    break;
                }
                // 주기적으로 작업 개수 확인 및 알림 업데이트
                _ = interval.tick() => {
                    let count = self.running_tasks.lock().await.len();
                    if count > 0 {
                        eprint!("현재 {}개의 작업이 실행 중", count);
                    }
                }
            }
        }
        Ok(())
    }

    /// 시작 시 다음 작업을 실행
    /// 1. 이전에 실행 중이던 Task 검색
    /// 2. 해당 Task와 Attempt를 Interrupted로 변경
    /// 3. 복구 대상 목록 반환
    /// 4. 프론트엔드에 목록 전달
    /// - 일반 커맨드에서 이 함수를 호출하는 일은 없음
    pub async fn recover_on_startup(&self) -> Result<Vec<TaskInfo>, AppError> {

        todo!()
    }

    /// 작업 재실행
    pub async fn resume_task(&self, task_id: &str) -> Result<(), AppError> {
        todo!()
    }

    /// 중단된 작업 재실행안함
    pub async fn dismiss_task(&self, task_id: &str) -> Result<(), AppError> {
        todo!()
    }
}
