use crate::prelude::*;
use crate::{
    drift::drift_scan_task::drift_scan_task,
    drift::drift_task::drift_task,
    run::check_all::check_all,
    run::gate::gate_steps,
    run::run_steps::run_steps,
    run::step_names::{ALL_TASK, DRIFT_SCAN_TASK, DRIFT_TASK},
    scan::run_structure::run_structure,
};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task {
    Gate,
    All,
    Drift,
    DriftScan,
}

impl Task {
    pub fn parse(name: Option<&str>) -> Option<Self> {
        match name {
            Some(GATE_TASK) => Some(Self::Gate),
            Some(ALL_TASK) => Some(Self::All),
            Some(DRIFT_TASK) => Some(Self::Drift),
            Some(DRIFT_SCAN_TASK) => Some(Self::DriftScan),
            _ => None,
        }
    }

    pub fn run(self, root: &Path) -> Result<Outcome, XtaskError> {
        match self {
            Self::Gate => Self::gate(root),
            Self::All => Self::all(root),
            Self::Drift => drift_task(root),
            Self::DriftScan => drift_scan_task(root),
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
