use crate::drift::scan_repo::scan_repo;
use crate::prelude::*;
use std::path::Path;

pub fn drift_scan_task(root: &Path) -> Result<Outcome, XtaskError> {
    let scan = scan_repo(root)?;
    for candidate in &scan.candidates {
        println!(
            "{}:{} {:?} {}",
            candidate.path, candidate.line, candidate.kind, candidate.subject
        );
    }
    println!("{} candidates", scan.candidates.len());
    Ok(Outcome::Passed)
}
