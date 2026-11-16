use std::sync::atomic::{AtomicU32, Ordering};

use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;

use super::*;
use crate::task::task_models::{TaskOwner, TaskType};

// ---------------------------------------
// 테스트 도우미
// ---------------------------------------

#[derive(Default)]
struct CollectSink(Mutex<Vec<TaskEvent>>);

impl TaskEventSink for CollectSink {
    fn emit(&self, event: &TaskEvent) {
        lock(&self.0).push(event.clone());
    }
}

impl CollectSink {
    fn kinds(&self, task_id: &str) -> Vec<&'static str> {
        lock(&self.0)
            .iter()
            .filter(|e| e.task_id() == task_id)
            .map(|e| match e {
                TaskEvent::Queued { .. } => "queued",
                TaskEvent::Started { .. } => "started",
                TaskEvent::Progress { .. } => "progress",
                TaskEvent::TextDelta { .. } => "delta",
                TaskEvent::ToolCallStarted { .. } => "tool_start",
                TaskEvent::ToolCallCompleted { .. } => "tool_done",
                TaskEvent::Retrying { .. } => "retrying",
                TaskEvent::Completed { .. } => "completed",
                TaskEvent::Failed { .. } => "failed",
                TaskEvent::Cancelled { .. } => "cancelled",
                TaskEvent::Interrupted { .. } => "interrupted",
            })
            .collect()
    }
}

/// 동작을 클로저로 주입하는 Executor
struct FnExecutor<F>(TaskType, F);

#[async_trait::async_trait]
impl<F, Fut> TaskExecutor for FnExecutor<F>
where
    F: Fn(TaskInfo, String, CancellationToken, TaskReporter) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<TaskOutput, TaskError>> + Send,
{
    fn task_type(&self) -> TaskType {
        self.0
    }

    async fn execute(&self, task: &TaskInfo, ctx: &TaskContext) -> Result<TaskOutput, TaskError> {
        (self.1)(
            task.clone(),
            ctx.attempt_id().to_string(),
            ctx.cancel_token().clone(),
            ctx.reporter().clone(),
        )
        .await
    }
}

async fn setup() -> (Arc<TaskManager>, Arc<CollectSink>) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("src/db/migrations").run(&pool).await.unwrap();
    let sink = Arc::new(CollectSink::default());
    let manager = TaskManager::with_options(pool, sink.clone(), 2, Duration::from_millis(1));
    (manager, sink)
}

fn request(task_type: TaskType) -> CreateTaskRequest {
    CreateTaskRequest::new(task_type, TaskOwner::Chat, json!({ "text": "hi" })).owner_ref("conv-1")
}

// ---------------------------------------
// 테스트
// ---------------------------------------

#[tokio::test]
async fn submit_runs_to_completion() {
    let (manager, sink) = setup().await;
    manager.register_executor(Arc::new(FnExecutor(
        TaskType::ChatStream,
        |task: TaskInfo, _a, _c, reporter: TaskReporter| async move {
            reporter.text_delta("he");
            reporter.text_delta("llo");
            let text = task.payload["text"].as_str().unwrap().to_string();
            Ok(TaskOutput::Text { content: text })
        },
    )));

    let handle = manager.submit(request(TaskType::ChatStream)).await.unwrap();
    assert_eq!(handle.task().status, TaskStatus::Pending);
    let id = handle.id().to_string();

    let out = handle.wait().await.unwrap();
    assert_eq!(out, TaskOutput::Text { content: "hi".into() });

    let task = manager.get_task(&id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    assert_eq!(task.attempt_count, 1);
    assert_eq!(task.result, Some(out));
    assert!(task.started_at.is_some() && task.finished_at.is_some());
    assert!(!manager.is_active(&id));
    assert_eq!(
        sink.kinds(&id),
        vec!["queued", "started", "delta", "delta", "completed"]
    );

    let attempts = manager.get_attempts(&id).await.unwrap();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].status, AttemptStatus::Completed);
}

#[tokio::test]
async fn retryable_error_opens_new_attempts() {
    let (manager, sink) = setup().await;
    let calls = Arc::new(AtomicU32::new(0));
    let c = calls.clone();
    manager.register_executor(Arc::new(FnExecutor(
        TaskType::ChatStream,
        move |_t, _a, _c, _r| {
            let c = c.clone();
            async move {
                if c.fetch_add(1, Ordering::SeqCst) < 2 {
                    Err(TaskError::failed(ErrorCode::AiProviderRateLimit, "429", true))
                } else {
                    Ok(TaskOutput::Empty)
                }
            }
        },
    )));

    let handle = manager
        .submit(request(TaskType::ChatStream).max_attempts(3))
        .await
        .unwrap();
    let id = handle.id().to_string();
    assert_eq!(handle.wait().await, Ok(TaskOutput::Empty));

    let task = manager.get_task(&id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    assert_eq!(task.attempt_count, 3);
    let attempts = manager.get_attempts(&id).await.unwrap();
    let statuses: Vec<_> = attempts.iter().map(|a| a.status).collect();
    assert_eq!(
        statuses,
        vec![AttemptStatus::Failed, AttemptStatus::Failed, AttemptStatus::Completed]
    );
    assert_eq!(sink.kinds(&id).iter().filter(|k| **k == "retrying").count(), 2);
}

#[tokio::test]
async fn non_retryable_error_fails_and_can_be_resumed() {
    let (manager, _sink) = setup().await;
    let calls = Arc::new(AtomicU32::new(0));
    let c = calls.clone();
    manager.register_executor(Arc::new(FnExecutor(
        TaskType::ChatStream,
        move |_t, _a, _c, _r| {
            let c = c.clone();
            async move {
                if c.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(TaskError::failed(ErrorCode::AiProviderAuthFailed, "401", false))
                } else {
                    Ok(TaskOutput::Empty)
                }
            }
        },
    )));

    let handle = manager
        .submit(request(TaskType::ChatStream).max_attempts(5))
        .await
        .unwrap();
    let id = handle.id().to_string();
    let err = handle.wait().await.unwrap_err();
    assert_eq!(err.code, "AI_PROVIDER_AUTH_FAILED");

    let task = manager.get_task(&id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Failed);
    assert_eq!(task.attempt_count, 1, "재시도 불가 오류는 다시 시도하지 않음");
    assert_eq!(task.error.unwrap().message, "401");

    // 사용자가 재시도
    let handle = manager.resume_task(&id).await.unwrap();
    assert_eq!(handle.wait().await, Ok(TaskOutput::Empty));
    let task = manager.get_task(&id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(task.error.is_none(), "재개 시 이전 오류는 지워짐");
    assert_eq!(task.attempt_count, 2);
}

#[tokio::test]
async fn cancel_running_task() {
    let (manager, sink) = setup().await;
    manager.register_executor(Arc::new(FnExecutor(
        TaskType::ChatStream,
        |_t, _a, cancel: CancellationToken, _r| async move {
            cancel.cancelled().await;
            Err(TaskError::Cancelled)
        },
    )));

    let handle = manager.submit(request(TaskType::ChatStream)).await.unwrap();
    let id = handle.id().to_string();
    wait_until(|| sink.kinds(&id).contains(&"started")).await;

    manager.cancel(&id).await.unwrap();
    let err = handle.wait().await.unwrap_err();
    assert_eq!(err.code, "TASK_CANCELLED");

    let task = manager.get_task(&id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Cancelled);
    assert_eq!(
        manager.get_attempts(&id).await.unwrap()[0].status,
        AttemptStatus::Cancelled
    );
    // 끝난 작업은 다시 취소할 수 없음
    assert!(manager.cancel(&id).await.is_err());
}

#[tokio::test]
async fn cancel_while_waiting_for_slot() {
    let (manager, _sink) = setup().await;
    manager.register_executor(Arc::new(FnExecutor(
        TaskType::ChatStream,
        |_t, _a, cancel: CancellationToken, _r| async move {
            cancel.cancelled().await;
            Err(TaskError::Cancelled)
        },
    )));

    // 동시 실행 2개를 채운 뒤 세 번째는 Pending으로 대기
    let a = manager.submit(request(TaskType::ChatStream)).await.unwrap();
    let b = manager.submit(request(TaskType::ChatStream)).await.unwrap();
    let c = manager.submit(request(TaskType::ChatStream)).await.unwrap();
    let c_id = c.id().to_string();
    wait_until_status(&manager, a.id(), TaskStatus::Running).await;
    wait_until_status(&manager, b.id(), TaskStatus::Running).await;
    assert_eq!(manager.get_task(&c_id).await.unwrap().status, TaskStatus::Pending);

    manager.cancel(&c_id).await.unwrap();
    assert!(c.wait().await.is_err());
    let task = manager.get_task(&c_id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Cancelled);
    assert_eq!(task.attempt_count, 0, "실행되지 않은 작업은 attempt가 없음");

    manager.shutdown(CancelReason::AppShutdown);
    let _ = a.wait().await;
    let _ = b.wait().await;
}

#[tokio::test]
async fn shutdown_interrupts_then_resume_completes() {
    let (manager, sink) = setup().await;
    let calls = Arc::new(AtomicU32::new(0));
    let c = calls.clone();
    manager.register_executor(Arc::new(FnExecutor(
        TaskType::ImageGen,
        move |_t, _a, cancel: CancellationToken, _r| {
            let c = c.clone();
            async move {
                if c.fetch_add(1, Ordering::SeqCst) == 0 {
                    // 첫 실행은 끝나지 않음 → 종료 신호로 강제 중단
                    cancel.cancelled().await;
                    Err(TaskError::Cancelled)
                } else {
                    Ok(TaskOutput::Image { asset_id: "a1".into() })
                }
            }
        },
    )));

    let handle = manager.submit(request(TaskType::ImageGen)).await.unwrap();
    let id = handle.id().to_string();
    wait_until(|| sink.kinds(&id).contains(&"started")).await;

    manager.shutdown(CancelReason::ServiceShutdown);
    let err = handle.wait().await.unwrap_err();
    assert_eq!(err.code, "TASK_INTERRUPTED");
    assert_eq!(manager.get_task(&id).await.unwrap().status, TaskStatus::Interrupted);
    assert_eq!(manager.list_recoverable().await.unwrap().len(), 1);

    let handle = manager.resume_task(&id).await.unwrap();
    assert_eq!(handle.wait().await, Ok(TaskOutput::Image { asset_id: "a1".into() }));
    assert!(manager.list_recoverable().await.unwrap().is_empty());
}

#[tokio::test]
async fn recover_on_startup_marks_unfinished_tasks() {
    let (manager, _sink) = setup().await;
    // 지난 실행에서 남은 작업을 흉내: 저장소에 직접 기록 (Executor 없이)
    let pending = manager.repo.insert(&request(TaskType::ChatStream)).await.unwrap();
    let running = manager.repo.insert(&request(TaskType::ChatStream)).await.unwrap();
    manager
        .repo
        .transition(&running.id, TaskStatus::Pending, TaskStatus::Running, TransitionPatch::default())
        .await
        .unwrap();
    let attempt = manager.repo.open_attempt(&running.id).await.unwrap();
    // 실행기가 없는 종류(즉시 실행 작업)는 다시 시도할 수 없어 취소로 정리된다
    let inline = manager.repo.insert(&request(TaskType::Translate)).await.unwrap();
    manager.register_executor(Arc::new(FnExecutor(TaskType::ChatStream, |_t: TaskInfo, _a, _c, _r| async { Ok(TaskOutput::Empty) })));

    let recoverable = manager.recover_on_startup().await.unwrap();
    let ids: Vec<_> = recoverable.iter().map(|t| t.id.clone()).collect();
    assert!(ids.contains(&pending.id) && ids.contains(&running.id));
    assert!(!ids.contains(&inline.id));
    assert_eq!(manager.get_task(&inline.id).await.unwrap().status, TaskStatus::Cancelled);

    let attempts = manager.get_attempts(&running.id).await.unwrap();
    assert_eq!(attempts[0].id, attempt.id);
    assert_eq!(attempts[0].status, AttemptStatus::Interrupted);

    manager.dismiss_task(&pending.id).await.unwrap();
    assert_eq!(manager.get_task(&pending.id).await.unwrap().status, TaskStatus::Cancelled);
    // dismiss는 Interrupted에서만
    assert!(manager.dismiss_task(&pending.id).await.is_err());
}

#[tokio::test]
async fn submit_without_executor_is_rejected() {
    let (manager, _sink) = setup().await;
    let err = manager.submit(request(TaskType::Embedding)).await.err().unwrap();
    assert_eq!(err.to_response().code, ErrorCode::TaskExecutorMissing);
    assert!(manager.list_active().await.unwrap().is_empty());
}

#[tokio::test]
async fn subscribe_gets_result_of_running_task() {
    let (manager, sink) = setup().await;
    let gate = Arc::new(tokio::sync::Notify::new());
    let g = gate.clone();
    manager.register_executor(Arc::new(FnExecutor(
        TaskType::GameDialogue,
        move |_t, _a, _c, _r| {
            let g = g.clone();
            async move {
                g.notified().await;
                Ok(TaskOutput::Text { content: "npc".into() })
            }
        },
    )));
    let handle = manager.submit(request(TaskType::GameDialogue)).await.unwrap();
    let id = handle.id().to_string();
    wait_until(|| sink.kinds(&id).contains(&"started")).await;

    let rx = manager.subscribe(&id).expect("진행 중인 작업");
    gate.notify_one();
    assert_eq!(rx.await.unwrap(), Ok(TaskOutput::Text { content: "npc".into() }));
    assert!(manager.subscribe(&id).is_none());
}

async fn wait_until(mut cond: impl FnMut() -> bool) {
    for _ in 0..500 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("조건을 만족하지 못함");
}

async fn wait_until_status(manager: &TaskManager, id: &str, status: TaskStatus) {
    for _ in 0..500 {
        if manager.get_task(id).await.unwrap().status == status {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("{id}가 {status} 상태가 되지 않음");
}

/// 취소 유예가 있는 Executor: 취소 신호를 받으면 받은 데까지 정리해서 돌려준다
struct GracefulExecutor;

#[async_trait::async_trait]
impl TaskExecutor for GracefulExecutor {
    fn task_type(&self) -> TaskType {
        TaskType::ChatStream
    }

    fn cancel_grace(&self) -> Option<Duration> {
        Some(Duration::from_secs(5))
    }

    async fn execute(&self, _task: &TaskInfo, ctx: &TaskContext) -> Result<TaskOutput, TaskError> {
        ctx.cancel_token().cancelled().await;
        // 부분 결과 저장 같은 마무리 작업
        tokio::time::sleep(Duration::from_millis(20)).await;
        Ok(TaskOutput::Text { content: "partial".into() })
    }
}

#[tokio::test]
async fn cancel_grace_keeps_executor_result() {
    let (manager, sink) = setup().await;
    manager.register_executor(Arc::new(GracefulExecutor));

    let handle = manager.submit(request(TaskType::ChatStream)).await.unwrap();
    let id = handle.id().to_string();
    // 실행이 시작될 때까지 기다렸다가 취소
    for _ in 0..100 {
        if manager.get_task(&id).await.unwrap().status == TaskStatus::Running {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    manager.cancel(&id).await.unwrap();

    let out = handle.wait().await.unwrap();
    assert_eq!(out, TaskOutput::Text { content: "partial".into() });
    assert_eq!(manager.get_task(&id).await.unwrap().status, TaskStatus::Completed);
    assert_eq!(sink.kinds(&id).last(), Some(&"completed"));
}

#[test]
fn task_error_info_keeps_code_for_frontend() {
    let info = TaskErrorInfo { code: "AI_PROVIDER_RATE_LIMIT".into(), message: "한도".into(), retryable: true };
    let res = AppError::from(info).to_response();
    assert_eq!(res.code, ErrorCode::AiProviderRateLimit);
    assert!(res.retryable);

    let unknown = TaskErrorInfo { code: "NOPE".into(), message: "?".into(), retryable: false };
    assert_eq!(AppError::from(unknown).to_response().code, ErrorCode::InternalError);
}

#[tokio::test]
async fn run_inline_records_feature_and_keeps_error_code() {
    let (manager, sink) = setup().await;
    let req = || CreateTaskRequest::new(TaskType::Translate, TaskOwner::System, json!({})).feature("translate").run_by(crate::task::RunBy::Auto).label("번역");

    // 성공: 작업이 남고, 안에서 "지금 작업"이 보인다
    let seen = manager.run_inline(req(), |_c| async { Ok(crate::task::current_task().map(|t| (t.task_id, t.feature))) }).await.unwrap();
    let (task_id, feature) = seen.unwrap();
    assert_eq!(feature.as_deref(), Some("translate"));
    let task = manager.get_task(&task_id).await.unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    assert_eq!(task.run_by, crate::task::RunBy::Auto);
    assert_eq!(task.label.as_deref(), Some("번역"));
    assert_eq!(sink.kinds(&task_id), vec!["queued", "started", "completed"]);

    // 실패: 원래 오류 코드를 그대로 돌려주고 작업은 failed
    let err = manager
        .run_inline(req(), |_c| async { Err::<(), _>(AppError::ai_provider(ErrorCode::AiProviderRateLimit, "한도", true)) })
        .await
        .unwrap_err();
    assert_eq!(err.to_response().code, ErrorCode::AiProviderRateLimit);
    let failed = manager.list_by_owner_ref("none").await.unwrap();
    assert!(failed.is_empty());
}

#[tokio::test]
async fn run_inline_can_be_cancelled() {
    let (manager, _sink) = setup().await;
    let m = manager.clone();
    let handle = tokio::spawn(async move {
        m.run_inline(CreateTaskRequest::new(TaskType::Tts, TaskOwner::System, json!({})).owner_ref("speak"), |_c| async {
            tokio::time::sleep(Duration::from_secs(30)).await;
            Ok(())
        })
        .await
    });
    // 실행 중이 될 때까지 기다렸다가 취소
    let mut id = None;
    for _ in 0..100 {
        if let Some(t) = manager.list_by_owner_ref("speak").await.unwrap().into_iter().find(|t| t.status == TaskStatus::Running) {
            id = Some(t.id);
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let id = id.expect("실행 중이어야 함");
    manager.cancel(&id).await.unwrap();
    let err = handle.await.unwrap().unwrap_err();
    assert_eq!(err.to_response().code, ErrorCode::TaskCancelled);
    assert_eq!(manager.get_task(&id).await.unwrap().status, TaskStatus::Cancelled);
}
