use crate::prelude::*;
use std::future::Future;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ModelError(pub String);

pub trait Model: Send + Sync {
    fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
    ) -> impl Future<Output = Result<Message, ModelError>> + Send;
}
