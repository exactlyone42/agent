use crate::{Message, ToolSpec};
use std::future::Future;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ModelError(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TurnStatus {
    Complete,
    ToolCalls,
    Continue,
    Truncated,
    Blocked,
}

#[derive(Clone, Debug)]
pub struct ModelReply {
    pub message: Message,
    pub status: TurnStatus,
}

pub trait Model: Send + Sync {
    fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
    ) -> impl Future<Output = Result<ModelReply, ModelError>> + Send;
}
