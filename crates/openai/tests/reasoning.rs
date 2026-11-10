use agent_harness::{Message, Session};
use agent_harness_http::Wire;
use agent_harness_openai::OpenAi;
use serde_json::json;

#[path = "reasoning/replay.rs"]
mod replay;

#[path = "reasoning/invalid.rs"]
mod invalid;

#[path = "reasoning/tool_indices.rs"]
mod tool_indices;

const OLLAMA_REPLY: &str = r#"{
  "id": "chat-fixture",
  "object": "chat.completion",
  "created": 0,
  "model": "fixture-model",
  "system_fingerprint": "fp_ollama",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "",
        "reasoning": "先讀來源，再修正連結。\n保留其餘內容。",
        "tool_calls": [
          {
            "id": "read-1",
            "index": 0,
            "type": "function",
            "function": {
              "name": "read_input",
              "arguments": "{}"
            }
          }
        ]
      },
      "finish_reason": "tool_calls"
    }
  ],
  "usage": {
    "prompt_tokens": 359,
    "prompt_tokens_details": {
      "cached_tokens": 355
    },
    "completion_tokens": 80,
    "total_tokens": 439
  }
}"#;

#[test]
fn reasoning_survives_saved_history_with_tool_calls() {
    let body = serde_json::from_str(OLLAMA_REPLY).unwrap();
    let reply = OpenAi
        .reply(&body)
        .expect("真實回應結構中的推理文字必須可保存");
    let session = Session {
        revision: 3,
        messages: vec![
            Message::User("修正連結".into()),
            reply.message,
            Message::Tool {
                call_id: "read-1".into(),
                content: "來源內容".into(),
                is_error: false,
            },
        ],
        next_turn: 1,
        completed: false,
        unresolved_effect: false,
    };
    let stored = serde_json::to_string(&session).unwrap();
    let restored: Session = serde_json::from_str(&stored).unwrap();
    assert_eq!(restored, session);
    let assistant = serde_json::to_value(&restored.messages[1]).unwrap();
    assert_eq!(
        assistant["Assistant"]["reasoning"],
        "先讀來源，再修正連結。\n保留其餘內容。"
    );
    assert_eq!(assistant["Assistant"]["calls"][0]["id"], "read-1");
    assert_eq!(assistant["Assistant"]["calls"][0]["args"], json!({}));
}
