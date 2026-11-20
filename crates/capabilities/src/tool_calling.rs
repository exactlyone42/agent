use agent_harness::{Capability, CapabilityContext, Event, Intent, Message};

#[derive(Debug, Default)]
pub struct ToolCalling;

impl ToolCalling {
    pub fn new() -> Self {
        Self
    }
}

impl Capability for ToolCalling {
    fn id(&self) -> &str {
        "tool_calling"
    }

    fn on_event(&mut self, event: &Event, _: &CapabilityContext) -> Vec<Intent> {
        let Event::ModelReplied(Message::Assistant { calls, .. }) = event else {
            return Vec::new();
        };
        calls.iter().cloned().map(Intent::CallTool).collect()
    }
}
