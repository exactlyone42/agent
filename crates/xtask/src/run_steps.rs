use crate::error::XtaskError;
use crate::step::Step;
use std::path::Path;

pub fn run_steps(root: &Path, steps: &[Step]) -> Result<i32, XtaskError> {
    for step in steps {
        println!("執行 {} {}", step.program, step.arguments.join(" "));
        let code = step.command(root).status()?.code().unwrap_or(1);
        if code != 0 {
            return Ok(code);
        }
    }
    Ok(0)
}
