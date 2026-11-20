use agent_harness::{
    AgentId, AgentIdentity, CancellationToken, Capability, CapabilityContext, Event, Intent,
    Message, Model, ModelError, MortalAgent, ToolSpec, Work,
};
use agent_harness_capabilities::{Loop, ToolCalling};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Fixed(Arc<AtomicUsize>);

impl Model for Fixed {
    async fn complete(&self, _: &[Message], _: &[ToolSpec]) -> Result<Message, ModelError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(Message::call("c1", "count", json!({})))
    }
}

fn run(capabilities: &mut [Box<dyn Capability>], event: &Event, turn: usize) -> Vec<Intent> {
    let context = CapabilityContext {
        turn,
        pending_tools: 0,
    };
    capabilities
        .iter_mut()
        .flat_map(|capability| capability.on_event(event, &context))
        .collect()
}

#[test]
fn each_capability_loads_alone() {
    let reply = Event::ModelReplied(Message::call("c1", "count", json!({})));
    let mut only_loop: Vec<Box<dyn Capability>> = vec![Box::new(Loop::new())];
    assert!(run(&mut only_loop, &reply, 0).is_empty());
    let mut only_tools: Vec<Box<dyn Capability>> = vec![Box::new(ToolCalling::new())];
    assert!(matches!(
        run(&mut only_tools, &reply, 0).as_slice(),
        [Intent::CallTool(_)]
    ));
}

#[test]
fn loaded_together_the_loop_waits_for_the_tool_report_then_asks_again() {
    let mut both: Vec<Box<dyn Capability>> =
        vec![Box::new(Loop::new()), Box::new(ToolCalling::new())];
    let reply = Event::ModelReplied(Message::call("c1", "count", json!({})));
    let first = run(&mut both, &reply, 0);
    assert!(matches!(first.as_slice(), [Intent::CallTool(_)]));
    let report = Event::ToolReported {
        call_id: "c1".into(),
        content: "完成".into(),
        is_error: false,
    };
    assert_eq!(run(&mut both, &report, 0), vec![Intent::AskModel]);
}

#[test]
fn registration_order_does_not_change_the_intents() {
    let reply = Event::ModelReplied(Message::call("c1", "count", json!({})));
    let mut a: Vec<Box<dyn Capability>> = vec![Box::new(Loop::new()), Box::new(ToolCalling::new())];
    let mut b: Vec<Box<dyn Capability>> = vec![Box::new(ToolCalling::new()), Box::new(Loop::new())];
    assert_eq!(run(&mut a, &reply, 0), run(&mut b, &reply, 0));
}

#[tokio::test]
async fn think_is_the_same_with_or_without_capabilities() {
    let asked = Arc::new(AtomicUsize::new(0));
    let model = Fixed(asked.clone());
    let agent = MortalAgent::new(AgentId("a1".into()), AgentIdentity { name: "w".into() });
    let work = Work::new("做事");
    let cancel = CancellationToken::new();
    let before = agent.think(&model, &work, &[], &cancel).await.unwrap();
    let _loaded: Vec<Box<dyn Capability>> =
        vec![Box::new(Loop::new()), Box::new(ToolCalling::new())];
    let after = agent.think(&model, &work, &[], &cancel).await.unwrap();
    assert_eq!(before, after);
    assert_eq!(asked.load(Ordering::SeqCst), 2);
}

#[test]
fn capability_ids_are_stable_and_distinct() {
    assert_eq!(Loop::new().id(), "loop");
    assert_eq!(ToolCalling::new().id(), "tool_calling");
}
