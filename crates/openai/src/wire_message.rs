use agent_harness::Message;
use serde_json::{Value, json};

pub(crate) fn wire_message(message: &Message) -> Value {
    let mut body = match message {
        Message::User(text) => json!({"role": "user", "content": text}),
        Message::Assistant { text, calls, .. } if calls.is_empty() => {
            json!({"role": "assistant", "content": text})
        }
        Message::Assistant { text, calls, .. } => {
            let calls: Vec<Value> = calls
                .iter()
                .map(|call| {
                    let arguments = call
                        .bad_args
                        .clone()
                        .unwrap_or_else(|| call.args.to_string());
                    json!({
                        "id": call.id,
                        "type": "function",
                        "function": {"name": call.name, "arguments": arguments},
                    })
                })
                .collect();
            let content = if text.is_empty() {
                Value::Null
            } else {
                json!(text)
            };
            json!({"role": "assistant", "content": content, "tool_calls": calls})
        }
        Message::Tool {
            call_id, content, ..
        } => {
            json!({"role": "tool", "tool_call_id": call_id, "content": content})
        }
    };
    if let Message::Assistant {
        reasoning: Some(reasoning),
        ..
    } = message
    {
        body["reasoning"] = json!(reasoning);
    }
    body
}
