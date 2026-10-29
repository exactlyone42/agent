use serde_json::{Value, json};

pub fn text_message(text: &str) -> Value {
    json!({"role": "assistant", "content": text})
}
