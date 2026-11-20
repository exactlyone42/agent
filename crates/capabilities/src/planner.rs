use crate::{plan_board::PlanBoard, plan_done_tool::PlanDoneTool, plan_set_tool::PlanSetTool};
use agent_harness::{Capability, CapabilityContext, Event, Intent, Message, StopReason, ToolCall};

const DEFAULT_MAX_NUDGES: usize = 3;

pub struct Planner {
    board: PlanBoard,
    nudges: usize,
    max_nudges: usize,
}

impl Planner {
    pub fn new(board: PlanBoard) -> Self {
        Self {
            board,
            nudges: 0,
            max_nudges: DEFAULT_MAX_NUDGES,
        }
    }

    pub fn max_nudges(mut self, max_nudges: usize) -> Self {
        self.max_nudges = max_nudges;
        self
    }

    fn is_plan_tool(name: &str) -> bool {
        name == PlanSetTool::NAME || name == PlanDoneTool::NAME
    }
}

impl Capability for Planner {
    fn id(&self) -> &str {
        "planner"
    }

    fn on_event(&mut self, event: &Event, _: &CapabilityContext) -> Vec<Intent> {
        let Event::ModelReplied(Message::Assistant { calls, .. }) = event else {
            return Vec::new();
        };
        if !calls.is_empty() || self.board.unfinished() == 0 {
            return Vec::new();
        }
        if self.nudges >= self.max_nudges {
            return vec![Intent::Stop(StopReason::LimitReached)];
        }
        self.nudges += 1;
        let note = format!(
            "The plan is not finished, so the answer was not accepted. Continue with the current step and call plan_done when it is finished.\n{}",
            self.board.describe()
        );
        vec![Intent::Note(note), Intent::AskModel]
    }

    fn forbids(&self, call: &ToolCall) -> Option<String> {
        if Self::is_plan_tool(&call.name) {
            return None;
        }
        self.board
            .allows(&call.name)
            .err()
            .map(|error| error.to_string())
    }
}
