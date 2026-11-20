use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Work {
    messages: Vec<Message>,
}

impl Work {
    pub fn new(prompt: &str) -> Self {
        Self {
            messages: vec![Message::User(prompt.to_owned())],
        }
    }

    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
}
