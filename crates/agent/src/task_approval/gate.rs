use crate::prelude::*;
use crate::task_approval::task_approvals::TaskApprovals;
use agent_harness_approval::{Messages, Policy};

pub type TaskGate = agent_harness_approval::ApprovalGate<TaskApprovals>;

pub fn task_gate(
    store: Arc<Mutex<TaskStore>>,
    task_id: String,
    cancellation: CancellationToken,
    manual: bool,
) -> TaskGate {
    let policy = Policy {
        manual,
        ..Policy::default()
    };
    TaskGate::new(
        Arc::new(TaskApprovals::new(store, task_id)),
        cancellation,
        policy,
        Messages::default(),
    )
}
