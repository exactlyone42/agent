use crate::approval_defaults::{DEFAULT_APPROVAL_WAIT, DEFAULT_MAX_CALLS, DEFAULT_RECEIPT_WAIT};
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Policy {
    pub manual: bool,
    pub max_calls: usize,
    pub approval_wait: Duration,
    pub receipt_wait: Duration,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            manual: false,
            max_calls: DEFAULT_MAX_CALLS,
            approval_wait: DEFAULT_APPROVAL_WAIT,
            receipt_wait: DEFAULT_RECEIPT_WAIT,
        }
    }
}
