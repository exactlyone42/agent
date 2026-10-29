use crate::{Claim, Fault};
use agent_harness::ToolCall;

pub trait ApprovalStore: Send + Sync {
    type Prepared: Send;

    fn prepare(&self, turn: usize, call: &ToolCall) -> Result<Self::Prepared, Fault>;

    fn claim(&self, prepared: &Self::Prepared) -> Result<Claim, Fault>;

    fn receipt(&self, turn: usize, call: &ToolCall, succeeded: bool) -> Result<(), Fault>;

    fn reject(&self, call: &ToolCall) -> Result<(), Fault>;
}
