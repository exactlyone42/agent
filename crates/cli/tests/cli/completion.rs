use serde_json::{Value, json};

pub fn completion(message: Value, finish_reason: &str) -> Value {
    json!({
        "id": "chat_fixture",
        "object": "chat.completion",
        "created": 1,
        "model": "fixture-model",
        "choices": [{"index": 0, "message": message, "finish_reason": finish_reason}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    })
}
