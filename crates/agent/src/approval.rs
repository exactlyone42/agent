mod fault_of;
mod task_approvals;
mod task_messages;

use crate::store::{APPROVAL_TTL_SECONDS, TOOL_CALL_LIMIT, TaskStore};
use agent_harness::CancellationToken;
use agent_harness_approval::Policy;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use task_messages::task_messages;

pub use task_approvals::TaskApprovals;

pub type ApprovalGate = agent_harness_approval::ApprovalGate<TaskApprovals>;

pub fn task_gate(
    store: Arc<Mutex<TaskStore>>,
    task_id: String,
    cancellation: CancellationToken,
    manual: bool,
) -> ApprovalGate {
    let policy = Policy {
        manual,
        max_calls: TOOL_CALL_LIMIT as usize,
        approval_wait: Duration::from_secs(APPROVAL_TTL_SECONDS as u64),
        ..Policy::default()
    };
    ApprovalGate::new(
        Arc::new(TaskApprovals::new(store, task_id)),
        cancellation,
        policy,
        task_messages(),
    )
}
