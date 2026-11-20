use crate::prelude::*;

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
