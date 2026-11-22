use crate::prelude::*;
use std::path::Path;

pub fn run_steps(root: &Path, steps: &[Step]) -> Result<Outcome, XtaskError> {
    for step in steps {
        println!("running {} {}", step.program, step.arguments.join(" "));
        if !step.command(root).status()?.success() {
            return Ok(Outcome::Failed);
        }
    }
    Ok(Outcome::Passed)
}
