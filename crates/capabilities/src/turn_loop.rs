use crate::prelude::*;

const DEFAULT_MAX_TURNS: usize = 8;

#[derive(Debug)]
pub struct Loop {
    max_turns: usize,
}

impl Loop {
    pub fn new() -> Self {
        Self {
            max_turns: DEFAULT_MAX_TURNS,
        }
    }

    pub fn max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns;
        self
    }

    fn after_reply(reply: &Message) -> Vec<Intent> {
        match reply {
            Message::Assistant { text, calls } if calls.is_empty() => {
                vec![Intent::Answer(text.clone())]
            }
            _ => Vec::new(),
        }
    }

    fn after_report(&self, context: &CapabilityContext) -> Vec<Intent> {
        if context.pending_tools > 0 {
            return Vec::new();
        }
        if context.turn >= self.max_turns {
            return vec![Intent::Stop(StopReason::LimitReached)];
        }
        vec![Intent::AskModel]
    }
}

impl Default for Loop {
    fn default() -> Self {
        Self::new()
    }
}

impl Capability for Loop {
    fn id(&self) -> &str {
        "loop"
    }

    fn on_event(&mut self, event: &Event, context: &CapabilityContext) -> Vec<Intent> {
        match event {
            Event::TaskReceived(_) => vec![Intent::AskModel],
            Event::ModelReplied(reply) => Self::after_reply(reply),
            Event::ToolReported { .. } => self.after_report(context),
        }
    }
}
