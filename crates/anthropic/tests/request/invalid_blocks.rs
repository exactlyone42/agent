use agent_harness_anthropic::Anthropic;
use agent_harness_http::Wire;
use serde_json::json;

#[test]
fn invalid_or_unpreserved_block_fields_reject_the_entire_reply() {
    let wire = Anthropic { max_tokens: 32 };
    let mut observations = Vec::new();
    for block in [
        json!({"type": "text", "text": 42}),
        json!({"type": "text"}),
        json!({"type": "text", "text": "answer", "citations": [{"url": "source"}]}),
        json!({"type": "tool_use", "id": "read-2", "name": "read_input", "input": {}, "signature": "opaque"}),
        json!({"type": "tool_use", "id": "read-2", "name": "read_input", "input": 42}),
        json!({"type": "tool_use", "id": "read-2", "name": "read_input"}),
    ] {
        let response = json!({
            "stop_reason": "tool_use",
            "content": [
                {"type": "tool_use", "id": "read-1", "name": "read_input", "input": {}},
                block
            ]
        });
        observations.push((block, wire.reply(&response).is_err()));
    }
    assert!(
        observations.iter().all(|(_, rejected)| *rejected),
        "合法工具同批存在時，也不得丟掉其他區塊內容：{observations:?}"
    );
}
