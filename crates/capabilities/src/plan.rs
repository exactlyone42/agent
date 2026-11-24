use crate::prelude::*;
pub(crate) mod board;
pub(crate) mod done_tool;
pub(crate) mod error;
pub(crate) mod planner;
pub(crate) mod set_tool;
pub(crate) mod step;
pub(crate) mod step_check;
pub(crate) mod step_checks;

pub const MAX_STEPS: usize = 20;

#[derive(Clone, Debug, Default)]
pub struct Plan {
    steps: Vec<Step>,
    revisions: usize,
}

impl Plan {
    pub fn new(steps: Vec<Step>) -> Result<Self, PlanError> {
        Self::validate(&steps, &[])?;
        Ok(Self {
            steps,
            revisions: 0,
        })
    }

    pub fn revise(&mut self, rest: Vec<Step>, max_revisions: usize) -> Result<(), PlanError> {
        if self.revisions >= max_revisions {
            return Err(PlanError::RevisionLimit(max_revisions));
        }
        let mut kept: Vec<Step> = self
            .steps
            .iter()
            .filter(|step| step.done)
            .cloned()
            .collect();
        Self::validate(&rest, &kept)?;
        kept.extend(rest);
        self.steps = kept;
        self.revisions += 1;
        Ok(())
    }

    pub fn current(&self) -> Option<&Step> {
        self.steps.iter().find(|step| !step.done)
    }

    pub fn complete(&mut self, id: &str) -> Result<(), PlanError> {
        let current = self.current().ok_or(PlanError::NothingToDo)?.id.clone();
        if current != id {
            return Err(PlanError::NotCurrent {
                asked: id.to_owned(),
                current,
            });
        }
        if let Some(step) = self.steps.iter_mut().find(|step| step.id == id) {
            step.done = true;
        }
        Ok(())
    }

    pub fn unfinished(&self) -> usize {
        self.steps.iter().filter(|step| !step.done).count()
    }

    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    pub fn allows(&self, tool: &str) -> Result<(), PlanError> {
        let Some(step) = self.current() else {
            return Ok(());
        };
        if step.tools.is_empty() || step.tools.iter().any(|allowed| allowed == tool) {
            return Ok(());
        }
        Err(PlanError::ToolNotInStep {
            tool: tool.to_owned(),
            step: step.id.clone(),
            allowed: step.tools.join(", "),
        })
    }

    fn validate(steps: &[Step], kept: &[Step]) -> Result<(), PlanError> {
        if steps.is_empty() {
            return Err(PlanError::Empty);
        }
        if steps.len() + kept.len() > MAX_STEPS {
            return Err(PlanError::TooManySteps(MAX_STEPS));
        }
        steps
            .iter()
            .enumerate()
            .try_for_each(|(index, step)| Self::validate_step(step, kept, &steps[..index]))
    }

    fn validate_step(step: &Step, kept: &[Step], before: &[Step]) -> Result<(), PlanError> {
        let blank = [
            ("id", &step.id),
            ("goal", &step.goal),
            ("check", &step.check),
        ]
        .into_iter()
        .find(|(_, value)| value.trim().is_empty());
        if let Some((name, _)) = blank {
            return Err(PlanError::BlankField(name));
        }
        if kept.iter().chain(before).any(|other| other.id == step.id) {
            return Err(PlanError::DuplicateId(step.id.clone()));
        }
        Ok(())
    }
}
