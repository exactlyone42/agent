use serde_json::{Value, json};

pub fn opinion_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "agree": {"type": "boolean"},
            "reason": {"type": "string"}
        },
        "required": ["agree", "reason"],
        "additionalProperties": false
    })
}
