use agent_harness::{Message, TurnStatus};
use agent_harness_http::Wire;
use agent_harness_openai::OpenAi;
use serde_json::{Value, json};

fn reply(message: Value) -> Value {
    json!({"choices": [{"finish_reason": "tool_calls", "message": message}]})
}

fn message() -> Value {
    json!({"role": "assistant", "content": null, "tool_calls": [{
        "id": "c1", "type": "function", "function": {"name": "read", "arguments": "{}"}
    }]})
}

#[test]
fn unsupported_message_content_is_rejected_before_accepting_calls() {
    for (key, value) in [
        ("reasoning_content", json!("private reasoning")),
        ("signature", json!("private signature")),
        ("refusal", json!("refused")),
        ("audio", json!({"id": "audio1"})),
        ("annotations", json!([{"url": "private source"}])),
        ("content", json!([{"type": "text", "text": "hello"}])),
        ("content", json!(42)),
        ("tool_calls", json!({"id": "c1"})),
        ("role", json!("user")),
    ] {
        let mut raw = message();
        raw[key] = value;
        let error = OpenAi.reply(&reply(raw)).expect_err(key);
        assert_eq!(error.0, "模型回應含有無法保存或續接的內容");
    }
}

#[test]
fn unsupported_call_data_is_rejected() {
    for (pointer, value) in [
        ("/tool_calls/0/signature", json!("private signature")),
        ("/tool_calls/0/type", json!("custom")),
        (
            "/tool_calls/0/function/signature",
            json!("private signature"),
        ),
    ] {
        let mut raw = message();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        raw.pointer_mut(parent).unwrap()[key] = value;
        assert!(OpenAi.reply(&reply(raw)).is_err(), "{pointer}");
    }
}

#[test]
fn supported_text_calls_and_empty_standard_fields_are_preserved() {
    let mut raw = message();
    raw["content"] = json!("hello");
    raw["refusal"] = Value::Null;
    raw["audio"] = Value::Null;
    raw["annotations"] = json!([]);
    let result = OpenAi.reply(&reply(raw)).unwrap();
    assert_eq!(result.status, TurnStatus::ToolCalls);
    let Message::Assistant { text, calls, .. } = result.message else {
        panic!("not assistant");
    };
    assert_eq!(text, "hello");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "c1");
    assert_eq!(calls[0].name, "read");
    assert_eq!(calls[0].args, json!({}));
}
