use crate::prelude::*;
use crate::{
    evidence::copy_snapshot::copy_snapshot, evidence::evidence_files::SNAPSHOT_DIRECTORY,
    evidence::evidence_files::SNAPSHOT_ERROR_FILE, evidence::evidence_files::SNAPSHOT_FILE,
    evidence::record_keys::FILES_KEY, run::secret_command::secret_command,
    run::tool_arguments::CACHED_FLAG, run::tool_arguments::EXCLUDE_STANDARD_FLAG,
    run::tool_arguments::OTHERS_FLAG,
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
