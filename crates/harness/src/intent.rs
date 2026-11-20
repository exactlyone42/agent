use crate::prelude::*;
use crate::{Effect, StopReason};

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
