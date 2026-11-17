use crate::prelude::*;
use crate::{
    claim_run::claim_run, evidence_files::ACTIVE_MARKER, evidence_files::CHECKS_DIRECTORY,
    evidence_files::LOCK_FILE, run_checks::run_checks,
};
use std::fs::{self, OpenOptions};
use std::path::Path;

pub fn check_all(root: &Path, plan: impl FnOnce(&Path) -> Plan) -> Result<Outcome, XtaskError> {
    let lock = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join(LOCK_FILE))?;
    lock.try_lock().map_err(|_| XtaskError::AlreadyRunning)?;
    let evidence = root.join(CHECKS_DIRECTORY).join(format!(
        "{}-{}",
        now_seconds() as u64,
        std::process::id()
    ));
    claim_run(&root.join(CHECKS_DIRECTORY).join(ACTIVE_MARKER), || {
        fs::create_dir_all(&evidence)?;
        run_checks(root, &evidence, &plan(&evidence))
    })
}
