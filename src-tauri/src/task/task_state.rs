use crate::task::task_models::{TaskError, TaskStatus};


pub fn can_transition(from: &TaskStatus, to: &TaskStatus) -> bool {
    todo!()
}

pub fn validate_transition(from: &TaskStatus, to: &TaskStatus) -> Result<(), TaskError> {
    todo!();
}
