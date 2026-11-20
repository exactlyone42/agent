use crate::{CapabilityContext, Event, Intent, ToolCall};

pub trait Capability: Send + Sync {
    fn id(&self) -> &str;

    fn on_event(&mut self, event: &Event, context: &CapabilityContext) -> Vec<Intent>;

    fn forbids(&self, _call: &ToolCall) -> Option<String> {
        None
    }
}
