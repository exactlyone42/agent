use serde_json::Value;
use std::{future::Future, pin::Pin};

#[derive(Clone, Debug, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ToolError(pub String);

impl ToolError {
    pub fn refused(why: &str) -> Self {
        Self(why.to_owned())
    }

    pub fn failed(error: impl std::error::Error) -> Self {
        Self(error.to_string())
    }
}

pub type ToolFuture<'a> = Pin<Box<dyn Future<Output = Result<String, ToolError>> + Send + 'a>>;

pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    fn call(&self, args: Value) -> ToolFuture<'_>;
}
