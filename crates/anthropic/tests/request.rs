#[path = "request/invalid_history.rs"]
mod invalid_history;

use agent_harness::{Agent, CancellationToken, Message, Model, RunError, ToolCall, ToolSpec};
use agent_harness_anthropic::Anthropic;
use agent_harness_http::{HttpModel, Wire};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};
use url::Url;

const REPLY: &str = r#"{"stop_reason":"end_turn","content":[{"type":"text","text":"ok"}]}"#;

fn model(url: &Url) -> HttpModel<Anthropic> {
    Anthropic::new(url, "m", "secret", Duration::from_secs(5)).unwrap()
}

async fn serve_reply(reply: &'static str) -> (Url, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = Url::parse(&format!("http://{}/v1", listener.local_addr().unwrap())).unwrap();
    let handle = tokio::spawn(async move {
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
            .await
            .expect("模型應在五秒內送出 HTTP 請求")
            .unwrap();
        let mut raw = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let read = stream.read(&mut chunk).await.unwrap();
            raw.extend_from_slice(&chunk[..read]);
            let text = String::from_utf8_lossy(&raw).to_string();
            let Some((head, body)) = text.split_once("\r\n\r\n") else {
                continue;
            };
            let length = head
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .map(str::to_owned)
                })
                .map(|value| value.trim().parse::<usize>().unwrap())
                .unwrap_or(0);
            if body.len() >= length {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
            reply.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8(raw).unwrap()
    });
    (url, handle)
}

fn body_of(raw: &str) -> Value {
    serde_json::from_str(raw.split_once("\r\n\r\n").unwrap().1).unwrap()
}

fn call(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "read".into(),
        args: json!({"path": "a"}),
        bad_args: None,
    }
}

#[tokio::test]
async fn request_carries_path_headers_and_required_fields() {
    let (url, handle) = serve_reply(REPLY).await;
    model(&url)
        .complete(&[Message::User("hi".into())], &[])
        .await
        .unwrap();
    let raw = handle.await.unwrap();
    let head = raw.to_ascii_lowercase();
    assert!(head.starts_with("post /v1/messages "));
    assert!(head.contains("x-api-key: secret"));
    assert!(head.contains("anthropic-version: 2023-06-01"));
    assert!(!head.contains("authorization"));
    let body = body_of(&raw);
    assert_eq!(body["max_tokens"], 4096);
    assert_eq!(
        body["messages"][0],
        json!({"role": "user", "content": "hi"})
    );
}

#[tokio::test]
async fn tools_use_input_schema_field() {
    let (url, handle) = serve_reply(REPLY).await;
    let spec = ToolSpec {
        name: "read".into(),
        description: "d".into(),
        parameters: json!({"type": "object"}),
    };
    model(&url)
        .complete(&[Message::User("hi".into())], &[spec])
        .await
        .unwrap();
    let body = body_of(&handle.await.unwrap());
    assert_eq!(
        body["tools"][0],
        json!({"name": "read", "description": "d", "input_schema": {"type": "object"}})
    );
}

#[tokio::test]
async fn parallel_tool_results_merge_into_one_user_message() {
    let (url, handle) = serve_reply(REPLY).await;
    let history = [
        Message::User("go".into()),
        Message::Assistant {
            reasoning: None,
            text: String::new(),
            calls: vec![call("c1"), call("c2")],
        },
        Message::Tool {
            call_id: "c1".into(),
            content: "r1".into(),
            is_error: false,
        },
        Message::Tool {
            call_id: "c2".into(),
            content: "r2".into(),
            is_error: true,
        },
    ];
    model(&url).complete(&history, &[]).await.unwrap();
    let body = body_of(&handle.await.unwrap());
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1]["content"][1]["type"], "tool_use");
    assert_eq!(messages[1]["content"][1]["id"], "c2");
    let results = messages[2]["content"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["tool_use_id"], "c1");
    assert_eq!(results[1]["is_error"], true);
}

#[tokio::test]
async fn tool_use_reply_becomes_tool_calls() {
    let reply = r#"{"stop_reason":"tool_use","content":[{"type":"text","text":"x"},{"type":"tool_use","id":"t1","name":"read","input":{"path":"a"}}]}"#;
    let (url, _handle) = serve_reply(reply).await;
    let reply = model(&url)
        .complete(&[Message::User("hi".into())], &[])
        .await
        .unwrap();
    let Message::Assistant { text, calls, .. } = reply.message else {
        panic!("not assistant");
    };
    assert_eq!(text, "x");
    assert_eq!(calls[0].id, "t1");
    assert_eq!(calls[0].args, json!({"path": "a"}));
}

#[tokio::test]
async fn max_tokens_does_not_finish_successfully() {
    let reply = r#"{"stop_reason":"max_tokens","content":[{"type":"text","text":"cut"}]}"#;
    let (url, handle) = serve_reply(reply).await;
    let result = Agent::new(model(&url))
        .run("hi", &CancellationToken::new())
        .await;
    handle.await.unwrap();
    assert!(matches!(result, Err(RunError::Truncated)));
}

#[tokio::test]
async fn refusal_stops_the_run() {
    let reply = r#"{"stop_reason":"refusal","content":[]}"#;
    let (url, handle) = serve_reply(reply).await;
    let result = Agent::new(model(&url))
        .run("hi", &CancellationToken::new())
        .await;
    handle.await.unwrap();
    assert!(matches!(result, Err(RunError::Blocked)));
}

#[tokio::test]
async fn missing_or_unknown_stop_reason_is_not_success() {
    for reply in [
        r#"{"content":[{"type":"text","text":"ok"}]}"#,
        r#"{"stop_reason":"stop_sequence","content":[{"type":"text","text":"ok"}]}"#,
    ] {
        let (url, handle) = serve_reply(reply).await;
        let result = Agent::new(model(&url))
            .run("hi", &CancellationToken::new())
            .await;
        handle.await.unwrap();
        assert!(matches!(result, Err(RunError::Model(_))));
    }
}

#[test]
fn tool_result_is_not_merged_into_an_empty_assistant_message() {
    let body = Anthropic { max_tokens: 4096 }
        .request(
            "m",
            &[
                Message::text(""),
                Message::Tool {
                    call_id: "c1".into(),
                    content: "r1".into(),
                    is_error: false,
                },
            ],
            &[],
        )
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0], json!({"role": "assistant", "content": []}));
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"][0]["type"], "tool_result");
    assert_eq!(messages[1]["content"][0]["tool_use_id"], "c1");
}

#[test]
fn pause_turn_preserves_text_and_requests_continuation() {
    let reply = Anthropic { max_tokens: 4096 }
        .reply(&json!({
            "stop_reason": "pause_turn", "content": [{"type": "text", "text": "partial"}],
        }))
        .unwrap();
    assert_eq!(reply.status, agent_harness::TurnStatus::Continue);
    assert_eq!(reply.message, Message::text("partial"));
}
#[path = "request/reasoning.rs"]
mod reasoning;

#[path = "request/unsupported_reply.rs"]
mod unsupported_reply;

#[path = "request/invalid_blocks.rs"]
mod invalid_blocks;
