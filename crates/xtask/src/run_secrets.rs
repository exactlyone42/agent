use crate::prelude::*;
use crate::{
    copy_snapshot::copy_snapshot, evidence_files::SNAPSHOT_DIRECTORY,
    evidence_files::SNAPSHOT_ERROR_FILE, evidence_files::SNAPSHOT_FILE, record_keys::FILES_KEY,
    secret_command::secret_command, tool_arguments::CACHED_FLAG,
    tool_arguments::EXCLUDE_STANDARD_FLAG, tool_arguments::OTHERS_FLAG,
};
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run_secrets(root: &Path, evidence: &Path) -> Result<i32, XtaskError> {
    let snapshot = evidence.join(SNAPSHOT_DIRECTORY);
    fs::create_dir_all(&snapshot)?;
    let listing = Command::new(GIT)
        .args([
            LIST_FILES,
            NUL_SEPARATED_FLAG,
            CACHED_FLAG,
            OTHERS_FLAG,
            EXCLUDE_STANDARD_FLAG,
        ])
        .current_dir(root)
        .output()?;
    let names: Vec<String> = String::from_utf8_lossy(&listing.stdout)
        .split(NUL)
        .map(str::to_string)
        .collect();
    let code = match copy_snapshot(root, &snapshot, &names) {
        Ok(count) => {
            fs::write(
                evidence.join(SNAPSHOT_FILE),
                json!({FILES_KEY: count}).to_string(),
            )?;
            run_step(
                SECRETS_STEP,
                &secret_command(evidence)?,
                &snapshot,
                evidence,
            )?
        }
        Err(error) => {
            fs::write(evidence.join(SNAPSHOT_ERROR_FILE), error.to_string())?;
            COMMAND_UNAVAILABLE_CODE
        }
    };
    fs::remove_dir_all(&snapshot)?;
    Ok(code)
}
