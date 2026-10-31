use agent_harness::{
    Agent, CancellationToken, Dispatch, DispatchAction, Hook, Message, Model, ModelError, RunError,
    Tool, ToolCall, ToolError, ToolFuture, ToolSpec,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Clone)]
struct Script {
    replies: Arc<Mutex<VecDeque<Result<Message, ModelError>>>>,
    asked: Arc<AtomicUsize>,
}

impl Script {
    fn new(replies: Vec<Result<Message, ModelError>>) -> Self {
        Self {
            replies: Arc::new(Mutex::new(replies.into())),
            asked: Arc::default(),
        }
    }
}

impl Model for Script {
    async fn complete(&self, _: &[Message], _: &[ToolSpec]) -> Result<Message, ModelError> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(ModelError("腳本用完".into())))
    }
}

struct Counter(Arc<AtomicUsize>);

impl Tool for Counter {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "count".into(),
            description: "計數".into(),
            parameters: json!({}),
        }
    }

    fn call(&self, args: Value) -> ToolFuture<'_> {
        Box::pin(async move {
            if args["fail"] == true {
                return Err(ToolError::refused("故意失敗"));
            }
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok("已計數".into())
        })
    }
}

struct Verdict(fn() -> DispatchAction);

impl Hook for Verdict {
    async fn on_dispatch(&self, _: Dispatch<'_>) -> DispatchAction {
        (self.0)()
    }
}

fn ran() -> (Arc<AtomicUsize>, Counter) {
    let counter = Arc::new(AtomicUsize::new(0));
    (counter.clone(), Counter(counter))
}

fn off() -> CancellationToken {
    CancellationToken::new()
}

#[tokio::test]
async fn plain_answer_ends_in_one_model_call() {
    let model = Script::new(vec![Ok(Message::text("好"))]);
    let out = Agent::new(model.clone()).run("hi", &off()).await.unwrap();
    assert_eq!(out.text, "好");
    assert_eq!(model.asked.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn tool_runs_and_result_goes_back_to_model() {
    let (count, tool) = ran();
    let model = Script::new(vec![
        Ok(Message::call("1", "count", json!({}))),
        Ok(Message::text("完")),
    ]);
    let out = Agent::new(model).tool(tool).run("x", &off()).await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(
        out.messages[2],
        Message::Tool {
            call_id: "1".into(),
            content: "已計數".into(),
            is_error: false
        }
    );
}

#[tokio::test]
async fn skip_hides_tool_and_tells_model() {
    let (count, tool) = ran();
    let model = Script::new(vec![
        Ok(Message::call("1", "count", json!({}))),
        Ok(Message::text("完")),
    ]);
    let hook = Verdict(|| DispatchAction::Skip("不准".into()));
    let out = Agent::new(model)
        .hook(hook)
        .tool(tool)
        .run("x", &off())
        .await
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(
        out.messages[2],
        Message::Tool {
            call_id: "1".into(),
            content: "不准".into(),
            is_error: true
        }
    );
}

#[tokio::test]
async fn stop_ends_run_without_running_tool() {
    let (count, tool) = ran();
    let model = Script::new(vec![Ok(Message::call("1", "count", json!({})))]);
    let hook = Verdict(|| DispatchAction::Stop("攔下".into()));
    let result = Agent::new(model)
        .hook(hook)
        .tool(tool)
        .run("x", &off())
        .await;
    assert!(matches!(result, Err(RunError::Stopped(why)) if why == "攔下"));
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unknown_tool_is_reported_and_hook_is_not_asked() {
    let model = Script::new(vec![
        Ok(Message::call("1", "nope", json!({}))),
        Ok(Message::text("完")),
    ]);
    let hook = Verdict(|| panic!("不該詢問"));
    let out = Agent::new(model).hook(hook).run("x", &off()).await.unwrap();
    assert!(matches!(
        &out.messages[2],
        Message::Tool { is_error: true, .. }
    ));
}

#[tokio::test]
async fn tool_failure_is_reported_to_model() {
    let (_, tool) = ran();
    let model = Script::new(vec![
        Ok(Message::call("1", "count", json!({"fail": true}))),
        Ok(Message::text("完")),
    ]);
    let out = Agent::new(model).tool(tool).run("x", &off()).await.unwrap();
    assert_eq!(
        out.messages[2],
        Message::Tool {
            call_id: "1".into(),
            content: "故意失敗".into(),
            is_error: true
        }
    );
}

#[tokio::test]
async fn cancelled_run_never_asks_model() {
    let model = Script::new(vec![Ok(Message::text("好"))]);
    let cancel = off();
    cancel.cancel();
    let result = Agent::new(model.clone()).run("x", &cancel).await;
    assert!(matches!(result, Err(RunError::Cancelled)));
    assert_eq!(model.asked.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn endless_tool_calls_hit_turn_limit() {
    let (_, tool) = ran();
    let replies = (0..5)
        .map(|i| Ok(Message::call(&i.to_string(), "count", json!({}))))
        .collect();
    let model = Script::new(replies);
    let result = Agent::new(model.clone())
        .tool(tool)
        .max_turns(3)
        .run("x", &off())
        .await;
    assert!(matches!(result, Err(RunError::TurnLimit)));
    assert_eq!(model.asked.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn model_error_is_returned() {
    let model = Script::new(vec![Err(ModelError("斷線".into()))]);
    let result = Agent::new(model).run("x", &off()).await;
    assert!(matches!(result, Err(RunError::Model(e)) if e.0 == "斷線"));
}

#[tokio::test]
async fn bad_arguments_go_back_to_model_without_running_tool() {
    let (count, tool) = ran();
    let bad = ToolCall {
        id: "1".into(),
        name: "count".into(),
        args: Value::Null,
        bad_args: Some("{oops".into()),
    };
    let model = Script::new(vec![
        Ok(Message::Assistant {
            text: String::new(),
            calls: vec![bad],
        }),
        Ok(Message::text("改好了")),
    ]);
    let out = Agent::new(model).tool(tool).run("x", &off()).await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(out.text, "改好了");
    assert_eq!(
        out.messages[2],
        Message::Tool {
            call_id: "1".into(),
            content: "工具參數不是有效 JSON".into(),
            is_error: true
        }
    );
}

fn call_of(id: &str, name: &str, args: Value) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: name.into(),
        args,
        bad_args: None,
    }
}

fn paired(messages: &[Message]) -> bool {
    let mut asked: Vec<String> = Vec::new();
    let mut answered: Vec<String> = Vec::new();
    for message in messages {
        match message {
            Message::Assistant { calls, .. } => asked.extend(calls.iter().map(|c| c.id.clone())),
            Message::Tool { call_id, .. } => answered.push(call_id.clone()),
            Message::User(_) => {}
        }
    }
    asked == answered
}

#[tokio::test]
async fn every_call_gets_exactly_one_result_in_order() {
    let (count, tool) = ran();
    let calls = vec![
        call_of("a", "count", json!({})),
        call_of("b", "nope", json!({})),
        call_of("c", "count", json!({"fail": true})),
        ToolCall {
            bad_args: Some("{".into()),
            ..call_of("d", "count", Value::Null)
        },
    ];
    let model = Script::new(vec![
        Ok(Message::Assistant {
            text: String::new(),
            calls,
        }),
        Ok(Message::text("完")),
    ]);
    let out = Agent::new(model).tool(tool).run("x", &off()).await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(paired(&out.messages));
}

#[tokio::test]
async fn duplicate_call_id_is_refused_and_not_run() {
    let (count, tool) = ran();
    let calls = vec![
        call_of("a", "count", json!({})),
        call_of("a", "count", json!({})),
    ];
    let model = Script::new(vec![
        Ok(Message::Assistant {
            text: String::new(),
            calls,
        }),
        Ok(Message::text("完")),
    ]);
    let out = Agent::new(model).tool(tool).run("x", &off()).await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(matches!(
        &out.messages[3],
        Message::Tool { is_error: true, .. }
    ));
}
