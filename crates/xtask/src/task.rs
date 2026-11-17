use crate::prelude::*;
use crate::{
    check_all::check_all, gate::gate_steps, run_steps::run_steps, run_structure::run_structure,
    step_names::ALL_TASK,
};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task {
    Gate,
    All,
}

impl Task {
    pub fn parse(name: Option<&str>) -> Option<Self> {
        match name {
            Some(GATE_TASK) => Some(Self::Gate),
            Some(ALL_TASK) => Some(Self::All),
            _ => None,
        }
    }

    pub fn run(self, root: &Path) -> Result<Outcome, XtaskError> {
        match self {
            Self::Gate => Self::gate(root),
            Self::All => Self::all(root),
        }
    }

    fn gate(root: &Path) -> Result<Outcome, XtaskError> {
        if run_structure(root)? == Outcome::Failed {
            return Ok(Outcome::Failed);
        }
        run_steps(root, &gate_steps(root))
    }

    fn all(root: &Path) -> Result<Outcome, XtaskError> {
        let executable = text(&std::env::current_exe()?);
        check_all(root, |evidence| {
            Plan::real(&executable, evidence, cfg!(windows))
        })
    }
}
