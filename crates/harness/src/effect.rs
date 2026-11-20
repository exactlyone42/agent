use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    CallModel,
    RunTool(ToolCall),
}
