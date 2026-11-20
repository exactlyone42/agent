use agent_harness::{Capability, CapabilityContext, Event, Intent, Message, StopReason, ToolCall};
use agent_harness_capabilities::Loop;
use serde_json::json;

fn after(turn: usize, pending_tools: usize) -> CapabilityContext {
    CapabilityContext {
        turn,
        pending_tools,
    }
}

fn reported(id: &str) -> Event {
    Event::ToolReported {
        call_id: id.into(),
        content: "完成".into(),
        is_error: false,
    }
}

fn two_calls() -> Event {
    let call = |id: &str| ToolCall {
        id: id.into(),
        name: "count".into(),
        args: json!({}),
    };
    Event::ModelReplied(Message::Assistant {
        text: String::new(),
        calls: vec![call("c1"), call("c2")],
    })
}

#[test]
fn a_new_task_starts_with_one_model_call() {
    let mut turn_loop = Loop::new();
    let intents = turn_loop.on_event(&Event::TaskReceived("題目".into()), &after(0, 0));
    assert_eq!(intents, vec![Intent::AskModel]);
}

#[test]
fn a_reply_without_calls_proposes_its_text_as_the_answer_and_nothing_else() {
    let mut turn_loop = Loop::new();
    let intents = turn_loop.on_event(&Event::ModelReplied(Message::text("答案")), &after(1, 0));
    assert_eq!(intents, vec![Intent::Answer("答案".into())]);
}

#[test]
fn a_reply_with_calls_proposes_nothing_itself() {
    let mut turn_loop = Loop::new();
    assert!(turn_loop.on_event(&two_calls(), &after(1, 0)).is_empty());
}

#[test]
fn it_waits_while_the_runtime_still_counts_pending_tools() {
    let mut turn_loop = Loop::new();
    assert!(turn_loop.on_event(&reported("c1"), &after(1, 2)).is_empty());
    assert!(turn_loop.on_event(&reported("c2"), &after(1, 1)).is_empty());
    assert_eq!(
        turn_loop.on_event(&reported("c2"), &after(1, 0)),
        vec![Intent::AskModel]
    );
}

#[test]
fn repeating_a_report_does_not_move_it_toward_the_next_model_call() {
    let mut turn_loop = Loop::new();
    for _ in 0..3 {
        assert!(turn_loop.on_event(&reported("c1"), &after(1, 1)).is_empty());
    }
}

#[test]
fn the_default_limit_is_eight_model_calls() {
    let mut turn_loop = Loop::new();
    assert_eq!(
        turn_loop.on_event(&reported("c1"), &after(7, 0)),
        vec![Intent::AskModel]
    );
    assert_eq!(
        turn_loop.on_event(&reported("c1"), &after(8, 0)),
        vec![Intent::Stop(StopReason::LimitReached)]
    );
}

#[test]
fn a_custom_limit_stops_after_that_many_model_calls() {
    let mut turn_loop = Loop::new().max_turns(1);
    assert_eq!(
        turn_loop.on_event(&reported("c1"), &after(1, 0)),
        vec![Intent::Stop(StopReason::LimitReached)]
    );
}

#[test]
fn the_limit_waits_for_pending_tools_too() {
    let mut turn_loop = Loop::new().max_turns(1);
    assert!(turn_loop.on_event(&reported("c1"), &after(1, 1)).is_empty());
}

#[test]
fn it_never_proposes_a_tool_call() {
    let mut turn_loop = Loop::new();
    let events = [
        Event::TaskReceived("題目".into()),
        two_calls(),
        reported("c1"),
        reported("c2"),
        Event::ModelReplied(Message::text("答案")),
    ];
    for event in &events {
        let intents = turn_loop.on_event(event, &after(1, 0));
        assert!(
            intents
                .iter()
                .all(|intent| !matches!(intent, Intent::CallTool(_)))
        );
    }
}
