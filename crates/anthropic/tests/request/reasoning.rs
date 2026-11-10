use agent_harness::{Message, Model, ToolCall};
use agent_harness_anthropic::Anthropic;
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpListener;
use url::Url;

#[tokio::test]
async fn incompatible_reasoning_history_is_rejected_before_http() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = Url::parse(&format!("http://{}/v1", listener.local_addr().unwrap())).unwrap();
    let model = Anthropic::new(&base, "model", "", Duration::from_secs(1)).unwrap();
    for (reasoning, calls) in [
        (Some("必須保留的原始推理文字".into()), vec![]),
        (
            None,
            vec![ToolCall {
                id: "bad-json".into(),
                name: "read_input".into(),
                args: json!({}),
                bad_args: Some("{broken".into()),
            }],
        ),
        (
            None,
            vec![ToolCall {
                id: "bad-object".into(),
                name: "read_input".into(),
                args: json!(42),
                bad_args: None,
            }],
        ),
    ] {
        let history = [Message::Assistant {
            text: String::new(),
            calls,
            reasoning,
        }];
        tokio::select! {
            result = model.complete(&history, &[]) => {
                let error = result.expect_err("不支援的歷史不得被靜默改寫");
                assert_eq!(error.0, "這個模型入口無法接續已保存的內容");
            }
            incoming = listener.accept() => panic!("拒絕前不得建立HTTP連線：{incoming:?}"),
        }
    }
}
