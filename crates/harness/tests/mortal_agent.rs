use agent_harness::{
    AgentId, AgentIdentity, CancellationToken, Capability, CapabilityBinding, CapabilityContext,
    Effect, Event, Goal, Intent, Message, Model, ModelError, MortalAgent, RunError, StopReason,
    ToolCall, ToolSpec, Work,
};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Fixed {
    reply: Message,
    asked: Arc<AtomicUsize>,
}

impl Model for Fixed {
    async fn complete(&self, _: &[Message], _: &[ToolSpec]) -> Result<Message, ModelError> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        Ok(self.reply.clone())
    }
}

fn fixed(reply: Message) -> (Fixed, Arc<AtomicUsize>) {
    let asked = Arc::new(AtomicUsize::new(0));
    let model = Fixed {
        reply,
        asked: asked.clone(),
    };
    (model, asked)
}

fn agent() -> MortalAgent {
    MortalAgent::new(
        AgentId("a1".into()),
        AgentIdentity {
            name: "worker".into(),
        },
    )
}

#[tokio::test]
async fn think_asks_the_model_once_and_stops_even_when_the_reply_has_tool_calls() {
    let (model, asked) = fixed(Message::call("c1", "count", json!({})));
    let work = Work::new("做事");
    let reply = agent()
        .think(&model, &work, &[], &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(reply, Message::call("c1", "count", json!({})));
    assert_eq!(asked.load(Ordering::SeqCst), 1);
    assert_eq!(work.messages().len(), 1);
}

#[tokio::test]
async fn think_after_cancel_never_reaches_the_model() {
    let (model, asked) = fixed(Message::text("不該被問"));
    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = agent()
        .think(&model, &Work::new("做事"), &[], &cancel)
        .await;
    assert!(matches!(result, Err(RunError::Cancelled)));
    assert_eq!(asked.load(Ordering::SeqCst), 0);
}

#[test]
fn work_starts_with_the_prompt_and_keeps_order() {
    let mut work = Work::new("題目");
    work.push(Message::text("回覆"));
    assert_eq!(
        work.messages(),
        [Message::User("題目".into()), Message::text("回覆")]
    );
}

#[test]
fn builder_keeps_goals_and_bindings_in_order() {
    let built = agent()
        .goal(Goal { text: "一".into() })
        .goal(Goal { text: "二".into() })
        .capability(CapabilityBinding::new("loop", "1"))
        .capability(CapabilityBinding::new("tools", "2").settings(json!({"max": 3})));
    assert_eq!(built.goals[1].text, "二");
    assert_eq!(built.capabilities[0].settings, json!(null));
    assert_eq!(built.capabilities[1].settings, json!({"max": 3}));
    assert_eq!(built.capabilities[1].name, "tools");
}

#[test]
fn only_intents_that_touch_the_world_map_to_an_effect() {
    let call = ToolCall {
        id: "c1".into(),
        name: "count".into(),
        args: json!({}),
    };
    assert_eq!(Intent::AskModel.effect(), Some(Effect::CallModel));
    assert_eq!(
        Intent::CallTool(call.clone()).effect(),
        Some(Effect::RunTool(call))
    );
    assert_eq!(Intent::Answer("完成".into()).effect(), None);
    assert_eq!(Intent::Stop(StopReason::LimitReached).effect(), None);
}

struct Echo;

impl Capability for Echo {
    fn id(&self) -> &str {
        "echo"
    }

    fn on_event(&mut self, event: &Event, _: &CapabilityContext) -> Vec<Intent> {
        match event {
            Event::ModelReplied(Message::Assistant { calls, text }) if calls.is_empty() => {
                vec![Intent::Answer(text.clone())]
            }
            Event::ModelReplied(Message::Assistant { calls, .. }) => {
                calls.iter().cloned().map(Intent::CallTool).collect()
            }
            _ => Vec::new(),
        }
    }
}

#[test]
fn a_capability_behind_a_trait_object_only_returns_intents() {
    let mut capability: Box<dyn Capability> = Box::new(Echo);
    let context = CapabilityContext {
        turn: 0,
        pending_tools: 0,
    };
    let finish = capability.on_event(&Event::ModelReplied(Message::text("好")), &context);
    assert_eq!(finish, vec![Intent::Answer("好".into())]);
    let calls = capability.on_event(
        &Event::ModelReplied(Message::call("c1", "count", json!({}))),
        &context,
    );
    assert!(matches!(calls.as_slice(), [Intent::CallTool(call)] if call.id == "c1"));
    let none = capability.on_event(&Event::TaskReceived("題目".into()), &context);
    assert!(none.is_empty());
}
