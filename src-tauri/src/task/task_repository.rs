//! generation_tasks / task_attempts 저장소.
//!
//! 구조체와 모든 메서드가 `pub(super)`라서 task 모듈 밖(ChatManager, GameRuntime, 커맨드 등)에서는
//! 이 저장소를 만들 수도, 호출할 수도 없다. 상태를 쓰는 SQL은 전부 여기 있고,
//! 여기를 부르는 건 TaskManager뿐이다.
use chrono::Utc;
use sqlx::AssertSqlSafe;
use uuid::Uuid;

use crate::{
    db::conn::DbPool,
    error::AppError,
    task::{
        task_models::{
            AttemptInfo, AttemptRow, AttemptStatus, CreateTaskRequest, TaskErrorInfo, TaskInfo,
            TaskInfoRow, TaskOutput, TaskStatus,
        },
        task_state::validate_transition,
    },
};

const TASK_COLUMNS: &str = r#"
    id, task_type, feature, run_by, label, owner, owner_ref, status, payload_json, result_json,
    error_code, error_message, attempt_count, max_attempts,
    created_at, started_at, updated_at, finished_at
"#;

fn now() -> i64 {
    Utc::now().timestamp_millis()
}

/// 상태 전이와 함께 갱신할 값
#[derive(Default)]
pub(super) struct TransitionPatch {
    pub error: Option<TaskErrorInfo>,
    pub result: Option<TaskOutput>,
}

pub(super) struct TaskRepository {
    db: DbPool,
}

impl TaskRepository {
    pub(super) fn new(db: DbPool) -> Self {
        Self { db }
    }

    // ---------------------------------------
    // 조회
    // ---------------------------------------

    pub(super) async fn find_by_id(&self, task_id: &str) -> Result<Option<TaskInfo>, AppError> {
        let sql = format!("SELECT {TASK_COLUMNS} FROM generation_tasks WHERE id = ?");
        let row = sqlx::query_as::<_, TaskInfoRow>(AssertSqlSafe(sql))
            .bind(task_id)
            .fetch_optional(&self.db)
            .await?;
        row.map(TaskInfo::try_from).transpose()
    }

    pub(super) async fn get(&self, task_id: &str) -> Result<TaskInfo, AppError> {
        self.find_by_id(task_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("작업을 찾을 수 없습니다: {task_id}")))
    }

    pub(super) async fn find_by_statuses(
        &self,
        statuses: &[TaskStatus],
    ) -> Result<Vec<TaskInfo>, AppError> {
        if statuses.is_empty() {
            return Ok(vec![]);
        }
        let placeholders = vec!["?"; statuses.len()].join(", ");
        let sql = format!(
            "SELECT {TASK_COLUMNS} FROM generation_tasks WHERE status IN ({placeholders}) ORDER BY created_at ASC"
        );
        let mut query = sqlx::query_as::<_, TaskInfoRow>(AssertSqlSafe(sql));
        for s in statuses {
            query = query.bind(s.to_string());
        }
        let rows = query.fetch_all(&self.db).await?;
        rows.into_iter().map(TaskInfo::try_from).collect()
    }

    pub(super) async fn find_by_owner_ref(
        &self,
        owner_ref: &str,
    ) -> Result<Vec<TaskInfo>, AppError> {
        let sql = format!(
            "SELECT {TASK_COLUMNS} FROM generation_tasks WHERE owner_ref = ? ORDER BY created_at ASC"
        );
        let rows = sqlx::query_as::<_, TaskInfoRow>(AssertSqlSafe(sql))
            .bind(owner_ref)
            .fetch_all(&self.db)
            .await?;
        rows.into_iter().map(TaskInfo::try_from).collect()
    }

    pub(super) async fn find_attempts(&self, task_id: &str) -> Result<Vec<AttemptInfo>, AppError> {
        let rows = sqlx::query_as::<_, AttemptRow>(
            r#"
            SELECT id, task_id, attempt_no, status, error_code, error_message, started_at, finished_at
            FROM task_attempts WHERE task_id = ? ORDER BY attempt_no ASC
            "#,
        )
        .bind(task_id)
        .fetch_all(&self.db)
        .await?;
        rows.into_iter().map(AttemptInfo::try_from).collect()
    }

    // ---------------------------------------
    // 생성 / 상태 전이
    // ---------------------------------------

    pub(super) async fn insert(&self, req: &CreateTaskRequest) -> Result<TaskInfo, AppError> {
        let id = Uuid::new_v4().to_string();
        let ts = now();
        sqlx::query(
            r#"
            INSERT INTO generation_tasks
                (id, task_type, feature, run_by, label, owner, owner_ref, status, payload_json,
                 attempt_count, max_attempts, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(req.task_type.to_string())
        .bind(&req.feature)
        .bind(req.run_by.to_string())
        .bind(&req.label)
        .bind(req.owner.to_string())
        .bind(&req.owner_ref)
        .bind(TaskStatus::Pending.to_string())
        .bind(serde_json::to_string(&req.payload)?)
        .bind(req.max_attempts.max(1) as i64)
        .bind(ts)
        .bind(ts)
        .execute(&self.db)
        .await?;
        self.get(&id).await
    }

    /// 상태 전이 (compare-and-set).
    ///
    /// `from` 상태일 때만 `to`로 바꾼다. 그 사이 다른 경로가 상태를 바꿨다면 실패한다.
    /// - Running 진입: started_at이 비어 있으면 채움
    /// - Pending 복귀: 이전 오류/결과/종료 시각 초기화
    /// - 그 외 종료 상태: finished_at 기록
    pub(super) async fn transition(
        &self,
        task_id: &str,
        from: TaskStatus,
        to: TaskStatus,
        patch: TransitionPatch,
    ) -> Result<TaskInfo, AppError> {
        validate_transition(from, to)?;
        let ts = now();
        let finished_at: Option<i64> = match to {
            TaskStatus::Pending | TaskStatus::Running => None,
            _ => Some(ts),
        };
        let result_json = patch
            .result
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        let res = sqlx::query(
            r#"
            UPDATE generation_tasks SET
                status = ?,
                updated_at = ?,
                started_at = CASE WHEN ? = 'running' THEN COALESCE(started_at, ?) ELSE started_at END,
                finished_at = ?,
                error_code = ?,
                error_message = ?,
                result_json = ?
            WHERE id = ? AND status = ?
            "#,
        )
        .bind(to.to_string())
        .bind(ts)
        .bind(to.to_string())
        .bind(ts)
        .bind(finished_at)
        .bind(patch.error.as_ref().map(|e| e.code.clone()))
        .bind(patch.error.as_ref().map(|e| e.message.clone()))
        .bind(result_json)
        .bind(task_id)
        .bind(from.to_string())
        .execute(&self.db)
        .await?;

        if res.rows_affected() == 0 {
            let current = self.get(task_id).await?;
            return Err(AppError::Validation {
                code: crate::error::ErrorCode::TaskInvalidTransition,
                message: format!(
                    "작업 상태가 이미 바뀌었습니다 (기대: {from}, 현재: {})",
                    current.status
                ),
                details: None,
            });
        }
        self.get(task_id).await
    }

    /// 앱 시작 시: 이전 실행에서 끝나지 못한 작업(Pending/Running)과 열린 attempt를 Interrupted로 표시.
    /// 메모리 큐는 프로세스와 함께 사라졌으므로 Pending도 다시 큐에 넣지 않고 사용자 판단에 맡긴다.
    pub(super) async fn interrupt_unfinished(&self) -> Result<u64, AppError> {
        let ts = now();
        let mut tx = self.db.begin().await?;
        sqlx::query(
            r#"
            UPDATE task_attempts SET status = 'interrupted', finished_at = ?
            WHERE status = 'running'
            "#,
        )
        .bind(ts)
        .execute(&mut *tx)
        .await?;
        let res = sqlx::query(
            r#"
            UPDATE generation_tasks SET status = 'interrupted', updated_at = ?, finished_at = ?
            WHERE status IN ('pending', 'running')
            "#,
        )
        .bind(ts)
        .bind(ts)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(res.rows_affected())
    }

    // ---------------------------------------
    // Attempt
    // ---------------------------------------

    /// 새 attempt를 열고 작업의 attempt_count를 올린다.
    pub(super) async fn open_attempt(&self, task_id: &str) -> Result<AttemptInfo, AppError> {
        let id = Uuid::new_v4().to_string();
        let ts = now();
        let mut tx = self.db.begin().await?;
        let next_no: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(attempt_no), 0) + 1 FROM task_attempts WHERE task_id = ?",
        )
        .bind(task_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO task_attempts (id, task_id, attempt_no, status, started_at)
            VALUES (?, ?, ?, 'running', ?)
            "#,
        )
        .bind(&id)
        .bind(task_id)
        .bind(next_no)
        .bind(ts)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE generation_tasks SET attempt_count = attempt_count + 1, updated_at = ? WHERE id = ?",
        )
        .bind(ts)
        .bind(task_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(AttemptInfo {
            id,
            task_id: task_id.to_string(),
            attempt_no: next_no as u32,
            status: AttemptStatus::Running,
            error: None,
            started_at: ts,
            finished_at: None,
        })
    }

    pub(super) async fn close_attempt(
        &self,
        attempt_id: &str,
        status: AttemptStatus,
        error: Option<&TaskErrorInfo>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE task_attempts SET status = ?, error_code = ?, error_message = ?, finished_at = ?
            WHERE id = ? AND status = 'running'
            "#,
        )
        .bind(status.to_string())
        .bind(error.map(|e| e.code.clone()))
        .bind(error.map(|e| e.message.clone()))
        .bind(now())
        .bind(attempt_id)
        .execute(&self.db)
        .await?;
        Ok(())
    }
}
