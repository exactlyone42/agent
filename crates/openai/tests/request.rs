use agent_harness::{Agent, CancellationToken, Message, Model, RunError, ToolCall};
use agent_harness_openai::OpenAi;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};
use url::Url;

const REPLY: &str = r#"{"choices":[{"finish_reason":"stop","message":{"content":"ok"}}]}"#;

#[tokio::test]
async fn truncated_reply_does_not_finish_successfully() {
    let reply = r#"{"choices":[{"finish_reason":"length","message":{"content":"incomplete"}}]}"#;
    let (url, handle) = serve_reply(reply).await;
    let model = OpenAi::new(&url, "m", "", Duration::from_secs(5)).unwrap();
    let result = Agent::new(model).run("hi", &CancellationToken::new()).await;
    handle.await.unwrap();
    assert!(matches!(result, Err(RunError::Truncated)));
}

#[tokio::test]
async fn missing_or_unknown_finish_reason_is_not_success() {
    for reply in [
        r#"{"choices":[{"message":{"content":"ok"}}]}"#,
        r#"{"choices":[{"finish_reason":"unexpected","message":{"content":"ok"}}]}"#,
    ] {
        let (url, handle) = serve_reply(reply).await;
        let model = OpenAi::new(&url, "m", "", Duration::from_secs(5)).unwrap();
        let result = Agent::new(model).run("hi", &CancellationToken::new()).await;
        handle.await.unwrap();
        assert!(matches!(result, Err(RunError::Model(_))));
    }
}

#[tokio::test]
async fn provider_content_filter_stops_the_run() {
    let reply = r#"{"choices":[{"finish_reason":"content_filter","message":{"content":""}}]}"#;
    let (url, handle) = serve_reply(reply).await;
    let model = OpenAi::new(&url, "m", "", Duration::from_secs(5)).unwrap();
    let result = Agent::new(model).run("hi", &CancellationToken::new()).await;
    handle.await.unwrap();
    assert!(matches!(result, Err(RunError::Blocked)));
}

async fn serve() -> (Url, JoinHandle<String>) {
    serve_reply(REPLY).await
}

async fn serve_reply(reply: &'static str) -> (Url, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = Url::parse(&format!("http://{}/v1", listener.local_addr().unwrap())).unwrap();
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
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

#[tokio::test]
async fn plain_assistant_message_has_no_tool_calls_field() {
    let (url, handle) = serve().await;
    let model = OpenAi::new(&url, "m", "", Duration::from_secs(5)).unwrap();
    let history = [Message::Assistant {
        text: "hello".into(),
        calls: vec![],
    }];
    model.complete(&history, &[]).await.unwrap();
    let body = body_of(&handle.await.unwrap());
    assert_eq!(
        body["messages"][0],
        json!({"role": "assistant", "content": "hello"})
    );
}

#[tokio::test]
async fn assistant_message_with_calls_keeps_them() {
    let (url, handle) = serve().await;
    let model = OpenAi::new(&url, "m", "", Duration::from_secs(5)).unwrap();
    let history = [Message::Assistant {
        text: String::new(),
        calls: vec![ToolCall {
            id: "c1".into(),
            name: "read".into(),
            args: json!({"path": "a"}),
            bad_args: None,
        }],
    }];
    model.complete(&history, &[]).await.unwrap();
    let body = body_of(&handle.await.unwrap());
    assert_eq!(body["messages"][0]["tool_calls"][0]["id"], "c1");
    assert_eq!(body["messages"][0]["content"], Value::Null);
}

#[tokio::test]
async fn bearer_header_is_sent_only_with_a_key() {
    let (url, handle) = serve().await;
    let model = OpenAi::new(&url, "m", "secret", Duration::from_secs(5)).unwrap();
    model
        .complete(&[Message::User("hi".into())], &[])
        .await
        .unwrap();
    let with_key = handle.await.unwrap().to_ascii_lowercase();
    assert!(with_key.contains("authorization: bearer secret"));

    let (url, handle) = serve().await;
    let model = OpenAi::new(&url, "m", "", Duration::from_secs(5)).unwrap();
    model
        .complete(&[Message::User("hi".into())], &[])
        .await
        .unwrap();
    let without_key = handle.await.unwrap().to_ascii_lowercase();
    assert!(!without_key.contains("authorization"));
}

#[tokio::test]
async fn invalid_tool_arguments_are_kept_for_the_harness_to_report() {
    let reply = r#"{"choices":[{"finish_reason":"tool_calls","message":{"content":"","tool_calls":[{"id":"c1","function":{"name":"read","arguments":"{oops"}}]}}]}"#;
    let (url, _handle) = serve_reply(reply).await;
    let model = OpenAi::new(&url, "m", "", Duration::from_secs(5)).unwrap();
    let message = model
        .complete(&[Message::User("hi".into())], &[])
        .await
        .unwrap();
    let Message::Assistant { calls, .. } = message.message else {
        panic!("not assistant");
    };
    assert_eq!(calls[0].bad_args.as_deref(), Some("{oops"));
}
