use crate::prelude::*;

pub struct Dispatch<'a> {
    pub turn: usize,
    pub call: &'a ToolCall,
}

#[derive(Debug, PartialEq)]
pub enum DispatchAction {
    Proceed,
    Skip(String),
    Stop(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToolResult {
    Succeeded,
    Failed,
}

pub struct Outcome<'a> {
    pub turn: usize,
    pub call: &'a ToolCall,
    pub result: ToolResult,
}

#[derive(Debug, PartialEq)]
pub enum OutcomeAction {
    Proceed,
    Stop(String),
}

pub trait Hook: Send + Sync {
    fn on_dispatch(&self, _: Dispatch<'_>) -> impl Future<Output = DispatchAction> + Send {
        async { DispatchAction::Proceed }
    }

    fn on_outcome(&self, _: Outcome<'_>) -> impl Future<Output = OutcomeAction> + Send {
        async { OutcomeAction::Proceed }
    }
}

impl Hook for () {}
