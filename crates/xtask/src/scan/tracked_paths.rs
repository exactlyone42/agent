use crate::prelude::*;
use std::path::Path;
use std::process::Command;

pub fn tracked_paths(root: &Path) -> Result<Vec<String>, XtaskError> {
    let listing = Command::new(GIT)
        .args([LIST_FILES, NUL_SEPARATED_FLAG])
        .current_dir(root)
        .output()?;
    if !listing.status.success() || listing.stdout.is_empty() {
        return Err(XtaskError::TrackedFilesUnavailable);
    }
    Ok(String::from_utf8_lossy(&listing.stdout)
        .split(NUL)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect())
}
