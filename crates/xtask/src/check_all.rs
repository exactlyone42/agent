use crate::claim_run::claim_run;
use crate::error::XtaskError;
use crate::now_seconds::now_seconds;
use crate::plan::Plan;
use crate::run_checks::run_checks;
use std::fs::{self, OpenOptions};
use std::path::Path;

pub fn check_all(root: &Path, plan: impl FnOnce(&Path) -> Plan) -> Result<i32, XtaskError> {
    let lock = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join(".git/agent-platform-check.lock"))?;
    lock.try_lock().map_err(|_| XtaskError::AlreadyRunning)?;
    let evidence = root.join(".runtime/checks").join(format!(
        "{}-{}",
        now_seconds() as u64,
        std::process::id()
    ));
    claim_run(&root.join(".runtime/checks/active.json"), || {
        fs::create_dir_all(&evidence)?;
        run_checks(root, &evidence, &plan(&evidence))
    })
}
