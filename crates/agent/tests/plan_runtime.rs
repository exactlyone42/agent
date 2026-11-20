use agent_harness::{
    AgentId, AgentIdentity, CancellationToken, Message, Model, ModelError, MortalAgent, RunError,
    RunOutput, Tool, ToolError, ToolFuture, ToolSpec,
};
use agent_harness_capabilities::{
    CheckFuture, Checks, Loop, PlanBoard, PlanDoneTool, PlanSetTool, Planner, Step, StepCheck,
    ToolCalling,
};
use agent_platform_agent::agent_runtime::AgentRuntime;
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

#[derive(Clone)]
struct Script {
    replies: Arc<Mutex<VecDeque<Message>>>,
    seen: Arc<Mutex<Vec<Vec<Message>>>>,
}

impl Script {
    fn new(replies: Vec<Message>) -> Self {
        Self {
            replies: Arc::new(Mutex::new(replies.into())),
            seen: Arc::default(),
        }
    }
}

impl Model for Script {
    async fn complete(&self, messages: &[Message], _: &[ToolSpec]) -> Result<Message, ModelError> {
        self.seen.lock().unwrap().push(messages.to_vec());
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .ok_or(ModelError("script is empty".into()))
    }
}

struct Work {
    name: &'static str,
    runs: Arc<AtomicUsize>,
}

impl Tool for Work {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.into(),
            description: "work".into(),
            parameters: json!({}),
        }
    }

    fn call(&self, _: Value) -> ToolFuture<'_> {
        Box::pin(async move {
            self.runs.fetch_add(1, Ordering::SeqCst);
            Ok::<String, ToolError>("worked".into())
        })
    }
}

struct Flag(Arc<AtomicBool>);

impl StepCheck for Flag {
    fn run(&self) -> CheckFuture<'_> {
        Box::pin(async move {
            match self.0.load(Ordering::SeqCst) {
                true => Ok(()),
                false => Err("flag is not set".into()),
            }
        })
    }
}

struct Fixture {
    board: PlanBoard,
    flag: Arc<AtomicBool>,
    reads: Arc<AtomicUsize>,
    writes: Arc<AtomicUsize>,
}

fn agent() -> MortalAgent {
    MortalAgent::new(
        AgentId("a1".into()),
        AgentIdentity {
            name: "worker".into(),
        },
    )
}

fn fixture() -> Fixture {
    Fixture {
        board: PlanBoard::new(),
        flag: Arc::new(AtomicBool::new(true)),
        reads: Arc::default(),
        writes: Arc::default(),
    }
}

fn build(fx: &Fixture, model: Script, max_turns: usize) -> AgentRuntime<Script> {
    let checks = Checks::new().with("flag", Flag(fx.flag.clone()));
    AgentRuntime::new(agent(), model)
        .tool(Work {
            name: "read",
            runs: fx.reads.clone(),
        })
        .tool(Work {
            name: "write",
            runs: fx.writes.clone(),
        })
        .tool(PlanSetTool::new(fx.board.clone(), checks.clone()))
        .tool(PlanDoneTool::new(fx.board.clone(), checks))
        .capability(Loop::new().max_turns(max_turns))
        .capability(ToolCalling::new())
        .capability(Planner::new(fx.board.clone()))
}

async fn run(mut runtime: AgentRuntime<Script>) -> Result<RunOutput, RunError> {
    runtime.run("x", &CancellationToken::new()).await
}

fn set_reply(id: &str, steps: Value) -> Message {
    Message::call(id, "plan_set", json!({"steps": steps}))
}

fn two_step_plan() -> Value {
    json!([
        {"id": "a", "goal": "look", "check": "flag", "tools": ["read"]},
        {"id": "b", "goal": "change", "check": "flag", "tools": ["write"]}
    ])
}

fn done_reply(id: &str, step: &str) -> Message {
    Message::call(id, "plan_done", json!({"step": step}))
}

fn tool_results(out: &RunOutput) -> Vec<(String, bool)> {
    out.messages
        .iter()
        .filter_map(|message| match message {
            Message::Tool {
                content, is_error, ..
            } => Some((content.clone(), *is_error)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn full_plan_runs_in_order_and_ends_with_answer() {
    let fx = fixture();
    let model = Script::new(vec![
        set_reply("1", two_step_plan()),
        Message::call("2", "read", json!({})),
        done_reply("3", "a"),
        Message::call("4", "write", json!({})),
        done_reply("5", "b"),
        Message::text("finished"),
    ]);
    let out = run(build(&fx, model, 20)).await.unwrap();
    assert_eq!(out.text, "finished");
    assert_eq!(fx.reads.load(Ordering::SeqCst), 1);
    assert_eq!(fx.writes.load(Ordering::SeqCst), 1);
    assert_eq!(fx.board.unfinished(), 0);
}

#[tokio::test]
async fn answer_before_plan_is_done_is_not_accepted() {
    let fx = fixture();
    let model = Script::new(vec![
        set_reply("1", two_step_plan()),
        Message::text("early answer"),
        Message::call("2", "read", json!({})),
        done_reply("3", "a"),
        Message::call("4", "write", json!({})),
        done_reply("5", "b"),
        Message::text("real answer"),
    ]);
    let out = run(build(&fx, model.clone(), 20)).await.unwrap();
    assert_eq!(out.text, "real answer");
    let second_call = &model.seen.lock().unwrap()[2];
    assert!(matches!(
        second_call.last(),
        Some(Message::User(note)) if note.contains("plan is not finished")
    ));
}

#[tokio::test]
async fn unfinished_plan_ends_at_turn_limit_not_with_answer() {
    let fx = fixture();
    let model = Script::new(vec![
        set_reply("1", two_step_plan()),
        Message::text("answer 1"),
        Message::text("answer 2"),
        Message::text("answer 3"),
        Message::text("answer 4"),
    ]);
    let result = run(build(&fx, model, 20)).await;
    assert!(matches!(result, Err(RunError::TurnLimit)));
    assert_eq!(fx.board.unfinished(), 2);
}

#[tokio::test]
async fn tool_outside_current_step_does_not_run() {
    let fx = fixture();
    let model = Script::new(vec![
        set_reply("1", two_step_plan()),
        Message::call("2", "write", json!({})),
        Message::call("3", "read", json!({})),
        done_reply("4", "a"),
        Message::call("5", "write", json!({})),
        done_reply("6", "b"),
        Message::text("ok"),
    ]);
    let out = run(build(&fx, model, 20)).await.unwrap();
    let results = tool_results(&out);
    assert!(results[1].1 && results[1].0.contains("not allowed in step a"));
    assert_eq!(fx.writes.load(Ordering::SeqCst), 1);
    assert_eq!(fx.reads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn failed_check_keeps_the_step_open() {
    let fx = fixture();
    fx.flag.store(false, Ordering::SeqCst);
    let model = Script::new(vec![
        set_reply("1", two_step_plan()),
        done_reply("2", "a"),
        Message::text("give up 1"),
        Message::text("give up 2"),
        Message::text("give up 3"),
        Message::text("give up 4"),
    ]);
    let result = run(build(&fx, model, 20)).await;
    assert!(matches!(result, Err(RunError::TurnLimit)));
    assert_eq!(fx.board.current().unwrap().id, "a");
}

#[tokio::test]
async fn failed_check_reports_reason_to_model() {
    let fx = fixture();
    fx.flag.store(false, Ordering::SeqCst);
    let model = Script::new(vec![
        set_reply("1", two_step_plan()),
        done_reply("2", "a"),
        Message::text("stop"),
    ]);
    let model_seen = model.clone();
    let _ = run(build(&fx, model, 3)).await;
    let last_seen = model_seen.seen.lock().unwrap().last().cloned().unwrap();
    let report = last_seen.iter().find_map(|message| match message {
        Message::Tool {
            call_id,
            content,
            is_error,
        } if call_id == "2" => Some((content.clone(), *is_error)),
        _ => None,
    });
    let (content, is_error) = report.unwrap();
    assert!(is_error && content.contains("flag is not set"));
}

#[tokio::test]
async fn skipping_a_step_is_refused() {
    let fx = fixture();
    let model = Script::new(vec![
        set_reply("1", two_step_plan()),
        done_reply("2", "b"),
        Message::text("x"),
    ]);
    let _ = run(build(&fx, model, 3)).await;
    assert_eq!(fx.board.current().unwrap().id, "a");
}

#[tokio::test]
async fn host_seeded_plan_is_enforced_without_model_cooperation() {
    let fx = fixture();
    fx.board
        .set(vec![Step::new("a", "look", "flag").tools(&["read"])])
        .unwrap();
    let model = Script::new(vec![
        Message::text("skip it"),
        Message::call("1", "read", json!({})),
        done_reply("2", "a"),
        Message::text("done"),
    ]);
    let out = run(build(&fx, model, 20)).await.unwrap();
    assert_eq!(out.text, "done");
    assert_eq!(fx.reads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn unknown_check_name_is_refused_when_planning() {
    let fx = fixture();
    let bad = json!([{"id": "a", "goal": "g", "check": "nope"}]);
    let model = Script::new(vec![set_reply("1", bad), Message::text("x")]);
    let out = run(build(&fx, model, 20)).await.unwrap();
    let results = tool_results(&out);
    assert!(results[0].1 && results[0].0.contains("no check named nope"));
    assert_eq!(fx.board.unfinished(), 0);
}

#[tokio::test]
async fn third_revision_is_refused() {
    let fx = fixture();
    let one = |id: &str| json!([{"id": id, "goal": "g", "check": "flag"}]);
    let model = Script::new(vec![
        set_reply("1", one("a")),
        set_reply("2", one("b")),
        set_reply("3", one("c")),
        set_reply("4", one("d")),
        done_reply("5", "c"),
        Message::text("end"),
    ]);
    let out = run(build(&fx, model, 20)).await.unwrap();
    let results = tool_results(&out);
    assert!(results[3].1 && results[3].0.contains("revised 2 times"));
    assert_eq!(out.text, "end");
}
