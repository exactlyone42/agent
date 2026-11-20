use agent_harness::{
    AgentId, AgentIdentity, CancellationToken, Capability, CapabilityContext, Dispatch,
    DispatchAction, Event, Hook, Intent, Message, Model, ModelError, MortalAgent, RunError,
    RunOutput, StopReason, Tool, ToolCall, ToolError, ToolFuture, ToolSpec,
};
use agent_harness_capabilities::{Loop, ToolCalling};
use agent_platform_agent::agent_runtime::AgentRuntime;
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

struct Counter {
    runs: Arc<AtomicUsize>,
    cancel_on_call: Option<CancellationToken>,
}

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
            self.runs.fetch_add(1, Ordering::SeqCst);
            if let Some(cancel) = &self.cancel_on_call {
                cancel.cancel();
            }
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

struct Probe {
    name: &'static str,
    fired: bool,
    propose: fn(&Event, &CapabilityContext) -> Vec<Intent>,
    once: bool,
}

impl Capability for Probe {
    fn id(&self) -> &str {
        self.name
    }

    fn on_event(&mut self, event: &Event, context: &CapabilityContext) -> Vec<Intent> {
        if self.once && self.fired {
            return Vec::new();
        }
        let intents = (self.propose)(event, context);
        self.fired |= !intents.is_empty();
        intents
    }
}

fn named(name: &'static str, propose: fn(&Event, &CapabilityContext) -> Vec<Intent>) -> Probe {
    Probe {
        name,
        fired: false,
        propose,
        once: true,
    }
}

fn probe(propose: fn(&Event, &CapabilityContext) -> Vec<Intent>) -> Probe {
    Probe {
        name: "probe",
        fired: false,
        propose,
        once: true,
    }
}

fn verify_call() -> ToolCall {
    ToolCall {
        id: "v1".into(),
        name: "count".into(),
        args: json!({}),
    }
}

fn after_plain_reply_verify(event: &Event, _: &CapabilityContext) -> Vec<Intent> {
    match event {
        Event::ModelReplied(Message::Assistant { calls, .. }) if calls.is_empty() => {
            vec![Intent::CallTool(verify_call())]
        }
        _ => Vec::new(),
    }
}

fn after_report_verify(event: &Event, _: &CapabilityContext) -> Vec<Intent> {
    match event {
        Event::ToolReported { .. } => vec![Intent::CallTool(verify_call())],
        _ => Vec::new(),
    }
}

fn stop_after_reply(event: &Event, _: &CapabilityContext) -> Vec<Intent> {
    match event {
        Event::ModelReplied(_) => vec![Intent::Stop(StopReason::LimitReached)],
        _ => Vec::new(),
    }
}

fn ask_after_report(event: &Event, _: &CapabilityContext) -> Vec<Intent> {
    match event {
        Event::ToolReported { .. } => vec![Intent::AskModel],
        _ => Vec::new(),
    }
}

fn always_ask(event: &Event, _: &CapabilityContext) -> Vec<Intent> {
    match event {
        Event::ModelReplied(_) => vec![Intent::AskModel],
        _ => Vec::new(),
    }
}

fn agent() -> MortalAgent {
    MortalAgent::new(
        AgentId("a1".into()),
        AgentIdentity {
            name: "worker".into(),
        },
    )
}

fn counter() -> (Arc<AtomicUsize>, Counter) {
    let runs = Arc::new(AtomicUsize::new(0));
    let tool = Counter {
        runs: runs.clone(),
        cancel_on_call: None,
    };
    (runs, tool)
}

fn proceed() -> Verdict {
    Verdict(|| DispatchAction::Proceed)
}

fn runtime(model: Script, tool: Counter, max_turns: usize) -> AgentRuntime<Script, Verdict> {
    AgentRuntime::new(agent(), model)
        .hook(proceed())
        .tool(tool)
        .capability(Loop::new().max_turns(max_turns))
        .capability(ToolCalling::new())
}

async fn run(mut runtime: AgentRuntime<Script, Verdict>) -> Result<RunOutput, RunError> {
    runtime.run("x", &CancellationToken::new()).await
}

fn tool_message(id: &str, content: &str, is_error: bool) -> Message {
    Message::Tool {
        call_id: id.into(),
        content: content.into(),
        is_error,
    }
}

fn call_reply(id: &str) -> Result<Message, ModelError> {
    Ok(Message::call(id, "count", json!({})))
}

#[tokio::test]
async fn plain_answer_ends_in_one_model_call() {
    let (_, tool) = counter();
    let model = Script::new(vec![Ok(Message::text("好"))]);
    let out = run(runtime(model.clone(), tool, 8)).await.unwrap();
    assert_eq!(out.text, "好");
    assert_eq!(model.asked.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn tool_runs_and_result_goes_back_to_model() {
    let (runs, tool) = counter();
    let model = Script::new(vec![call_reply("1"), Ok(Message::text("完"))]);
    let out = run(runtime(model, tool, 8)).await.unwrap();
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    assert_eq!(out.messages[2], tool_message("1", "已計數", false));
}

#[tokio::test]
async fn skip_hides_tool_and_tells_model() {
    let (runs, tool) = counter();
    let model = Script::new(vec![call_reply("1"), Ok(Message::text("完"))]);
    let hook = Verdict(|| DispatchAction::Skip("不准".into()));
    let rt = AgentRuntime::new(agent(), model)
        .hook(hook)
        .tool(tool)
        .capability(Loop::new())
        .capability(ToolCalling::new());
    let out = run_with(rt).await.unwrap();
    assert_eq!(runs.load(Ordering::SeqCst), 0);
    assert_eq!(out.messages[2], tool_message("1", "不准", true));
}

async fn run_with(mut runtime: AgentRuntime<Script, Verdict>) -> Result<RunOutput, RunError> {
    runtime.run("x", &CancellationToken::new()).await
}

#[tokio::test]
async fn stop_ends_run_without_running_tool() {
    let (runs, tool) = counter();
    let model = Script::new(vec![call_reply("1")]);
    let hook = Verdict(|| DispatchAction::Stop("攔下".into()));
    let rt = AgentRuntime::new(agent(), model)
        .hook(hook)
        .tool(tool)
        .capability(Loop::new())
        .capability(ToolCalling::new());
    let result = run_with(rt).await;
    assert!(matches!(result, Err(RunError::Stopped(why)) if why == "攔下"));
    assert_eq!(runs.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unauthorized_tool_is_refused_reported_and_hook_is_not_asked() {
    let (runs, tool) = counter();
    let model = Script::new(vec![
        Ok(Message::call("1", "delete_everything", json!({}))),
        Ok(Message::text("完")),
    ]);
    let hook = Verdict(|| panic!("不該詢問"));
    let rt = AgentRuntime::new(agent(), model)
        .hook(hook)
        .tool(tool)
        .capability(Loop::new())
        .capability(ToolCalling::new());
    let out = run_with(rt).await.unwrap();
    assert_eq!(runs.load(Ordering::SeqCst), 0);
    assert!(matches!(
        &out.messages[2],
        Message::Tool { is_error: true, .. }
    ));
}

#[tokio::test]
async fn tool_failure_is_reported_to_model() {
    let (_, tool) = counter();
    let model = Script::new(vec![
        Ok(Message::call("1", "count", json!({"fail": true}))),
        Ok(Message::text("完")),
    ]);
    let out = run(runtime(model, tool, 8)).await.unwrap();
    assert_eq!(out.messages[2], tool_message("1", "故意失敗", true));
}

#[tokio::test]
async fn cancelled_run_never_asks_model_or_runs_tools() {
    let (runs, tool) = counter();
    let model = Script::new(vec![call_reply("1")]);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let mut rt = runtime(model.clone(), tool, 8);
    let result = rt.run("x", &cancel).await;
    assert!(matches!(result, Err(RunError::Cancelled)));
    assert_eq!(model.asked.load(Ordering::SeqCst), 0);
    assert_eq!(runs.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cancel_during_a_tool_blocks_the_remaining_calls() {
    let cancel = CancellationToken::new();
    let runs = Arc::new(AtomicUsize::new(0));
    let tool = Counter {
        runs: runs.clone(),
        cancel_on_call: Some(cancel.clone()),
    };
    let two = Message::Assistant {
        text: String::new(),
        calls: vec![
            verify_call(),
            ToolCall {
                id: "v2".into(),
                ..verify_call()
            },
        ],
    };
    let model = Script::new(vec![Ok(two), Ok(Message::text("不該被問"))]);
    let mut rt = runtime(model.clone(), tool, 8);
    let result = rt.run("x", &cancel).await;
    assert!(matches!(result, Err(RunError::Cancelled)));
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    assert_eq!(model.asked.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn endless_tool_calls_hit_the_limit_and_never_count_as_success() {
    let (_, tool) = counter();
    let replies = (0..5).map(|i| call_reply(&i.to_string())).collect();
    let model = Script::new(replies);
    let result = run(runtime(model.clone(), tool, 3)).await;
    assert!(matches!(result, Err(RunError::TurnLimit)));
    assert_eq!(model.asked.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn model_error_is_returned() {
    let (_, tool) = counter();
    let model = Script::new(vec![Err(ModelError("斷線".into()))]);
    let result = run(runtime(model, tool, 8)).await;
    assert!(matches!(result, Err(RunError::Model(e)) if e.0 == "斷線"));
}

fn verifier_runtime(
    model: Script,
    tool: Counter,
    order: [usize; 3],
) -> AgentRuntime<Script, Verdict> {
    let mut rt = AgentRuntime::new(agent(), model).hook(proceed()).tool(tool);
    for slot in order {
        rt = match slot {
            0 => rt.capability(Loop::new()),
            1 => rt.capability(ToolCalling::new()),
            _ => rt.capability(probe(after_plain_reply_verify)),
        };
    }
    rt
}

async fn verifier_summary(order: [usize; 3]) -> (String, usize, usize) {
    let (runs, tool) = counter();
    let model = Script::new(vec![Ok(Message::text("答案")), Ok(Message::text("確認"))]);
    let result = run_with(verifier_runtime(model.clone(), tool, order)).await;
    let out = result.unwrap();
    (
        format!("{:?}", out.messages),
        model.asked.load(Ordering::SeqCst),
        runs.load(Ordering::SeqCst),
    )
}

#[tokio::test]
async fn the_loop_and_a_verifier_proposing_together_are_both_served() {
    let (messages, asked, runs) = verifier_summary([0, 1, 2]).await;
    assert_eq!(asked, 2);
    assert_eq!(runs, 1);
    assert!(messages.ends_with("Assistant { text: \"確認\", calls: [] }]"));
}

#[tokio::test]
async fn registration_order_does_not_change_the_result() {
    let first = verifier_summary([0, 1, 2]).await;
    let orders = [[0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];
    for order in orders {
        assert_eq!(verifier_summary(order).await, first, "order {order:?}");
    }
}

#[tokio::test]
async fn a_loop_stop_does_not_end_the_run_while_other_work_is_pending() {
    let (runs, tool) = counter();
    let model = Script::new(vec![call_reply("c1"), Ok(Message::text("不該被問"))]);
    let rt = AgentRuntime::new(agent(), model.clone())
        .hook(proceed())
        .tool(tool)
        .capability(Loop::new().max_turns(1))
        .capability(ToolCalling::new())
        .capability(probe(after_report_verify));
    let result = run_with(rt).await;
    assert!(matches!(result, Err(RunError::TurnLimit)));
    assert_eq!(runs.load(Ordering::SeqCst), 2);
    assert_eq!(model.asked.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_duplicate_call_id_runs_once_and_never_continues_early_or_waits_forever() {
    let (runs, tool) = counter();
    let dup = Message::Assistant {
        text: String::new(),
        calls: vec![verify_call(), verify_call()],
    };
    let model = Script::new(vec![Ok(dup), Ok(Message::text("完"))]);
    let out = run(runtime(model.clone(), tool, 8)).await.unwrap();
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    assert_eq!(model.asked.load(Ordering::SeqCst), 2);
    assert_eq!(out.messages[2], tool_message("v1", "已計數", false));
    assert!(matches!(
        &out.messages[3],
        Message::Tool { is_error: true, .. }
    ));
    assert_eq!(out.text, "完");
}

#[tokio::test]
async fn a_model_call_proposed_too_early_waits_until_every_tool_has_reported() {
    let (runs, tool) = counter();
    let two = Message::Assistant {
        text: String::new(),
        calls: vec![
            verify_call(),
            ToolCall {
                id: "v2".into(),
                ..verify_call()
            },
        ],
    };
    let model = Script::new(vec![Ok(two), Ok(Message::text("完"))]);
    let rt = AgentRuntime::new(agent(), model.clone())
        .hook(proceed())
        .tool(tool)
        .capability(Loop::new())
        .capability(ToolCalling::new())
        .capability(probe(always_ask));
    let out = run_with(rt).await.unwrap();
    assert_eq!(runs.load(Ordering::SeqCst), 2);
    assert_eq!(model.asked.load(Ordering::SeqCst), 2);
    assert_eq!(out.messages.len(), 5);
}

#[tokio::test]
async fn without_tool_calling_the_loop_does_not_wait_for_tools_that_never_run() {
    let (runs, tool) = counter();
    let model = Script::new(vec![call_reply("1")]);
    let rt = AgentRuntime::new(agent(), model.clone())
        .hook(proceed())
        .tool(tool)
        .capability(Loop::new());
    let result = run_with(rt).await;
    assert!(matches!(result, Err(RunError::Stalled)));
    assert_eq!(runs.load(Ordering::SeqCst), 0);
    assert_eq!(model.asked.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn with_no_capabilities_nothing_runs_and_the_run_stalls() {
    let (_, tool) = counter();
    let model = Script::new(vec![Ok(Message::text("好"))]);
    let rt = AgentRuntime::new(agent(), model.clone())
        .hook(proceed())
        .tool(tool);
    let result = run_with(rt).await;
    assert!(matches!(result, Err(RunError::Stalled)));
    assert_eq!(model.asked.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_stop_stays_in_force_when_another_capability_later_asks_for_the_model() {
    let (runs, tool) = counter();
    let model = Script::new(vec![call_reply("c1"), Ok(Message::text("not to be asked"))]);
    let rt = AgentRuntime::new(agent(), model.clone())
        .hook(proceed())
        .tool(tool)
        .capability(Loop::new())
        .capability(ToolCalling::new())
        .capability(Probe {
            name: "stopper",
            fired: false,
            propose: stop_after_reply,
            once: true,
        })
        .capability(probe(ask_after_report));
    let result = run_with(rt).await;
    assert!(matches!(result, Err(RunError::TurnLimit)));
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    assert_eq!(model.asked.load(Ordering::SeqCst), 1);
}

fn ask_on_task(event: &Event, _: &CapabilityContext) -> Vec<Intent> {
    match event {
        Event::TaskReceived(_) => vec![Intent::AskModel],
        _ => Vec::new(),
    }
}

fn answer_jia_after_reply(event: &Event, _: &CapabilityContext) -> Vec<Intent> {
    match event {
        Event::ModelReplied(_) => vec![Intent::Answer("甲".into())],
        _ => Vec::new(),
    }
}

fn answer_yi_after_reply(event: &Event, _: &CapabilityContext) -> Vec<Intent> {
    match event {
        Event::ModelReplied(_) => vec![Intent::Answer("乙".into())],
        _ => Vec::new(),
    }
}

fn answer_one_and_call(event: &Event, _: &CapabilityContext) -> Vec<Intent> {
    match event {
        Event::TaskReceived(_) => {
            vec![Intent::Answer("一".into()), Intent::CallTool(verify_call())]
        }
        _ => Vec::new(),
    }
}

fn answer_two_after_report(event: &Event, _: &CapabilityContext) -> Vec<Intent> {
    match event {
        Event::ToolReported { .. } => vec![Intent::Answer("二".into())],
        _ => Vec::new(),
    }
}

async fn tie_answer(first_is_a: bool) -> String {
    let (_, tool) = counter();
    let model = Script::new(vec![Ok(Message::text("原文"))]);
    let a = named("a", answer_jia_after_reply);
    let b = named("b", answer_yi_after_reply);
    let rt = AgentRuntime::new(agent(), model)
        .hook(proceed())
        .tool(tool)
        .capability(named("asker", ask_on_task));
    let rt = if first_is_a {
        rt.capability(a).capability(b)
    } else {
        rt.capability(b).capability(a)
    };
    run_with(rt).await.unwrap().text
}

#[tokio::test]
async fn same_event_exclusive_ties_do_not_depend_on_registration_order() {
    assert_eq!(tie_answer(true).await, "甲");
    assert_eq!(tie_answer(false).await, "甲");
}

#[tokio::test]
async fn the_first_received_answer_is_kept_when_a_later_event_answers_again() {
    let (_, tool) = counter();
    let model = Script::new(Vec::new());
    let rt = AgentRuntime::new(agent(), model)
        .hook(proceed())
        .tool(tool)
        .capability(named("first", answer_one_and_call))
        .capability(named("second", answer_two_after_report));
    assert_eq!(run_with(rt).await.unwrap().text, "一");
}
