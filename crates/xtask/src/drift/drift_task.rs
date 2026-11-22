use crate::drift::{
    civil_date::civil_date,
    claude_reviewer::claude_reviewer,
    codex_reviewer::codex_reviewer,
    reviewers::Reviewers,
    run_drift::{DRIFT_DIRECTORY, run_drift},
    select_files::MAX_FILES_PER_RUN,
};
use crate::prelude::*;
use std::fs;
use std::path::Path;

pub fn drift_task(root: &Path) -> Result<Outcome, XtaskError> {
    let now = now_seconds() as u64;
    let logs = root
        .join(DRIFT_DIRECTORY)
        .join(format!("logs-{}", civil_date(now)));
    fs::create_dir_all(&logs)?;
    let mut reviewers = Reviewers {
        claude: claude_reviewer(root, &logs),
        codex: codex_reviewer(root, &logs),
    };
    run_drift(root, now, MAX_FILES_PER_RUN, &mut reviewers)
}
