use super::fault_of::fault_of;
use crate::prelude::*;
use crate::{store::Claim as StoreClaim, store::PreparedIntent};
use agent_harness::ToolCall;
use agent_harness_approval::{ApprovalStore, Claim, Fault};
use std::sync::{Arc, Mutex};

pub struct TaskApprovals {
    store: Arc<Mutex<TaskStore>>,
    task_id: String,
}

impl TaskApprovals {
    pub fn new(store: Arc<Mutex<TaskStore>>, task_id: String) -> Self {
        Self { store, task_id }
    }
}

impl ApprovalStore for TaskApprovals {
    type Prepared = PreparedIntent;

    fn prepare(&self, turn: usize, call: &ToolCall) -> Result<PreparedIntent, Fault> {
        let mut store = self.store.lock().map_err(|_| Fault::Failed)?;
        store
            .prepare_intent(
                &self.task_id,
                turn as i64,
                &call.id,
                &call.name,
                &call.args.to_string(),
            )
            .map_err(fault_of)
    }

    fn claim(&self, prepared: &PreparedIntent) -> Result<Claim, Fault> {
        let mut store = self.store.lock().map_err(|_| Fault::Failed)?;
        Ok(match store.claim(prepared).map_err(fault_of)? {
            StoreClaim::Pending => Claim::Pending,
            StoreClaim::Granted => Claim::Granted,
            StoreClaim::Denied => Claim::Denied,
        })
    }

    fn receipt(&self, turn: usize, call: &ToolCall, succeeded: bool) -> Result<(), Fault> {
        let mut store = self.store.lock().map_err(|_| Fault::Failed)?;
        store
            .receipt(&self.task_id, turn as i64, &call.id, succeeded)
            .map_err(fault_of)
    }

    fn reject(&self, call: &ToolCall) -> Result<(), Fault> {
        let mut store = self.store.lock().map_err(|_| Fault::Failed)?;
        store
            .record_rejection(&self.task_id, &call.id)
            .map_err(fault_of)
    }
}
