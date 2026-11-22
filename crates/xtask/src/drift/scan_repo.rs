use crate::drift::{
    candidate::Candidate, last_changed::last_changed, scan_candidates::scan_candidates,
};
use crate::prelude::*;
use crate::scan::{read_tracked::read_tracked, tracked_paths::tracked_paths};
use std::collections::BTreeMap;
use std::path::Path;

pub struct RepoScan {
    pub files: BTreeMap<String, Vec<u8>>,
    pub candidates: Vec<Candidate>,
}

pub fn scan_repo(root: &Path) -> Result<RepoScan, XtaskError> {
    let paths = tracked_paths(root)?;
    let files = read_tracked(root, &paths)?;
    let times = last_changed(root)?;
    let candidates = scan_candidates(&files, &times);
    Ok(RepoScan { files, candidates })
}
