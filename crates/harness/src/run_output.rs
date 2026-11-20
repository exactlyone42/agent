use crate::prelude::*;

#[derive(Debug)]
pub struct RunOutput {
    pub messages: Vec<Message>,
    pub text: String,
}
