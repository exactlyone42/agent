use agent_harness_anthropic::Anthropic;
use agent_harness_http::Wire;
use serde_json::json;

#[test]
fn unsupported_reply_blocks_are_not_silently_dropped() {
    let wire = Anthropic { max_tokens: 32 };
    for block in [
        json!({"type": "thinking", "thinking": "private", "signature": "opaque"}),
        json!({"type": "redacted_thinking", "data": "opaque"}),
        json!({"type": "unknown", "data": "private"}),
    ] {
        let response = json!({
            "stop_reason": "tool_use",
            "content": [
                {"type": "tool_use", "id": "read-1", "name": "read_input", "input": {}},
                block
            ]
        });
        let error = wire
            .reply(&response)
            .expect_err("無法保存的內容必須明確拒絕");
        assert_eq!(error.0, "模型回應含有無法保存或續接的內容");
    }
}
