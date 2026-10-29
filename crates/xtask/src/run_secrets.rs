use crate::copy_snapshot::copy_snapshot;
use crate::error::XtaskError;
use crate::run_step::run_step;
use crate::secret_command::secret_command;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run_secrets(root: &Path, evidence: &Path) -> Result<i32, XtaskError> {
    let snapshot = evidence.join("snapshot");
    fs::create_dir_all(&snapshot)?;
    let listing = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .output()?;
    let names: Vec<String> = String::from_utf8_lossy(&listing.stdout)
        .split('\0')
        .map(str::to_string)
        .collect();
    let code = match copy_snapshot(root, &snapshot, &names) {
        Ok(count) => {
            fs::write(
                evidence.join("snapshot.json"),
                json!({"files": count}).to_string(),
            )?;
            run_step("secrets", &secret_command(evidence)?, &snapshot, evidence)?
        }
        Err(error) => {
            fs::write(evidence.join("snapshot-error.txt"), error.to_string())?;
            127
        }
    };
    fs::remove_dir_all(&snapshot)?;
    Ok(code)
}
