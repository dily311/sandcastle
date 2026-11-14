use crate::{
    db::conn::DbPool, error::AppError, task::task_models::{AttemptInfo, AttemptResult, AttemptStatus, NewTask, TaskErrorInfo, TaskInfo, TaskStatus},
};

pub struct TaskRepository {
    db: DbPool,
}

impl TaskRepository {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }
    // DB 관리
    pub async fn create(&self, task: &NewTask) -> Result<TaskInfo, AppError> {
        todo!()
    }

    pub async fn find_by_id(&self, task_id: &str) -> Result<Option<TaskInfo>, AppError> {
        todo!()
    }

    pub async fn find_by_status(&self, status: TaskStatus) -> Result<Vec<TaskInfo>, AppError> {
        todo!()
    }

    pub async fn update_status(&self, task_id: &str, status: TaskStatus) -> Result<(), AppError> {
        todo!()
    }

    pub async fn update_error(&self, task_id: &str, error: &TaskErrorInfo) -> Result<(), AppError> {
        todo!()
    }

    pub async fn find_active(&self) -> Result<Vec<TaskInfo>, AppError> {
        todo!()
    }

    // Attempt 관리
    pub async fn create_attempt(&self, submission_id: &str) -> Result<AttemptInfo, AppError> {
        todo!()
    }

    pub async fn find_attempt(&self, attempt_id: &str) -> Result<Option<AttemptInfo>, AppError> {
        todo!()
    }

    pub async fn find_attempts(&self, submission_id: &str) -> Result<Vec<AttemptInfo>, AppError> {
        todo!()
    }

    pub async fn update_attempt_status(
        &self,
        attempt_id: &str,
        status: AttemptStatus,
    ) -> Result<(), AppError> {
        todo!()
    }

    pub async fn finish_attempt(
        &self,
        attempt_id: &str,
        result: AttemptResult,
    ) -> Result<(), AppError> {
        todo!()
    }

    // 복구대상 관리
    pub async fn find_interrupted(&self) -> Result<Vec<TaskInfo>, AppError> {
        todo!()
    }

    pub async fn mark_interrupted(&self, task_id: &str) -> Result<(), AppError> {
        todo!()
    }

    pub async fn dismiss_recovery(&self, task_id: &str) -> Result<(), AppError> {
        todo!()
    }
}
