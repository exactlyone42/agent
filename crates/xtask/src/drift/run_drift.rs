use crate::drift::{
    civil_date::civil_date, group_candidates::group_candidates, render_report::render_report,
    review_batch::review_batch, review_state::ReviewState, reviewers::Reviewers, run_lock::RunLock,
    scan_repo::scan_repo, select_files::select_files,
};
use crate::prelude::*;
use std::fs;
use std::path::Path;

pub const DRIFT_DIRECTORY: &str = ".runtime/drift";
const LOCK_FILE: &str = "running.lock";
const STATE_FILE: &str = "state.json";

pub fn run_drift(
    root: &Path,
    now: u64,
    limit: usize,
    reviewers: &mut Reviewers,
) -> Result<Outcome, XtaskError> {
    let directory = root.join(DRIFT_DIRECTORY);
    let _lock = RunLock::acquire(&directory.join(LOCK_FILE))?;
    let scan = scan_repo(root)?;
    let state_path = directory.join(STATE_FILE);
    let mut state = ReviewState::load(&state_path)?;
    let groups = group_candidates(&scan.candidates, &scan.files);
    let selection = select_files(groups, &state, now, limit);
    let batch = review_batch(&selection.chosen, &scan.files, reviewers, &mut state, now)?;
    state.save(&state_path)?;
    let report = render_report(&scan.candidates, &selection, &batch, now);
    let report_path = directory.join(format!("report-{}.md", civil_date(now)));
    fs::write(&report_path, report)?;
    println!("report: {}", text(&report_path));
    Ok(Outcome::judge(batch.failure.is_none()))
}
