use crate::{Message, RunOutput};

pub(crate) fn finish(messages: Vec<Message>) -> RunOutput {
    let text = match messages.last() {
        Some(Message::Assistant { text, .. }) => text.clone(),
        _ => String::new(),
    };
    RunOutput { messages, text }
}
