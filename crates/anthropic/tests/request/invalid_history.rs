use agent_harness::{Message, ToolCall};
use agent_harness_anthropic::Anthropic;
use agent_harness_http::Wire;
use serde_json::Value;

#[test]
fn malformed_tool_history_is_rejected_without_rewriting_it() {
    let history = [Message::Assistant {
        reasoning: None,
        text: String::new(),
        calls: vec![ToolCall {
            id: "c1".into(),
            name: "read".into(),
            args: Value::Null,
            bad_args: Some("{oops".into()),
        }],
    }];
    let original = serde_json::to_value(&history).unwrap();
    let error = Anthropic { max_tokens: 4096 }
        .request("model", &history, &[])
        .expect_err("不可把保存的錯誤參數改成空物件");
    assert_eq!(error.0, "這個模型入口無法接續已保存的內容");
    assert_eq!(serde_json::to_value(&history).unwrap(), original);
}
