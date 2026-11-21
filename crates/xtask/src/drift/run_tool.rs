use crate::drift::{exit_result::exit_result, tool_call::ToolCall, tool_failure::tool_failure};
use crate::prelude::*;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(500);

pub fn run_tool(call: &ToolCall) -> Result<(), XtaskError> {
    let program = call.program;
    let mut child = Command::new(program)
        .args(&call.arguments)
        .current_dir(call.directory)
        .stdin(Stdio::piped())
        .stdout(File::create(call.output)?)
        .stderr(File::create(call.errors)?)
        .spawn()
        .map_err(|error| tool_failure(program, error))?;
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(call.input.as_bytes())
            .map_err(|error| tool_failure(program, error))?;
    }
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return exit_result(program, status);
        }
        if started.elapsed() > call.timeout {
            child.kill()?;
            child.wait()?;
            return Err(tool_failure(program, "timed out"));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}
