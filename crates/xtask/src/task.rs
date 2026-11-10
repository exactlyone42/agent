use crate::check_all::check_all;
use crate::error::XtaskError;
use crate::gate::gate_steps;
use crate::plan::Plan;
use crate::run_steps::run_steps;
use crate::run_structure::run_structure;
use crate::text::text;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task {
    Gate,
    All,
}

impl Task {
    pub fn parse(name: Option<&str>) -> Option<Self> {
        match name {
            Some("gate") => Some(Self::Gate),
            Some("all") => Some(Self::All),
            _ => None,
        }
    }

    pub fn run(self, root: &Path) -> Result<i32, XtaskError> {
        match self {
            Self::Gate => Self::gate(root),
            Self::All => Self::all(root),
        }
    }

    fn gate(root: &Path) -> Result<i32, XtaskError> {
        let code = run_structure(root)?;
        if code != 0 {
            return Ok(code);
        }
        run_steps(root, &gate_steps(root))
    }

    fn all(root: &Path) -> Result<i32, XtaskError> {
        let executable = text(&std::env::current_exe()?);
        check_all(root, |evidence| {
            Plan::real(&executable, evidence, cfg!(windows))
        })
    }
}
