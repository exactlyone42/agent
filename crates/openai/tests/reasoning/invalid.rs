use agent_harness_http::Wire;
use agent_harness_openai::OpenAi;
use serde_json::json;

#[test]
fn invalid_reasoning_is_rejected_before_accepting_tool_calls() {
    for reasoning in [json!(42), json!(true), json!([]), json!({"text": "bad"})] {
        let response = json!({"choices": [{
            "finish_reason": "tool_calls",
            "message": {
                "role": "assistant",
                "content": null,
                "reasoning": reasoning,
                "tool_calls": [{
                    "id": "read-1",
                    "type": "function",
                    "function": {"name": "read_input", "arguments": "{}"}
                }]
            }
        }]});
        let error = OpenAi.reply(&response).expect_err("錯型別不可進入工具執行");
        assert_eq!(error.0, "模型回應含有無法保存或續接的內容");
    }
}
