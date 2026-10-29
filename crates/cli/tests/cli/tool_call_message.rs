use serde_json::{Value, json};

pub fn tool_call_message(id: &str, name: &str, arguments: &Value) -> Value {
    json!({
        "role": "assistant",
        "content": null,
        "tool_calls": [{
            "id": id,
            "type": "function",
            "function": {"name": name, "arguments": arguments.to_string()}
        }]
    })
}
