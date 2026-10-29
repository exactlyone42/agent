use crate::{
    store::ApprovalMode,
    task::{MANUAL_TASK_TIMEOUT, TASK_TIMEOUT},
};
use std::time::Duration;

pub fn task_timeout(mode: ApprovalMode) -> Duration {
    if mode == ApprovalMode::Manual {
        return MANUAL_TASK_TIMEOUT;
    }
    TASK_TIMEOUT
}
