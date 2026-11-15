//! 전역 작업 관리자.
//!
//! 작업 상태를 바꾸는 유일한 주체다. 다른 Manager는 아래 공개 메서드로 "요청"만 한다.
//!
//! | 호출자가 하는 일              | TaskManager가 하는 상태 변경                        |
//! |------------------------------|----------------------------------------------------|
//! | `submit`                     | (생성) Pending → Running → Completed/Failed        |
//! | `cancel`                     | Pending/Running → Cancelled, Interrupted → Cancelled |
//! | `resume_task`                | Interrupted/Failed → Pending → …                   |
//! | `dismiss_task`               | Interrupted → Cancelled                            |
//! | `shutdown` / `run_worker`    | Pending/Running → Interrupted                      |
//! | `recover_on_startup`         | (지난 실행의) Pending/Running → Interrupted          |
use std::{
    collections::HashMap,
    panic::AssertUnwindSafe,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use futures::FutureExt;
use tokio::sync::{oneshot, Semaphore};
use tokio_util::sync::CancellationToken;

use crate::{
    db::conn::DbPool,
    error::{AppError, ErrorCode},
    task::{
        task_event::TaskEventSink,
        task_models::{
            error_code_str, AttemptInfo, AttemptStatus, CancelReason, CreateTaskRequest,
            TaskError, TaskErrorInfo, TaskEvent, TaskInfo, TaskOutput, TaskStatus,
        },
        task_repository::{TaskRepository, TransitionPatch},
        task_runner::{ExecutorRegistry, TaskContext, TaskExecutor, TaskReporter},
    },
};

/// 동시에 실행할 수 있는 작업 수 (외부 API 호출 폭주 방지)
const DEFAULT_CONCURRENCY: usize = 4;
/// 재시도 대기 기본값 (attempt마다 2배, 최대 30초)
const DEFAULT_RETRY_BASE_DELAY: Duration = Duration::from_secs(1);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

/// 작업 최종 결과
pub type TaskResult = Result<TaskOutput, TaskErrorInfo>;

/// submit / resume이 돌려주는 핸들. 결과가 필요 없으면 그냥 버려도 작업은 계속 진행된다.
pub struct TaskHandle {
    task: TaskInfo,
    result: oneshot::Receiver<TaskResult>,
}

impl TaskHandle {
    pub fn id(&self) -> &str {
        &self.task.id
    }

    /// 접수 시점의 스냅샷
    pub fn task(&self) -> &TaskInfo {
        &self.task
    }

    /// 작업이 끝날 때까지 기다린다 (Completed → Ok, 그 외 → Err)
    pub async fn wait(self) -> TaskResult {
        self.result.await.unwrap_or_else(|_| Err(channel_closed()))
    }
}

/// 메모리에 올라가 있는(Pending/Running) 작업
struct ActiveTask {
    cancel: CancellationToken,
    reason: Arc<Mutex<Option<CancelReason>>>,
    waiters: Vec<oneshot::Sender<TaskResult>>,
}

pub struct TaskManager {
    repo: TaskRepository,
    executors: ExecutorRegistry,
    events: Arc<dyn TaskEventSink>,
    active: Mutex<HashMap<String, ActiveTask>>,
    semaphore: Arc<Semaphore>,
    retry_base_delay: Duration,
}

impl TaskManager {
    pub fn new(db: DbPool, events: Arc<dyn TaskEventSink>) -> Arc<Self> {
        Self::with_options(db, events, DEFAULT_CONCURRENCY, DEFAULT_RETRY_BASE_DELAY)
    }

    pub fn with_options(
        db: DbPool,
        events: Arc<dyn TaskEventSink>,
        concurrency: usize,
        retry_base_delay: Duration,
    ) -> Arc<Self> {
        Arc::new(Self {
            repo: TaskRepository::new(db),
            executors: ExecutorRegistry::new(),
            events,
            active: Mutex::new(HashMap::new()),
            semaphore: Arc::new(Semaphore::new(concurrency.max(1))),
            retry_base_delay,
        })
    }

    /// 작업 종류별 Executor 등록 (같은 종류를 다시 등록하면 교체)
    pub fn register_executor(&self, executor: Arc<dyn TaskExecutor>) {
        self.executors.register(executor);
    }

    // =======================================
    // 요청 API (다른 Manager / 커맨드용)
    // =======================================

    /// 작업 생성 후 바로 실행 큐에 넣는다.
    pub async fn submit(self: &Arc<Self>, request: CreateTaskRequest) -> Result<TaskHandle, AppError> {
        self.ensure_executor(&request.task_type)?;
        let task = self.repo.insert(&request).await?;
        self.emit_queued(&task);
        let result = self.enqueue(task.id.clone());
        Ok(TaskHandle { task, result })
    }

    /// 작업 취소.
    /// - 실행 중/대기 중: 취소 신호를 보내고, 실제 상태 변경은 실행 루프가 마무리하면서 한다.
    /// - Interrupted: 바로 Cancelled로 (= dismiss)
    pub async fn cancel(&self, task_id: &str) -> Result<(), AppError> {
        if self.signal_cancel(task_id, CancelReason::UserCancelled) {
            return Ok(());
        }
        let task = self.repo.get(task_id).await?;
        match task.status {
            // 메모리에 없는 Pending은 이전 실행의 잔재 — 그대로 취소 처리
            TaskStatus::Pending | TaskStatus::Interrupted => {
                self.repo
                    .transition(task_id, task.status, TaskStatus::Cancelled, TransitionPatch::default())
                    .await?;
                self.events.emit(&TaskEvent::Cancelled {
                    task_id: task_id.to_string(),
                });
                Ok(())
            }
            other => Err(invalid_state(task_id, other, "취소")),
        }
    }

    /// 중단(Interrupted)되었거나 실패(Failed)한 작업을 다시 실행
    pub async fn resume_task(self: &Arc<Self>, task_id: &str) -> Result<TaskHandle, AppError> {
        if self.is_active(task_id) {
            return Err(invalid_state(task_id, TaskStatus::Running, "재실행"));
        }
        let task = self.repo.get(task_id).await?;
        if !matches!(task.status, TaskStatus::Interrupted | TaskStatus::Failed) {
            return Err(invalid_state(task_id, task.status, "재실행"));
        }
        self.ensure_executor(&task.task_type)?;
        let task = self
            .repo
            .transition(task_id, task.status, TaskStatus::Pending, TransitionPatch::default())
            .await?;
        self.emit_queued(&task);
        let result = self.enqueue(task.id.clone());
        Ok(TaskHandle { task, result })
    }

    /// 중단된 작업을 재실행하지 않기로 함 (Interrupted → Cancelled)
    pub async fn dismiss_task(&self, task_id: &str) -> Result<(), AppError> {
        let task = self.repo.get(task_id).await?;
        if task.status != TaskStatus::Interrupted {
            return Err(invalid_state(task_id, task.status, "복구 취소"));
        }
        self.repo
            .transition(task_id, TaskStatus::Interrupted, TaskStatus::Cancelled, TransitionPatch::default())
            .await?;
        self.events.emit(&TaskEvent::Cancelled {
            task_id: task_id.to_string(),
        });
        Ok(())
    }

    /// 이미 진행 중인 작업의 결과를 기다린다 (submit하지 않은 쪽에서 결과가 필요할 때).
    /// 메모리에 없는(이미 끝났거나 큐에 없는) 작업이면 None.
    pub fn subscribe(&self, task_id: &str) -> Option<oneshot::Receiver<TaskResult>> {
        let mut active = lock(&self.active);
        let entry = active.get_mut(task_id)?;
        let (tx, rx) = oneshot::channel();
        entry.waiters.push(tx);
        Some(rx)
    }

    // =======================================
    // 조회
    // =======================================

    pub async fn get_task(&self, task_id: &str) -> Result<TaskInfo, AppError> {
        self.repo.get(task_id).await
    }

    pub async fn get_attempts(&self, task_id: &str) -> Result<Vec<AttemptInfo>, AppError> {
        self.repo.find_attempts(task_id).await
    }

    /// Pending / Running 작업
    pub async fn list_active(&self) -> Result<Vec<TaskInfo>, AppError> {
        self.repo
            .find_by_statuses(&[TaskStatus::Pending, TaskStatus::Running])
            .await
    }

    /// 사용자에게 재개/포기를 물어봐야 하는 작업
    pub async fn list_recoverable(&self) -> Result<Vec<TaskInfo>, AppError> {
        self.repo.find_by_statuses(&[TaskStatus::Interrupted]).await
    }

    /// owner_ref(conversation_id 등)로 묶인 작업
    pub async fn list_by_owner_ref(&self, owner_ref: &str) -> Result<Vec<TaskInfo>, AppError> {
        self.repo.find_by_owner_ref(owner_ref).await
    }

    pub fn is_active(&self, task_id: &str) -> bool {
        lock(&self.active).contains_key(task_id)
    }

    // =======================================
    // 수명 주기
    // =======================================

    /// 앱 시작 시 한 번 호출.
    /// 1. 지난 실행에서 끝나지 못한 Pending/Running 작업과 열린 attempt를 Interrupted로 표시
    /// 2. 복구 대상 목록 반환 (프론트는 `get_recoverable_tasks`로 다시 받을 수 있음)
    ///
    /// 이번 프로세스에서 이미 실행 중인 작업이 있으면 1단계는 건너뛴다 (실행 중인 작업을 망가뜨리지 않기 위해).
    pub async fn recover_on_startup(&self) -> Result<Vec<TaskInfo>, AppError> {
        if lock(&self.active).is_empty() {
            let count = self.repo.interrupt_unfinished().await?;
            if count > 0 {
                eprintln!("[task] 이전 실행에서 중단된 작업 {count}개를 interrupted로 표시");
            }
        }
        self.list_recoverable().await
    }

    /// 실행 중/대기 중인 모든 작업에 종료 신호. 각 작업은 Interrupted로 끝난다.
    pub fn shutdown(&self, reason: CancelReason) {
        let active = lock(&self.active);
        for entry in active.values() {
            set_reason(&entry.reason, reason);
            entry.cancel.cancel();
        }
    }

    /// 백그라운드 서비스 루프: 종료 신호를 받으면 모든 작업을 중단시킨다.
    pub async fn run_worker(&self, shutdown: CancellationToken) -> Result<(), AppError> {
        shutdown.cancelled().await;
        self.shutdown(CancelReason::ServiceShutdown);
        Ok(())
    }

    // =======================================
    // 내부: 실행 루프
    // =======================================

    fn ensure_executor(&self, task_type: &crate::task::task_models::TaskType) -> Result<(), AppError> {
        if self.executors.get(*task_type).is_none() {
            return Err(AppError::Validation {
                code: ErrorCode::TaskExecutorMissing,
                message: format!("{task_type} 작업을 처리할 실행기가 등록되지 않았습니다"),
                details: None,
            });
        }
        Ok(())
    }

    fn emit_queued(&self, task: &TaskInfo) {
        self.events.emit(&TaskEvent::Queued {
            task_id: task.id.clone(),
            task_type: task.task_type,
            owner: task.owner,
            owner_ref: task.owner_ref.clone(),
        });
    }

    /// 메모리에 등록하고 실행 루프를 띄운다.
    fn enqueue(self: &Arc<Self>, task_id: String) -> oneshot::Receiver<TaskResult> {
        let (tx, rx) = oneshot::channel();
        let cancel = CancellationToken::new();
        let reason = Arc::new(Mutex::new(None));
        lock(&self.active).insert(
            task_id.clone(),
            ActiveTask {
                cancel: cancel.clone(),
                reason: reason.clone(),
                waiters: vec![tx],
            },
        );

        let this = Arc::clone(self);
        spawn(async move {
            let result = this.drive(&task_id, &cancel, &reason).await;
            let waiters = lock(&this.active)
                .remove(&task_id)
                .map(|a| a.waiters)
                .unwrap_or_default();
            for w in waiters {
                let _ = w.send(result.clone());
            }
        });
        rx
    }

    fn signal_cancel(&self, task_id: &str, reason: CancelReason) -> bool {
        let active = lock(&self.active);
        match active.get(task_id) {
            Some(entry) => {
                set_reason(&entry.reason, reason);
                entry.cancel.cancel();
                true
            }
            None => false,
        }
    }

    async fn drive(
        &self,
        task_id: &str,
        cancel: &CancellationToken,
        reason: &Mutex<Option<CancelReason>>,
    ) -> TaskResult {
        // 1. 실행 슬롯 대기 (상태는 Pending 유지)
        let _permit = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                return self.stop(task_id, None, TaskStatus::Pending, reason).await;
            }
            permit = self.semaphore.clone().acquire_owned() => match permit {
                Ok(p) => p,
                Err(_) => {
                    set_reason(reason, CancelReason::AppShutdown);
                    return self.stop(task_id, None, TaskStatus::Pending, reason).await;
                }
            },
        };

        // 2. Pending → Running
        let task = match self
            .repo
            .transition(task_id, TaskStatus::Pending, TaskStatus::Running, TransitionPatch::default())
            .await
        {
            Ok(t) => t,
            Err(e) => return Err(log_internal(task_id, "실행 시작", e)),
        };

        let Some(executor) = self.executors.get(task.task_type) else {
            let err = TaskError::UnsupportedTaskType(task.task_type);
            return self.fail(task_id, err.to_info()).await;
        };

        // 3. attempt 루프
        let mut attempts_in_run: u32 = 0;
        loop {
            attempts_in_run += 1;
            let attempt = match self.repo.open_attempt(task_id).await {
                Ok(a) => a,
                Err(e) => {
                    let info = TaskError::from(e).to_info();
                    return self.fail(task_id, info).await;
                }
            };
            self.events.emit(&TaskEvent::Started {
                task_id: task_id.to_string(),
                attempt_no: attempt.attempt_no,
            });

            let ctx = TaskContext::new(
                task_id.to_string(),
                attempt.id.clone(),
                attempt.attempt_no,
                cancel.child_token(),
                TaskReporter::new(task_id.to_string(), self.events.clone()),
            );

            let run = AssertUnwindSafe(executor.execute(&task, &ctx)).catch_unwind();
            tokio::pin!(run);
            let outcome = tokio::select! {
                biased;
                _ = cancel.cancelled() => match executor.cancel_grace() {
                    // 협조적 취소: Executor가 마무리(부분 결과 저장 등)할 시간을 준다
                    Some(grace) => match tokio::time::timeout(grace, &mut run).await {
                        Ok(r) => unwind_result(r),
                        Err(_) => Err(TaskError::Cancelled),
                    },
                    None => Err(TaskError::Cancelled),
                },
                r = &mut run => unwind_result(r),
            };

            match outcome {
                Ok(output) => return self.complete(task_id, &attempt.id, output).await,

                Err(TaskError::Cancelled) => {
                    // Executor가 스스로 멈춘 경우에도 사유가 없으면 사용자 취소로 본다
                    set_reason(reason, CancelReason::UserCancelled);
                    return self
                        .stop(task_id, Some(&attempt.id), TaskStatus::Running, reason)
                        .await;
                }

                Err(err) => {
                    let info = err.to_info();
                    if let Err(e) = self
                        .repo
                        .close_attempt(&attempt.id, AttemptStatus::Failed, Some(&info))
                        .await
                    {
                        log_internal(task_id, "attempt 기록", e);
                    }

                    if err.is_retryable() && attempts_in_run < task.max_attempts {
                        self.events.emit(&TaskEvent::Retrying {
                            task_id: task_id.to_string(),
                            attempt_no: attempt.attempt_no,
                            error: info,
                        });
                        let delay = self.retry_delay(attempts_in_run);
                        tokio::select! {
                            biased;
                            _ = cancel.cancelled() => {
                                return self.stop(task_id, None, TaskStatus::Running, reason).await;
                            }
                            _ = tokio::time::sleep(delay) => continue,
                        }
                    }
                    return self.fail(task_id, info).await;
                }
            }
        }
    }

    async fn complete(&self, task_id: &str, attempt_id: &str, output: TaskOutput) -> TaskResult {
        if let Err(e) = self
            .repo
            .close_attempt(attempt_id, AttemptStatus::Completed, None)
            .await
        {
            log_internal(task_id, "attempt 완료 기록", e);
        }
        let patch = TransitionPatch {
            result: Some(output.clone()),
            ..Default::default()
        };
        if let Err(e) = self
            .repo
            .transition(task_id, TaskStatus::Running, TaskStatus::Completed, patch)
            .await
        {
            return Err(log_internal(task_id, "완료 처리", e));
        }
        self.events.emit(&TaskEvent::Completed {
            task_id: task_id.to_string(),
            output: output.clone(),
        });
        Ok(output)
    }

    /// Running → Failed
    async fn fail(&self, task_id: &str, info: TaskErrorInfo) -> TaskResult {
        let patch = TransitionPatch {
            error: Some(info.clone()),
            ..Default::default()
        };
        if let Err(e) = self
            .repo
            .transition(task_id, TaskStatus::Running, TaskStatus::Failed, patch)
            .await
        {
            log_internal(task_id, "실패 처리", e);
        }
        self.events.emit(&TaskEvent::Failed {
            task_id: task_id.to_string(),
            error: info.clone(),
        });
        Err(info)
    }

    /// 취소 신호로 멈춤: 사유에 따라 Cancelled 또는 Interrupted
    async fn stop(
        &self,
        task_id: &str,
        attempt_id: Option<&str>,
        from: TaskStatus,
        reason: &Mutex<Option<CancelReason>>,
    ) -> TaskResult {
        let reason = lock(reason).unwrap_or(CancelReason::UserCancelled);
        let to = reason.final_status();
        let (attempt_status, info) = match to {
            TaskStatus::Interrupted => (
                AttemptStatus::Interrupted,
                TaskErrorInfo {
                    code: error_code_str(ErrorCode::TaskInterrupted),
                    message: "앱이 종료되어 작업이 중단되었습니다".into(),
                    retryable: true,
                },
            ),
            _ => (AttemptStatus::Cancelled, TaskError::Cancelled.to_info()),
        };

        if let Some(a) = attempt_id {
            if let Err(e) = self.repo.close_attempt(a, attempt_status, None).await {
                log_internal(task_id, "attempt 중단 기록", e);
            }
        }
        if let Err(e) = self
            .repo
            .transition(task_id, from, to, TransitionPatch::default())
            .await
        {
            log_internal(task_id, "중단 처리", e);
        }
        let event = match to {
            TaskStatus::Interrupted => TaskEvent::Interrupted {
                task_id: task_id.to_string(),
            },
            _ => TaskEvent::Cancelled {
                task_id: task_id.to_string(),
            },
        };
        self.events.emit(&event);
        Err(info)
    }

    fn retry_delay(&self, attempt: u32) -> Duration {
        let factor = 2u32.saturating_pow(attempt.saturating_sub(1));
        self.retry_base_delay
            .saturating_mul(factor)
            .min(MAX_RETRY_DELAY)
    }
}

// =======================================
// 도우미
// =======================================

fn unwind_result(r: std::thread::Result<Result<TaskOutput, TaskError>>) -> Result<TaskOutput, TaskError> {
    r.unwrap_or_else(|_| Err(TaskError::failed(ErrorCode::InternalError, "작업 실행 중 패닉이 발생했습니다", false)))
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// 먼저 들어온 사유를 유지 (사용자 취소 뒤 종료 신호가 와도 Cancelled로 남게)
fn set_reason(slot: &Mutex<Option<CancelReason>>, reason: CancelReason) {
    let mut guard = lock(slot);
    if guard.is_none() {
        *guard = Some(reason);
    }
}

fn spawn<F>(fut: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            handle.spawn(fut);
        }
        Err(_) => {
            tauri::async_runtime::spawn(fut);
        }
    }
}

fn invalid_state(task_id: &str, status: TaskStatus, action: &str) -> AppError {
    AppError::Validation {
        code: ErrorCode::TaskInvalidTransition,
        message: format!("{status} 상태의 작업은 {action}할 수 없습니다 ({task_id})"),
        details: None,
    }
}

fn log_internal(task_id: &str, step: &str, e: AppError) -> TaskErrorInfo {
    eprintln!("[task] {task_id} {step} 실패: {e}");
    TaskError::from(e).to_info()
}

fn channel_closed() -> TaskErrorInfo {
    TaskErrorInfo {
        code: error_code_str(ErrorCode::InternalError),
        message: "작업 결과를 받지 못했습니다".into(),
        retryable: false,
    }
}

#[cfg(test)]
mod tests;
