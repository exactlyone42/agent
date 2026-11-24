use crate::drift::tool_failure::tool_failure;
use crate::prelude::*;
use std::process::ExitStatus;

pub(crate) fn exit_result(program: &str, status: ExitStatus) -> Result<(), XtaskError> {
    if status.success() {
        return Ok(());
    }
    Err(tool_failure(program, format!("exited with {status}")))
}
