use crate::task::{task_event::TaskEventEmitter, task_models::*};
use std::{collections::HashMap, sync::Arc};
use tokio_util::sync::CancellationToken;

#[async_trait::async_trait]
pub trait TaskExecutor: Send + Sync {
    fn task_type(&self) -> TaskType;

    async fn execute(
        &self,
        task: TaskInfo,
        context: TaskContext,
        emitter: TaskEventEmitter,
        cancel: CancellationToken,
    ) -> Result<TaskOutput, TaskError>;
}

pub struct TaskRunner {
    executors: HashMap<TaskType, Arc<dyn TaskExecutor>>,
}

impl TaskRunner {
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
        }
    }

    pub fn register(&mut self, executor: Arc<dyn TaskExecutor>) {
        self.executors.insert(executor.task_type(), executor);
    }

    pub async fn execute(
        &self,
        task: TaskInfo,
        context: TaskContext,
        emitter: TaskEventEmitter,
        cancel: CancellationToken,
    ) -> Result<TaskOutput, TaskError> {
        let task_type = task.task_type.clone();
        let executor = self
            .executors
            .get(&task_type)
            .ok_or_else(|| TaskError::UnsupportedTaskType(task_type))?;
        executor.execute(task, context, emitter, cancel).await
    }
}
