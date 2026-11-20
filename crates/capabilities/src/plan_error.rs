#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("a plan needs at least one step")]
    Empty,
    #[error("a plan may have at most {0} steps")]
    TooManySteps(usize),
    #[error("step field {0} must not be empty")]
    BlankField(&'static str),
    #[error("step id {0} is used twice")]
    DuplicateId(String),
    #[error("the plan was already revised {0} times; no more revisions are allowed")]
    RevisionLimit(usize),
    #[error("there is no unfinished step")]
    NothingToDo,
    #[error("step {asked} is not the current step; the current step is {current}")]
    NotCurrent { asked: String, current: String },
    #[error("tool {tool} is not allowed in step {step}; allowed tools: {allowed}")]
    ToolNotInStep {
        tool: String,
        step: String,
        allowed: String,
    },
    #[error("there is no check named {name}; available checks: {available}")]
    UnknownCheck { name: String, available: String },
}
