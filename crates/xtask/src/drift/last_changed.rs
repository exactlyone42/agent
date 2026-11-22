use crate::drift::parse_log::parse_log;
use crate::prelude::*;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

const LOG_COMMAND: &str = "log";
const NO_QUOTE_ARGUMENTS: [&str; 2] = ["-c", "core.quotepath=off"];
const LOG_ARGUMENTS: [&str; 3] = ["--name-only", "--no-renames", "--format=@%ct"];

pub fn last_changed(root: &Path) -> Result<BTreeMap<String, u64>, XtaskError> {
    let output = Command::new(GIT)
        .args(NO_QUOTE_ARGUMENTS)
        .arg(LOG_COMMAND)
        .args(LOG_ARGUMENTS)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(XtaskError::GitHistoryUnavailable);
    }
    Ok(parse_log(&String::from_utf8_lossy(&output.stdout)))
}
