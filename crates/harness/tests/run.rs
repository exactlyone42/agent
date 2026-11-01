use agent_harness::{
    Agent, CancellationToken, Dispatch, DispatchAction, Hook, Message, Model, ModelError,
    ModelReply, RunError, Tool, ToolCall, ToolError, ToolFuture, ToolSpec, TurnStatus,
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
    async fn complete(&self, _: &[Message], _: &[ToolSpec]) -> Result<ModelReply, ModelError> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(ModelError("腳本用完".into())))
            .map(|message| {
                let status = match &message {
                    Message::Assistant { calls, .. } if !calls.is_empty() => TurnStatus::ToolCalls,
                    _ => TurnStatus::Complete,
                };
                ModelReply { message, status }
            })
    }
}

struct StatusScript(Mutex<VecDeque<ModelReply>>);

impl Model for StatusScript {
    async fn complete(&self, _: &[Message], _: &[ToolSpec]) -> Result<ModelReply, ModelError> {
        self.0
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| ModelError("腳本用完".into()))
    }
}

#[tokio::test]
async fn continuation_without_tools_runs_again_and_obeys_turn_limit() {
    let make = || ModelReply {
        message: Message::text("等待"),
        status: TurnStatus::Continue,
    };
    let model = StatusScript(Mutex::new(vec![make(), make()].into()));
    let result = Agent::new(model).max_turns(2).run("x", &off()).await;
    assert!(matches!(result, Err(RunError::TurnLimit)));
}

#[tokio::test]
async fn conflicting_stop_reason_never_dispatches_tool() {
    let (count, tool) = ran();
    let reply = ModelReply {
        message: Message::call("1", "count", json!({})),
        status: TurnStatus::Complete,
    };
    let model = StatusScript(Mutex::new(vec![reply].into()));
    let result = Agent::new(model).tool(tool).run("x", &off()).await;
    assert!(matches!(result, Err(RunError::Model(_))));
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn continuation_with_tool_calls_is_rejected_without_dispatch() {
    let (count, tool) = ran();
    let reply = ModelReply {
        message: Message::call("1", "count", json!({})),
        status: TurnStatus::Continue,
    };
    let model = StatusScript(Mutex::new(vec![reply].into()));
    let result = Agent::new(model).tool(tool).run("x", &off()).await;
    assert!(
        matches!(result, Err(RunError::Model(error)) if error.0 == "模型停止原因與工具呼叫不一致")
    );
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn tool_call_stop_without_calls_is_rejected() {
    let reply = ModelReply {
        message: Message::text("沒有工具呼叫"),
        status: TurnStatus::ToolCalls,
    };
    let model = StatusScript(Mutex::new(vec![reply].into()));
    let result = Agent::new(model).run("x", &off()).await;
    assert!(
        matches!(result, Err(RunError::Model(error)) if error.0 == "模型停止原因與工具呼叫不一致")
    );
}

#[tokio::test]
async fn truncated_turn_has_its_own_stop_reason() {
    let reply = ModelReply {
        message: Message::text("未完成"),
        status: TurnStatus::Truncated,
    };
    let model = StatusScript(Mutex::new(vec![reply].into()));
    let result = Agent::new(model).run("x", &off()).await;
    assert!(matches!(result, Err(RunError::Truncated)));
}

#[tokio::test]
async fn blocked_turn_stops_for_policy() {
    let reply = ModelReply {
        message: Message::text(""),
        status: TurnStatus::Blocked,
    };
    let model = StatusScript(Mutex::new(vec![reply].into()));
    let result = Agent::new(model).run("x", &off()).await;
    assert!(matches!(result, Err(RunError::Blocked)));
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
    assert!(matches!(result, Err(RunError::Cancelled { .. })));
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
async fn duplicate_call_id_rejects_the_batch_before_any_dispatch() {
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
    let result = Agent::new(model).tool(tool).run("x", &off()).await;
    assert!(matches!(result, Err(RunError::Model(_))));
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn reused_call_id_in_later_turn_is_rejected_before_dispatch() {
    let (count, tool) = ran();
    let model = Script::new(vec![
        Ok(Message::call("a", "count", json!({}))),
        Ok(Message::call("a", "count", json!({}))),
        Ok(Message::text("完")),
    ]);
    let result = Agent::new(model).tool(tool).run("x", &off()).await;
    assert!(matches!(result, Err(RunError::Model(_))));
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

struct Slow(Arc<AtomicUsize>);

struct CancelDuring(CancellationToken);

struct CancelWhileAwaitingHook(CancellationToken);

struct CancelAfterOutcome(CancellationToken);

impl Hook for CancelAfterOutcome {
    async fn on_outcome(&self, _: agent_harness::Outcome<'_>) -> agent_harness::OutcomeAction {
        self.0.cancel();
        std::future::pending().await
    }
}

#[tokio::test]
async fn cancellation_while_saving_outcome_preserves_completed_result() {
    let (count, tool) = ran();
    let token = off();
    let calls = vec![
        call_of("a", "count", json!({})),
        call_of("b", "count", json!({})),
    ];
    let model = Script::new(vec![Ok(Message::Assistant {
        text: String::new(),
        calls,
    })]);
    let agent = Agent::new(model)
        .tool(tool)
        .hook(CancelAfterOutcome(token.clone()));
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        agent.run("x", &token),
    )
    .await
    .expect("取消必須喚醒等待中的掛勾");
    let Err(RunError::Cancelled { messages }) = result else {
        panic!("取消未回傳歷史")
    };
    assert!(paired(&messages));
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(
        matches!(&messages[2], Message::Tool { content, is_error: false, .. } if content == "已計數")
    );
    assert!(
        matches!(&messages[3], Message::Tool { content, is_error: true, .. } if content == "未執行")
    );
}

impl Hook for CancelWhileAwaitingHook {
    async fn on_dispatch(&self, _: Dispatch<'_>) -> DispatchAction {
        self.0.cancel();
        std::future::pending().await
    }
}

#[tokio::test]
async fn cancellation_while_waiting_for_hook_settles_as_unstarted() {
    let (count, tool) = ran();
    let token = off();
    let model = Script::new(vec![Ok(Message::call("a", "count", json!({})))]);
    let agent = Agent::new(model)
        .tool(tool)
        .hook(CancelWhileAwaitingHook(token.clone()));
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        agent.run("x", &token),
    )
    .await
    .expect("取消必須喚醒等待中的掛勾");
    let Err(RunError::Cancelled { messages }) = result else {
        panic!("取消未回傳歷史")
    };
    assert!(paired(&messages));
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert!(
        matches!(&messages[2], Message::Tool { content, is_error: true, .. } if content == "未執行")
    );
}

impl Tool for CancelDuring {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "cancel_during".into(),
            description: "測試取消".into(),
            parameters: json!({}),
        }
    }

    fn call(&self, _: Value) -> ToolFuture<'_> {
        Box::pin(async move {
            self.0.cancel();
            std::future::pending().await
        })
    }
}

#[tokio::test]
async fn cancellation_settles_completed_inflight_and_unstarted_calls() {
    let (count, tool) = ran();
    let cancel = off();
    let calls = vec![
        call_of("a", "count", json!({})),
        call_of("b", "cancel_during", json!({})),
        call_of("c", "count", json!({})),
    ];
    let model = Script::new(vec![Ok(Message::Assistant {
        text: String::new(),
        calls,
    })]);
    let result = Agent::new(model)
        .tool(tool)
        .tool(CancelDuring(cancel.clone()))
        .run("x", &cancel)
        .await;
    let Err(RunError::Cancelled { messages }) = result else {
        panic!("取消未回傳歷史")
    };
    assert!(paired(&messages));
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(
        matches!(&messages[2], Message::Tool { call_id, content, is_error: false } if call_id == "a" && content == "已計數")
    );
    assert!(
        matches!(&messages[3], Message::Tool { call_id, content, is_error: true } if call_id == "b" && content == "結果未知")
    );
    assert!(
        matches!(&messages[4], Message::Tool { call_id, content, is_error: true } if call_id == "c" && content == "未執行")
    );
}

impl Tool for Slow {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "slow".into(),
            description: "延遲後寫入".into(),
            parameters: json!({}),
        }
    }

    fn call(&self, _: Value) -> ToolFuture<'_> {
        Box::pin(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok("已寫入".into())
        })
    }
}

#[tokio::test]
async fn cancel_during_a_slow_tool_stops_it_before_it_writes() {
    let writes = Arc::new(AtomicUsize::new(0));
    let model = Script::new(vec![Ok(Message::call("1", "slow", json!({})))]);
    let cancel = off();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        trigger.cancel();
    });
    let result = Agent::new(model)
        .tool(Slow(writes.clone()))
        .run("x", &cancel)
        .await;
    assert!(matches!(result, Err(RunError::Cancelled { .. })));
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    assert_eq!(writes.load(Ordering::SeqCst), 0);
}
