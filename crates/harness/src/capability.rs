use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityContext {
    pub turn: usize,
    pub pending_tools: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    TaskReceived(String),
    ModelReplied(Message),
    ToolReported {
        call_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    LimitReached,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    CallModel,
    RunTool(ToolCall),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    AskModel,
    CallTool(ToolCall),
    Answer(String),
    Stop(StopReason),
    Note(String),
}

impl Intent {
    pub fn effect(&self) -> Option<Effect> {
        match self {
            Self::AskModel => Some(Effect::CallModel),
            Self::CallTool(call) => Some(Effect::RunTool(call.clone())),
            Self::Answer(_) | Self::Stop(_) | Self::Note(_) => None,
        }
    }
}

pub trait Capability: Send + Sync {
    fn id(&self) -> &str;

    fn on_event(&mut self, event: &Event, context: &CapabilityContext) -> Vec<Intent>;

    fn forbids(&self, _call: &ToolCall) -> Option<String> {
        None
    }
}
