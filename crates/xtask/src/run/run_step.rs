use crate::prelude::*;
use crate::{
    evidence::evidence_files::LOG_EXTENSION, evidence::evidence_files::RECORD_EXTENSION,
    evidence::record_keys::COMMAND_KEY, evidence::record_keys::EXIT_CODE_KEY,
    evidence::record_keys::FINISHED_KEY,
};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run_step(
    name: &str,
    command: &[String],
    root: &Path,
    evidence: &Path,
) -> Result<i32, XtaskError> {
    println!("running {name}");
    let started = now_seconds();
    let mut log = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(evidence.join(format!("{name}.{LOG_EXTENSION}")))?;
    let exit_code = match command.split_first() {
        Some((program, arguments)) => {
            match Command::new(program)
                .args(arguments)
                .current_dir(root)
                .env_remove(CARGO_PKG_NAME_VARIABLE)
                .stdout(Stdio::from(log.try_clone()?))
                .stderr(Stdio::from(log.try_clone()?))
                .status()
            {
                Ok(status) => status.code().unwrap_or(FAILURE_CODE),
                Err(error) => {
                    writeln!(log, "{:?}: {error}", error.kind())?;
                    COMMAND_UNAVAILABLE_CODE
                }
            }
        }
        None => COMMAND_UNAVAILABLE_CODE,
    };
    let record = serde_json::json!({
        COMMAND_KEY: command,
        STARTED_KEY: started,
        FINISHED_KEY: now_seconds(),
        EXIT_CODE_KEY: exit_code,
    });
    fs::write(
        evidence.join(format!("{name}.{RECORD_EXTENSION}")),
        serde_json::to_string_pretty(&record)?,
    )?;
    Ok(exit_code)
}
