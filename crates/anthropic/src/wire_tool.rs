use agent_harness::ToolSpec;
use serde_json::{Value, json};

pub(crate) fn wire_tool(spec: &ToolSpec) -> Value {
    json!({
        "name": spec.name,
        "description": spec.description,
        "input_schema": spec.parameters,
    })
}
