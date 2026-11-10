use agent_harness::ToolSpec;
use serde_json::{Value, json};

pub(crate) fn wire_tool(spec: &ToolSpec) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": spec.name,
            "description": spec.description,
            "parameters": spec.parameters,
        },
    })
}
