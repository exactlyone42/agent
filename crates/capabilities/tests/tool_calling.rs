use agent_harness::{Capability, CapabilityContext, Event, Intent, Message, ToolCall};
use agent_harness_capabilities::ToolCalling;
use serde_json::json;

const CONTEXT: CapabilityContext = CapabilityContext {
    turn: 0,
    pending_tools: 0,
};

fn call(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "count".into(),
        args: json!({"id": id}),
    }
}

#[test]
fn every_call_in_a_reply_becomes_a_call_intent_in_order() {
    let mut capability = ToolCalling::new();
    let reply = Message::Assistant {
        text: String::new(),
        calls: vec![call("c1"), call("c2")],
    };
    let intents = capability.on_event(&Event::ModelReplied(reply), &CONTEXT);
    assert_eq!(
        intents,
        vec![Intent::CallTool(call("c1")), Intent::CallTool(call("c2"))]
    );
}

#[test]
fn a_reply_without_calls_proposes_nothing() {
    let mut capability = ToolCalling::new();
    let intents = capability.on_event(&Event::ModelReplied(Message::text("答案")), &CONTEXT);
    assert!(intents.is_empty());
}

#[test]
fn other_events_propose_nothing() {
    let mut capability = ToolCalling::new();
    let report = Event::ToolReported {
        call_id: "c1".into(),
        content: "完成".into(),
        is_error: false,
    };
    assert!(
        capability
            .on_event(&Event::TaskReceived("題目".into()), &CONTEXT)
            .is_empty()
    );
    assert!(capability.on_event(&report, &CONTEXT).is_empty());
}

#[test]
fn it_never_asks_the_model_or_finishes() {
    let mut capability = ToolCalling::new();
    let reply = Message::Assistant {
        text: "文字".into(),
        calls: vec![call("c1")],
    };
    let intents = capability.on_event(&Event::ModelReplied(reply), &CONTEXT);
    assert!(
        intents
            .iter()
            .all(|intent| matches!(intent, Intent::CallTool(_)))
    );
}
