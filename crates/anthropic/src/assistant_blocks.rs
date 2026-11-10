use agent_harness::ToolCall;
use serde_json::{Value, json};

pub(crate) fn assistant_blocks(text: &str, calls: &[ToolCall]) -> Vec<Value> {
    let mut blocks = Vec::new();
    if !text.is_empty() {
        blocks.push(json!({"type": "text", "text": text}));
    }
    for call in calls {
        blocks.push(json!({
            "type": "tool_use",
            "id": call.id,
            "name": call.name,
            "input": call.args,
        }));
    }
    blocks
}
