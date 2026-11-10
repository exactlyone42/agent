use serde_json::{Value, json};

pub(crate) fn push_tool_result(out: &mut Vec<Value>, block: Value) {
    let open = out.last_mut().and_then(|last| {
        let results_only = last["role"] == "user"
            && last["content"]
                .as_array()
                .is_some_and(|blocks| blocks.iter().all(|b| b["type"] == "tool_result"));
        results_only
            .then(|| last["content"].as_array_mut())
            .flatten()
    });
    match open {
        Some(blocks) => blocks.push(block),
        None => out.push(json!({"role": "user", "content": [block]})),
    }
}
