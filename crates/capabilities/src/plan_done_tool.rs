use crate::{plan_board::PlanBoard, plan_error::PlanError, step_checks::Checks};
use agent_harness::{Tool, ToolError, ToolFuture, ToolSpec};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DoneArgs {
    step: String,
}

pub struct PlanDoneTool {
    board: PlanBoard,
    checks: Checks,
}

impl PlanDoneTool {
    pub const NAME: &'static str = "plan_done";

    pub fn new(board: PlanBoard, checks: Checks) -> Self {
        Self { board, checks }
    }

    async fn finish(&self, args: Value) -> Result<String, ToolError> {
        let parsed: DoneArgs = serde_json::from_value(args).map_err(ToolError::failed)?;
        let current = self
            .board
            .current()
            .ok_or(ToolError::failed(PlanError::NothingToDo))?;
        if current.id != parsed.step {
            return Err(ToolError::failed(PlanError::NotCurrent {
                asked: parsed.step,
                current: current.id,
            }));
        }
        let check = self
            .checks
            .find(&current.check)
            .map_err(ToolError::failed)?;
        if let Err(reason) = check.run().await {
            return Err(ToolError(format!(
                "check {} failed for step {}: {reason}. Fix it and call plan_done again, or revise the plan with plan_set",
                current.check, current.id
            )));
        }
        self.board
            .complete(&current.id)
            .map_err(ToolError::failed)?;
        Ok(match self.board.current() {
            Some(next) => format!(
                "step {} is done. Next step {}: {}",
                current.id, next.id, next.goal
            ),
            None => format!(
                "step {} is done. All steps are done; give the final answer",
                current.id
            ),
        })
    }
}

impl Tool for PlanDoneTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: Self::NAME.to_owned(),
            description: "Report that the current step is finished. The program runs the step's check; the step counts as done only when the check passes.".to_owned(),
            parameters: json!({
                "type": "object",
                "properties": {"step": {"type": "string"}},
                "required": ["step"]
            }),
        }
    }

    fn call(&self, args: Value) -> ToolFuture<'_> {
        Box::pin(self.finish(args))
    }
}
