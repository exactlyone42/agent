use crate::prelude::*;
use crate::{
    AgentId, AgentIdentity, CapabilityBinding, Goal, Model, RunError, Work, ask_model::ask_model,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, PartialEq)]
pub struct MortalAgent {
    pub id: AgentId,
    pub identity: AgentIdentity,
    pub goals: Vec<Goal>,
    pub capabilities: Vec<CapabilityBinding>,
}

impl MortalAgent {
    pub fn new(id: AgentId, identity: AgentIdentity) -> Self {
        Self {
            id,
            identity,
            goals: Vec::new(),
            capabilities: Vec::new(),
        }
    }

    pub fn goal(mut self, goal: Goal) -> Self {
        self.goals.push(goal);
        self
    }

    pub fn capability(mut self, binding: CapabilityBinding) -> Self {
        self.capabilities.push(binding);
        self
    }

    pub async fn think(
        &self,
        model: &impl Model,
        work: &Work,
        specs: &[ToolSpec],
        cancel: &CancellationToken,
    ) -> Result<Message, RunError> {
        ask_model(model, work.messages(), specs, cancel).await
    }
}
