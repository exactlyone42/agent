use crate::prelude::*;
use agent_harness::Intent;

#[derive(Debug, Default, PartialEq)]
pub struct Arbitration {
    pub stop: Option<StopReason>,
    pub ask: bool,
    pub answer: Option<String>,
    pub calls: Vec<ToolCall>,
    pub notes: Vec<String>,
}

impl Arbitration {
    pub fn decide_by_capability(mut proposals: Vec<(String, Vec<Intent>)>) -> Self {
        proposals.sort_by(|left, right| left.0.cmp(&right.0));
        Self::decide(
            proposals
                .into_iter()
                .flat_map(|(_, intents)| intents)
                .collect(),
        )
    }

    pub fn decide(intents: Vec<Intent>) -> Self {
        let mut decision = Self::default();
        for intent in intents {
            decision.take(intent);
        }
        decision.settle_exclusive();
        decision
    }

    fn take(&mut self, intent: Intent) {
        match intent {
            Intent::CallTool(call) => self.calls.push(call),
            Intent::AskModel => self.ask = true,
            Intent::Note(text) => self.notes.push(text),
            Intent::Answer(text) => {
                self.answer.get_or_insert(text);
            }
            Intent::Stop(reason) => {
                self.stop.get_or_insert(reason);
            }
        }
    }

    fn settle_exclusive(&mut self) {
        if self.stop.is_some() {
            self.ask = false;
            self.answer = None;
            return;
        }
        if self.ask {
            self.answer = None;
        }
    }
}
