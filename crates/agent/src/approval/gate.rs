use crate::prelude::*;
use agent_harness::CancellationToken;
use agent_harness_approval::{Messages, Policy};
use std::sync::{Arc, Mutex};

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
        Messages::default(),
    )
}
