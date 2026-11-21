use crate::prelude::*;
use crate::{
    scan::duplicate_lines::duplicate_lines, scan::empty_files::empty_files,
    scan::locale_literals::locale_literals, scan::read_tracked::read_tracked,
    scan::structure_violations::structure_violations, scan::tracked_paths::tracked_paths,
    scan::unreferenced_docs::unreferenced_docs,
};
use std::path::Path;

pub fn run_structure(root: &Path) -> Result<Outcome, XtaskError> {
    println!("checking the directory structure");
    let paths = tracked_paths(root)?;
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
