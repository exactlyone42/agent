use agent_harness::{Message, Session};
use agent_harness_http::Wire;
use agent_harness_openai::OpenAi;
use serde_json::{Value, json};

#[test]
fn saved_reasoning_is_replayed_without_changing_legacy_messages() {
    let legacy = json!({
        "revision": 1,
        "messages": [{"Assistant": {"text": "old", "calls": []}}],
        "next_turn": 0,
        "completed": false,
        "unresolved_effect": false
    });
    let old: Session = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(serde_json::to_value(&old).unwrap(), legacy);
    let request = OpenAi.request("model", &old.messages, &[]).unwrap();
    assert_eq!(
        request["messages"][0],
        json!({"role": "assistant", "content": "old"})
    );
    for (content, reasoning, calls) in [
        (json!("done"), json!("先檢查。\n再交付。"), json!([])),
        (json!("done"), json!(""), json!([])),
        (json!("done"), Value::Null, json!([])),
        (
            Value::Null,
            json!("先讀取指定檔案。"),
            json!([{
                "id": "read-1",
                "type": "function",
                "function": {"name": "read_input", "arguments": "{}"}
            }]),
        ),
    ] {
        let message = json!({
            "role": "assistant",
            "content": content,
            "reasoning": reasoning,
            "tool_calls": calls
        });
        let finish = if calls.as_array().unwrap().is_empty() {
            "stop"
        } else {
            "tool_calls"
        };
        let reply = OpenAi
            .reply(&json!({"choices": [{"finish_reason": finish, "message": message}]}))
            .unwrap();
        let encoded = serde_json::to_string(&reply.message).unwrap();
        let restored: Message = serde_json::from_str(&encoded).unwrap();
        let request = OpenAi.request("model", &[restored], &[]).unwrap();
        let replayed = &request["messages"][0];
        assert_eq!(replayed["reasoning"], reasoning);
        if reasoning.is_null() {
            assert!(replayed.get("reasoning").is_none());
        }
        if !calls.as_array().unwrap().is_empty() {
            assert_eq!(replayed["tool_calls"], calls);
            assert_eq!(replayed["content"], Value::Null);
        }
    }
}
