use serde_json::{Value, json};

pub fn finding_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "verdict": {"type": "string", "enum": ["ok", "update", "delete"]},
            "reason": {"type": "string"},
            "evidence": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "line": {"type": "integer"},
                        "quote": {"type": "string"}
                    },
                    "required": ["path", "line", "quote"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["verdict", "reason", "evidence"],
        "additionalProperties": false
    })
}
