use agent_harness::{Intent, StopReason, ToolCall};
use agent_platform_agent::arbitration::Arbitration;
use serde_json::json;

fn call(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "count".into(),
        args: json!({}),
    }
}

#[test]
fn compatible_intents_all_survive() {
    let decision = Arbitration::decide(vec![
        Intent::Answer("答".into()),
        Intent::CallTool(call("a")),
        Intent::CallTool(call("b")),
    ]);
    assert_eq!(decision.answer.as_deref(), Some("答"));
    assert_eq!(decision.calls, vec![call("a"), call("b")]);
}

#[test]
fn stop_outranks_ask_and_answer_but_not_tools() {
    let decision = Arbitration::decide(vec![
        Intent::AskModel,
        Intent::Answer("答".into()),
        Intent::CallTool(call("a")),
        Intent::Stop(StopReason::LimitReached),
    ]);
    assert_eq!(decision.stop, Some(StopReason::LimitReached));
    assert!(!decision.ask);
    assert_eq!(decision.answer, None);
    assert_eq!(decision.calls, vec![call("a")]);
}

#[test]
fn ask_outranks_answer() {
    let decision = Arbitration::decide(vec![Intent::Answer("答".into()), Intent::AskModel]);
    assert!(decision.ask);
    assert_eq!(decision.answer, None);
}

#[test]
fn equal_level_answers_go_to_the_first_received() {
    let decision = Arbitration::decide(vec![
        Intent::Answer("一".into()),
        Intent::Answer("二".into()),
    ]);
    assert_eq!(decision.answer.as_deref(), Some("一"));
}

#[test]
fn nothing_proposed_means_nothing_decided() {
    assert_eq!(Arbitration::decide(Vec::new()), Arbitration::default());
}

fn proposal(capability: &str, intents: Vec<Intent>) -> (String, Vec<Intent>) {
    (capability.to_owned(), intents)
}

#[test]
fn same_event_ties_go_to_the_smaller_capability_id_in_any_input_order() {
    let a = proposal("a", vec![Intent::Answer("甲".into())]);
    let b = proposal("b", vec![Intent::Answer("乙".into())]);
    let forward = Arbitration::decide_by_capability(vec![a.clone(), b.clone()]);
    let backward = Arbitration::decide_by_capability(vec![b, a]);
    assert_eq!(forward.answer.as_deref(), Some("甲"));
    assert_eq!(forward, backward);
}

#[test]
fn calls_run_by_capability_id_then_by_the_capability_own_order() {
    let a = proposal(
        "a",
        vec![Intent::CallTool(call("a1")), Intent::CallTool(call("a2"))],
    );
    let b = proposal("b", vec![Intent::CallTool(call("b1"))]);
    let forward = Arbitration::decide_by_capability(vec![a.clone(), b.clone()]);
    let backward = Arbitration::decide_by_capability(vec![b, a]);
    assert_eq!(forward.calls, vec![call("a1"), call("a2"), call("b1")]);
    assert_eq!(forward, backward);
}

#[test]
fn one_capability_keeps_its_own_order_when_it_ties_with_itself() {
    let only = proposal(
        "a",
        vec![Intent::Answer("先".into()), Intent::Answer("後".into())],
    );
    let decision = Arbitration::decide_by_capability(vec![only]);
    assert_eq!(decision.answer.as_deref(), Some("先"));
}
