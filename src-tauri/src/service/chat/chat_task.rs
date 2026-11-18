//! 채팅 생성 작업 (TaskType::ChatStream). 프로젝트 전반 ADR-002.
//!
//! - 커맨드(chat_send / chat_regenerate / phone_peek)는 `run`으로 작업을 제출하고 결과를 기다린다.
//!   프론트가 넘긴 스트리밍 Channel은 대화 id로 맡겨 두고 Executor가 꺼내 쓴다.
//! - 같은 대화에는 생성 작업이 하나만 돈다. 멈추기는 `cancel`(= TaskManager::cancel).
//! - 앱이 생성 중에 꺼지면 작업은 Interrupted로 남고, 사용자가 다시 시도(resume_task)하거나 버린다.
//!   다시 시도할 때는 Channel이 없으니 스트리밍 없이 만들고, 끝나면 chat:dialogue:done이 나간다.
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tauri::{ipc::Channel, AppHandle, Manager};

use crate::{
    commands::chat::into_app_error,
    db::{chat_ui, conn::DbPool},
    error::AppError,
    models::{
        ai_service::StreamPayload,
        conversation::{ChatTurn, Message},
        settings::AppSetting,
    },
    service::chat::{
        chat_manager::{ChatManager, TurnKind},
        rewrite::{self, AutoCtx},
    },
    state::AppState,
    sys::settings::SettingsState,
    task::{CreateTaskRequest, TaskContext, TaskError, TaskExecutor, TaskInfo, TaskManager, TaskOutput, TaskOwner, TaskStatus, TaskType},
};

/// 멈추기를 누른 뒤 받은 데까지 저장할 시간
const CANCEL_GRACE: Duration = Duration::from_secs(10);

/// 작업 payload
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ChatJob {
    Send {
        text: String,
        attachments: Vec<String>,
    },
    Regenerate {
        swipe: bool,
        /// 이번 생성에만 붙이는 지시 (없으면 평범한 다시 생성). 예전에 저장된 작업에는 없으므로 기본값 필요
        #[serde(default, skip_serializing_if = "Option::is_none")]
        extra: Option<String>,
        /// 자동 재생성(3-B-4)이 낸 작업이면 묶인 메시지·시도 횟수. 사용자가 시작한 작업에는 없음
        #[serde(default, skip_serializing_if = "Option::is_none")]
        auto: Option<AutoCtx>,
    },
    Peek,
}

/// 대화 id → 지금 기다리는 커맨드의 스트리밍 Channel
static STREAMS: LazyLock<Mutex<HashMap<String, Channel<StreamPayload>>>> = LazyLock::new(Default::default);

fn streams() -> std::sync::MutexGuard<'static, HashMap<String, Channel<StreamPayload>>> {
    STREAMS.lock().unwrap_or_else(|e| e.into_inner())
}

/// 커맨드가 끝나면(성공·실패·취소) Channel을 치운다
struct StreamSlot(String);
impl Drop for StreamSlot {
    fn drop(&mut self) {
        streams().remove(&self.0);
    }
}

pub struct ChatTaskExecutor {
    app: AppHandle,
}

impl ChatTaskExecutor {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

fn task_err(e: anyhow::Error) -> TaskError {
    TaskError::from(into_app_error(e))
}

/// 이 작업이 저장한 사용자 메시지가 대화의 마지막인가 (= 입력은 저장됐고 응답만 못 받음)
async fn input_already_saved(db: &DbPool, conversation_id: &str, task_id: &str) -> Result<bool, TaskError> {
    let last = chat_ui::last_message(db, conversation_id)
        .await
        .map_err(|e| TaskError::from(AppError::from(e)))?;
    Ok(last.is_some_and(|m| m.role == "user" && m.meta.as_ref().and_then(|v| v.get("taskId")).and_then(|v| v.as_str()) == Some(task_id)))
}

#[async_trait::async_trait]
impl TaskExecutor for ChatTaskExecutor {
    fn task_type(&self) -> TaskType {
        TaskType::ChatStream
    }

    fn cancel_grace(&self) -> Option<Duration> {
        Some(CANCEL_GRACE)
    }

    async fn execute(&self, task: &TaskInfo, ctx: &TaskContext) -> Result<TaskOutput, TaskError> {
        let job: ChatJob = task.payload_as()?;
        let conversation_id = task
            .owner_ref
            .clone()
            .ok_or_else(|| TaskError::InvalidPayload("대화 id(owner_ref)가 없음".into()))?;
        let state = self.app.state::<AppState>();
        let settings = self.app.state::<SettingsState<AppSetting>>();
        let db = &state.db;
        // 자동 재생성은 사용자의 화면에 스트리밍하지 않는다 (사용자 명령의 Channel을 가로채지 않음)
        let auto = match &job {
            ChatJob::Regenerate { auto, .. } => auto.clone(),
            _ => None,
        };
        let channel = if auto.is_some() { None } else { streams().get(&conversation_id).cloned() };

        let kind = match job {
            ChatJob::Peek => {
                let saved = ChatManager
                    .peek(&self.app, db, &settings, &conversation_id, channel, ctx.cancel_token())
                    .await
                    .map_err(task_err)?;
                return Ok(TaskOutput::Messages {
                    message_ids: saved.into_iter().map(|m| m.id).collect(),
                });
            }
            ChatJob::Regenerate { swipe, extra, auto } => TurnKind::Regenerate { swipe, extra, auto },
            ChatJob::Send { text, attachments } => {
                if input_already_saved(db, &conversation_id, &task.id).await? {
                    TurnKind::Reply
                } else {
                    TurnKind::Send(text, attachments)
                }
            }
        };
        let turn = match ChatManager
            .turn(&self.app, db, &settings, &conversation_id, kind, channel, ctx.cancel_token(), Some(&task.id))
            .await
        {
            Ok(t) => t,
            Err(e) => {
                let e = into_app_error(e);
                if let Some(a) = &auto {
                    // 자동 재생성이 실패·취소됨: 원본은 그대로, 사슬은 여기서 끝 (같은 요청을 다시 보내지 않는다)
                    let code = e.to_response().code;
                    let cancelled = matches!(code, crate::error::ErrorCode::AiRequestCancelled | crate::error::ErrorCode::TaskCancelled);
                    rewrite::on_failed(&self.app, db, &conversation_id, a, &format!("{code:?}"), cancelled).await;
                }
                return Err(TaskError::from(e));
            }
        };
        // linter가 걸었고 자동 재생성이 켜져 있으면 다음 시도를 예약, 아니면 사슬이 끝난 것이니 마무리
        if let Some(p) = rewrite::pending_of(&turn.assistant) {
            rewrite::schedule(&self.app, &conversation_id, &turn.assistant, p);
        } else if let Some(a) = &auto {
            rewrite::finish_chain(&self.app, db, &conversation_id, &turn.assistant, a.attempt).await;
        }
        let mut ids: Vec<String> = turn.user.into_iter().map(|m| m.id).collect();
        ids.push(turn.assistant.id);
        Ok(TaskOutput::Messages { message_ids: ids })
    }
}

fn manager(app: &AppHandle) -> Result<Arc<TaskManager>, AppError> {
    app.try_state::<Arc<TaskManager>>()
        .map(|m| m.inner().clone())
        .ok_or_else(|| AppError::internal("TaskManager가 초기화되지 않았습니다"))
}

/// 이 대화에서 대기 중이거나 실행 중인 생성 작업
pub(crate) async fn active_task_of(app: &AppHandle, conversation_id: &str) -> Result<Option<TaskInfo>, AppError> {
    active_task(&*manager(app)?, conversation_id).await
}

/// 자동 재생성 작업을 낸다 (기다리지 않음). `run_by = auto`, 기능 `rewrite` — 예산 한도가 막고, 사용자가 보내면 먼저 멈춘다
pub(crate) async fn submit_auto(app: &AppHandle, conversation_id: &str, job: ChatJob, label: &str) -> Result<(), AppError> {
    let manager = manager(app)?;
    let payload = serde_json::to_value(&job).map_err(|e| AppError::internal(e.to_string()))?;
    manager
        .submit(
            CreateTaskRequest::new(TaskType::ChatStream, TaskOwner::Chat, payload)
                .owner_ref(conversation_id)
                .feature(rewrite::FEATURE)
                .label(label)
                .run_by(crate::task::RunBy::Auto),
        )
        .await?;
    Ok(())
}

async fn active_task(manager: &TaskManager, conversation_id: &str) -> Result<Option<TaskInfo>, AppError> {
    Ok(manager
        .list_by_owner_ref(conversation_id)
        .await?
        .into_iter()
        .find(|t| t.task_type == TaskType::ChatStream && matches!(t.status, TaskStatus::Pending | TaskStatus::Running)))
}

/// 작업을 제출하고 끝날 때까지 기다린다. 저장된 메시지를 저장 순서대로 돌려준다.
pub async fn run(app: &AppHandle, conversation_id: &str, job: ChatJob, channel: Channel<StreamPayload>) -> Result<Vec<Message>, AppError> {
    let manager = manager(app)?;
    let busy = || AppError::validation("이미 응답을 만드는 중", None);
    let _slot = {
        let mut map = streams();
        if map.contains_key(conversation_id) {
            return Err(busy());
        }
        map.insert(conversation_id.to_string(), channel);
        StreamSlot(conversation_id.to_string())
    };
    // 지난 실행에서 다시 시도한 작업이 아직 돌고 있을 수 있다. 자동 재생성이 돌고 있으면 사용자가 우선 — 먼저 멈추고 이어 간다
    if let Some(t) = active_task(&manager, conversation_id).await? {
        if t.feature.as_deref() != Some(rewrite::FEATURE) {
            return Err(busy());
        }
        manager.cancel(&t.id).await?;
        for _ in 0..30 {
            if active_task(&manager, conversation_id).await?.is_none() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if active_task(&manager, conversation_id).await?.is_some() {
            return Err(busy());
        }
    }

    // 실행 기록에 보일 기능·이름: 폰 대화면 phone, 아니면 chat
    let is_phone = crate::db::conversation::get_conversation(&app.state::<AppState>().db, conversation_id)
        .await
        .is_ok_and(|c| c.mode == "phone");
    let feature = if is_phone { crate::sys::plugins::id::PHONE } else { "chat" };
    let label = match &job {
        ChatJob::Send { .. } => "응답 생성",
        ChatJob::Regenerate { extra: Some(_), .. } => "지시와 함께 다시 생성",
        ChatJob::Regenerate { swipe: true, .. } => "다른 응답",
        ChatJob::Regenerate { swipe: false, .. } => "다시 생성",
        ChatJob::Peek => "엿보기",
    };
    let payload = serde_json::to_value(&job).map_err(|e| AppError::internal(e.to_string()))?;
    let handle = manager
        .submit(
            CreateTaskRequest::new(TaskType::ChatStream, TaskOwner::Chat, payload)
                .owner_ref(conversation_id)
                .feature(feature)
                .label(label),
        )
        .await?;
    let ids = match handle.wait().await.map_err(AppError::from)? {
        TaskOutput::Messages { message_ids } => message_ids,
        other => return Err(AppError::internal(format!("채팅 작업의 결과 형식이 다름: {other:?}"))),
    };

    let db = &app.state::<AppState>().db;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(chat_ui::get_message(db, &id).await.map_err(AppError::from)?);
    }
    Ok(out)
}

/// 보내기·다시 생성 결과를 화면용 한 차례로
pub fn into_turn(mut messages: Vec<Message>) -> Result<ChatTurn, AppError> {
    let assistant = messages.pop().ok_or_else(|| AppError::internal("응답 메시지가 없음"))?;
    Ok(ChatTurn {
        user: messages.pop(),
        assistant,
    })
}

/// 진행 중인 생성 멈추기 (받은 데까지는 저장됨). 멈출 게 있었으면 true
pub async fn cancel(app: &AppHandle, conversation_id: &str) -> Result<bool, AppError> {
    let manager = manager(app)?;
    match active_task(&manager, conversation_id).await? {
        Some(task) => manager.cancel(&task.id).await.map(|_| true),
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_payload_round_trip() {
        let jobs = [
            ChatJob::Send {
                text: "안녕".into(),
                attachments: vec!["a.png".into()],
            },
            ChatJob::Regenerate {
                swipe: true,
                extra: None,
                auto: None,
            },
            ChatJob::Regenerate {
                swipe: false,
                extra: Some("묘사는 짧게".into()),
                auto: None,
            },
            ChatJob::Regenerate {
                swipe: true,
                extra: Some("다음 표현은 쓰지 말 것".into()),
                auto: Some(AutoCtx {
                    message_id: "m".into(),
                    swipe_id: 1,
                    attempt: 2,
                    max: 3,
                    rules: vec!["banned".into()],
                }),
            },
            ChatJob::Peek,
        ];
        for job in jobs {
            let v = serde_json::to_value(&job).unwrap();
            assert_eq!(serde_json::from_value::<ChatJob>(v).unwrap(), job);
        }
        assert_eq!(serde_json::to_value(ChatJob::Peek).unwrap(), serde_json::json!({ "kind": "peek" }));
        // 지시가 생기기 전에 저장된 작업도 읽힌다
        let old = serde_json::json!({ "kind": "regenerate", "swipe": true });
        assert_eq!(
            serde_json::from_value::<ChatJob>(old).unwrap(),
            ChatJob::Regenerate {
                swipe: true,
                extra: None,
                auto: None
            }
        );
    }
}
