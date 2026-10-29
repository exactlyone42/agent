use crate::error::XtaskError;
use crate::now_seconds::now_seconds;
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
    println!("執行 {name}");
    let started = now_seconds();
    let mut log = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(evidence.join(format!("{name}.txt")))?;
    let exit_code = match command.split_first() {
        Some((program, arguments)) => {
            match Command::new(program)
                .args(arguments)
                .current_dir(root)
                .env_remove("CARGO_PKG_NAME")
                .stdout(Stdio::from(log.try_clone()?))
                .stderr(Stdio::from(log.try_clone()?))
                .status()
            {
                Ok(status) => status.code().unwrap_or(1),
                Err(error) => {
                    writeln!(log, "{:?}: {error}", error.kind())?;
                    127
                }
            }
        }
        None => 127,
    };
    let record = serde_json::json!({
        "command": command,
        "started": started,
        "finished": now_seconds(),
        "exit_code": exit_code,
    });
    fs::write(
        evidence.join(format!("{name}.json")),
        serde_json::to_string_pretty(&record)?,
    )?;
    Ok(exit_code)
}
