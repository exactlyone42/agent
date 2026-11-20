use crate::prelude::*;
use crate::{
    scan::duplicate_lines::duplicate_lines, scan::empty_files::empty_files,
    scan::locale_literals::locale_literals, scan::read_tracked::read_tracked,
    scan::structure_violations::structure_violations, scan::unreferenced_docs::unreferenced_docs,
};
use std::path::Path;
use std::process::Command;

pub fn run_structure(root: &Path) -> Result<Outcome, XtaskError> {
    println!("checking the directory structure");
    let listing = Command::new(GIT)
        .args([LIST_FILES, NUL_SEPARATED_FLAG])
        .current_dir(root)
        .output()?;
    if !listing.status.success() || listing.stdout.is_empty() {
        return Err(XtaskError::TrackedFilesUnavailable);
    }
    let paths: Vec<String> = String::from_utf8_lossy(&listing.stdout)
        .split(NUL)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect();
    let files = read_tracked(root, &paths)?;
    let mut violations = structure_violations(&paths);
    violations.extend(empty_files(&files));
    violations.extend(locale_literals(&files));
    violations.extend(unreferenced_docs(&files));
    violations.extend(duplicate_lines(&files));
    for violation in &violations {
        eprintln!("{violation}");
    }
    Ok(Outcome::judge(violations.is_empty()))
}
