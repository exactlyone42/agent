use crate::{runtime::arbitration::Arbitration, runtime::pending_tools::PendingTools};
use agent_harness::{
    CapabilityContext, Event, Message, RunError, RunOutput, StopReason, ToolCall, Work,
};
use std::collections::VecDeque;

pub struct RunState {
    pub work: Work,
    pub pending: PendingTools,
    pub queue: VecDeque<Event>,
    pub turn: usize,
    answer: Option<String>,
    stop: Option<StopReason>,
    ask_pending: bool,
    notes: Vec<String>,
}

impl RunState {
    pub fn new(prompt: &str) -> Self {
        Self {
            work: Work::new(prompt),
            pending: PendingTools::default(),
            queue: VecDeque::from([Event::TaskReceived(prompt.to_owned())]),
            turn: 0,
            answer: None,
            stop: None,
            ask_pending: false,
            notes: Vec::new(),
        }
    }

    pub fn context(&self) -> CapabilityContext {
        CapabilityContext {
            turn: self.turn,
            pending_tools: self.pending.len(),
        }
    }

    pub fn accepts(&mut self, event: &Event) -> bool {
        match event {
            Event::ToolReported { call_id, .. } => self.pending.report(call_id),
            _ => true,
        }
    }

    pub fn adopt(&mut self, arbitration: &Arbitration) {
        self.stop = self.stop.or(arbitration.stop);
        self.ask_pending |= arbitration.ask;
        self.notes.extend(arbitration.notes.iter().cloned());
        if self.answer.is_none() {
            self.answer.clone_from(&arbitration.answer);
        }
    }

    pub fn expect_calls(&mut self, calls: &[ToolCall]) -> Vec<bool> {
        calls
            .iter()
            .map(|call| self.pending.expect(&call.id))
            .collect()
    }

    pub fn refuse_duplicate(&mut self, call: &ToolCall) {
        self.work.push(Message::Tool {
            call_id: call.id.clone(),
            content: format!("the call id {} is already in use", call.id),
            is_error: true,
        });
    }

    pub fn record_report(&mut self, call_id: &str, content: String, is_error: bool) {
        self.work.push(Message::Tool {
            call_id: call_id.to_owned(),
            content: content.clone(),
            is_error,
        });
        self.queue.push_back(Event::ToolReported {
            call_id: call_id.to_owned(),
            content,
            is_error,
        });
    }

    pub fn ready_to_ask(&self) -> bool {
        self.ask_pending && self.pending.is_empty() && self.stop.is_none()
    }

    pub fn start_model_call(&mut self) {
        self.ask_pending = false;
        self.answer = None;
        for note in std::mem::take(&mut self.notes) {
            self.work.push(Message::User(note));
        }
    }

    pub fn record_reply(&mut self, reply: Message) {
        self.turn += 1;
        self.work.push(reply.clone());
        self.queue.push_back(Event::ModelReplied(reply));
    }

    pub fn finish(self) -> Result<RunOutput, RunError> {
        if let Some(StopReason::LimitReached) = self.stop {
            return Err(RunError::TurnLimit);
        }
        let text = self.answer.ok_or(RunError::Stalled)?;
        Ok(RunOutput {
            messages: self.work.messages().to_vec(),
            text,
        })
    }
}
