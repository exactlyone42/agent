use crate::prelude::*;
use crate::{
    evidence::claim_run::claim_run, evidence::evidence_files::ACTIVE_MARKER,
    evidence::evidence_files::CHECKS_DIRECTORY, evidence::evidence_files::LOCK_FILE,
    run::run_checks::run_checks,
};

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
