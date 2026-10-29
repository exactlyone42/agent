mod fault_of;
mod task_approvals;
mod task_messages;

use crate::store::TaskStore;
use agent_harness::CancellationToken;
use agent_harness_approval::Policy;
use std::sync::{Arc, Mutex};

pub use task_approvals::TaskApprovals;
pub use task_messages::task_messages;

pub type ApprovalGate = agent_harness_approval::ApprovalGate<TaskApprovals>;

pub fn task_gate(
    store: Arc<Mutex<TaskStore>>,
    task_id: String,
    cancellation: CancellationToken,
    manual: bool,
) -> ApprovalGate {
    let policy = Policy {
        manual,
        ..Policy::default()
    };
    ApprovalGate::new(
        Arc::new(TaskApprovals::new(store, task_id)),
        cancellation,
        policy,
        task_messages(),
    )
}
