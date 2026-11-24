use crate::prelude::*;
use serde_json::json;

const DESCRIPTION: &str = "Save the step plan, or revise the steps that are not done yet. Steps run in order. Each step names a check that the program runs before the step counts as done.";

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct StepArgs {
    id: String,
    goal: String,
    check: String,
    #[serde(default)]
    tools: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SetArgs {
    steps: Vec<StepArgs>,
}

pub struct PlanSetTool {
    board: PlanBoard,
    checks: Checks,
}

impl PlanSetTool {
    pub const NAME: &'static str = "plan_set";

    pub fn new(board: PlanBoard, checks: Checks) -> Self {
        Self { board, checks }
    }

    fn apply(&self, args: Value) -> Result<String, ToolError> {
        let parsed: SetArgs = serde_json::from_value(args).map_err(ToolError::failed)?;
        let mut steps = Vec::with_capacity(parsed.steps.len());
        for item in parsed.steps {
            self.checks.find(&item.check).map_err(ToolError::failed)?;
            let tools: Vec<&str> = item.tools.iter().map(String::as_str).collect();
            steps.push(Step::new(&item.id, &item.goal, &item.check).tools(&tools));
        }
        self.board.set(steps).map_err(ToolError::failed)?;
        Ok(format!("plan saved\n{}", self.board.describe()))
    }
}

impl Tool for PlanSetTool {
    fn spec(&self) -> ToolSpec {
        let mut spec = ToolSpec::typed::<SetArgs>(Self::NAME, DESCRIPTION);
        if let Some(check) = spec
            .parameters
            .pointer_mut("/properties/steps/items/properties/check")
        {
            check["enum"] = json!(self.checks.names());
        }
        spec
    }

    fn call(&self, args: Value) -> ToolFuture<'_> {
        Box::pin(async move { self.apply(args) })
    }
}
