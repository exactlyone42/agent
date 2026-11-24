use crate::prelude::*;
use crate::{
    store::ApprovalMode,
    task::{MANUAL_TASK_TIMEOUT, TASK_TIMEOUT},
};

pub fn task_timeout(mode: ApprovalMode) -> Duration {
    if mode == ApprovalMode::Manual {
        return MANUAL_TASK_TIMEOUT;
    }
    TASK_TIMEOUT
}
