use crate::plan::{Plan, error::PlanError, step::Step};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

const DEFAULT_MAX_REVISIONS: usize = 2;

#[derive(Debug)]
struct State {
    plan: Option<Plan>,
    max_revisions: usize,
}

#[derive(Clone, Debug)]
pub struct PlanBoard {
    state: Arc<Mutex<State>>,
}

impl PlanBoard {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                plan: None,
                max_revisions: DEFAULT_MAX_REVISIONS,
            })),
        }
    }

    pub fn max_revisions(self, max_revisions: usize) -> Self {
        self.lock().max_revisions = max_revisions;
        self
    }

    pub fn set(&self, steps: Vec<Step>) -> Result<(), PlanError> {
        let mut state = self.lock();
        let max_revisions = state.max_revisions;
        match state.plan.as_mut() {
            Some(plan) => plan.revise(steps, max_revisions),
            None => {
                state.plan = Some(Plan::new(steps)?);
                Ok(())
            }
        }
    }

    pub fn complete(&self, id: &str) -> Result<(), PlanError> {
        match self.lock().plan.as_mut() {
            Some(plan) => plan.complete(id),
            None => Err(PlanError::NothingToDo),
        }
    }

    pub fn current(&self) -> Option<Step> {
        self.lock().plan.as_ref()?.current().cloned()
    }

    pub fn unfinished(&self) -> usize {
        self.lock().plan.as_ref().map_or(0, Plan::unfinished)
    }

    pub fn allows(&self, tool: &str) -> Result<(), PlanError> {
        match self.lock().plan.as_ref() {
            Some(plan) => plan.allows(tool),
            None => Ok(()),
        }
    }

    pub fn describe(&self) -> String {
        let state = self.lock();
        let Some(plan) = state.plan.as_ref() else {
            return "there is no plan yet".to_owned();
        };
        let current = plan.current().map(|step| step.id.as_str());
        let lines: Vec<String> = plan
            .steps()
            .iter()
            .map(|step| {
                let mark = match (step.done, Some(step.id.as_str()) == current) {
                    (true, _) => "done",
                    (false, true) => "current",
                    (false, false) => "todo",
                };
                format!(
                    "[{mark}] {}: {} (check: {})",
                    step.id, step.goal, step.check
                )
            })
            .collect();
        lines.join("\n")
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for PlanBoard {
    fn default() -> Self {
        Self::new()
    }
}
