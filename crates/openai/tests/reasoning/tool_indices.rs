use agent_harness::{Message, Session};
use agent_harness_http::Wire;
use agent_harness_openai::OpenAi;
use serde_json::{Value, json};

#[test]
fn tool_indices_must_match_the_preserved_call_order() {
    let body = json!({"choices": [{
        "finish_reason": "tool_calls",
        "message": {
            "role": "assistant",
            "content": null,
            "tool_calls": [
                {"id": "first", "index": 0, "type": "function", "function": {"name": "read_input", "arguments": "{}"}},
                {"id": "second", "index": 1, "type": "function", "function": {"name": "write_output", "arguments": "{\"content\":\"result\"}"}}
            ]
        }
    }]});
    let reply = OpenAi.reply(&body).unwrap();
    let saved = Session {
        revision: 1,
        messages: vec![Message::User("work".into()), reply.message],
        next_turn: 0,
        completed: false,
        unresolved_effect: false,
    };
    let restored: Session = serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
    let request = OpenAi.request("model", &restored.messages, &[]).unwrap();
    let calls = request["messages"][1]["tool_calls"].as_array().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["id"], "first");
    assert_eq!(calls[0]["function"]["name"], "read_input");
    assert_eq!(calls[0]["function"]["arguments"], "{}");
    assert_eq!(calls[1]["id"], "second");
    assert_eq!(calls[1]["function"]["name"], "write_output");
    assert_eq!(
        calls[1]["function"]["arguments"],
        "{\"content\":\"result\"}"
    );
    for (position, index) in [
        (0, json!(1)),
        (0, json!(-1)),
        (0, json!(0.5)),
        (0, json!("0")),
        (1, json!(0)),
    ] {
        let mut invalid = body.clone();
        invalid["choices"][0]["message"]["tool_calls"][position]["index"] = index;
        assert!(OpenAi.reply(&invalid).is_err());
    }
    let mut absent = body;
    absent["choices"][0]["message"]["tool_calls"][0]["index"] = Value::Null;
    absent["choices"][0]["message"]["tool_calls"][1]
        .as_object_mut()
        .unwrap()
        .remove("index");
    assert!(OpenAi.reply(&absent).is_ok());
}
