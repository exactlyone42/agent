use std::time::Duration;

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
            max_calls: 64,
            approval_wait: Duration::from_secs(120),
            receipt_wait: Duration::from_secs(2),
        }
    }
}
