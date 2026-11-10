use crate::assistant_blocks::assistant_blocks;
use crate::push_tool_result::push_tool_result;
use agent_harness::Message;
use serde_json::{Value, json};

pub(crate) fn wire_messages(messages: &[Message]) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for message in messages {
        match message {
            Message::User(text) => out.push(json!({"role": "user", "content": text})),
            Message::Assistant { text, calls, .. } => out.push(json!({
                "role": "assistant",
                "content": assistant_blocks(text, calls),
            })),
            Message::Tool {
                call_id,
                content,
                is_error,
            } => {
                let block = json!({
                    "type": "tool_result",
                    "tool_use_id": call_id,
                    "content": content,
                    "is_error": is_error,
                });
                push_tool_result(&mut out, block);
            }
        }
    }
    out
}
