use super::{StoreError, TaskSpec, digest::digest};
use agent_harness::Message;

pub(super) struct RecoveryCall {
    pub call_id: String,
    pub tool_name: String,
    pub args_json: String,
    pub args_hash: String,
    pub scope_path: String,
    pub status: String,
}

impl RecoveryCall {
    pub fn unstarted(&self) -> bool {
        matches!(self.status.as_str(), "prepared" | "denied" | "cancelled")
    }

    pub fn validate(&self, messages: &[Message], spec: &TaskSpec) -> Result<(), StoreError> {
        if self.args_json.len() > 6 * crate::task::FILE_LIMIT as usize + 1024
            || digest(&self.args_json) != self.args_hash
            || matches!(self.status.as_str(), "dispatched" | "outcome_unknown")
        {
            return Err(StoreError::Unsettled);
        }
        let call = messages
            .iter()
            .flat_map(|message| match message {
                Message::Assistant { calls, .. } => calls.as_slice(),
                _ => &[],
            })
            .find(|call| call.id == self.call_id)
            .ok_or(StoreError::Unsettled)?;
        let args: serde_json::Value =
            serde_json::from_str(&self.args_json).map_err(|_| StoreError::InvalidToolCall)?;
        let (scope, allowed) = if self.tool_name == "read_input" {
            (&spec.task.input, spec.task.allow_read)
        } else {
            (&spec.task.output, spec.task.allow_write)
        };
        if call.name != self.tool_name
            || call.bad_args.is_some()
            || call.args != args
            || scope.to_str() != Some(self.scope_path.as_str())
            || (!self.unstarted() && !allowed)
        {
            return Err(StoreError::Unsettled);
        }
        let result = messages.iter().find_map(|message| match message {
            Message::Tool {
                call_id, is_error, ..
            } if call_id == &self.call_id => Some(*is_error),
            _ => None,
        });
        if (self.status == "succeeded" && result != Some(false))
            || (self.status == "failed" && result != Some(true))
            || (self.unstarted() && result == Some(false))
        {
            return Err(StoreError::Unsettled);
        }
        Ok(())
    }
}
