use crate::drift::{
    candidate::Candidate, dead_references::dead_references, stale_age::stale_by_age,
};
use std::collections::BTreeMap;

pub fn scan_candidates(
    files: &BTreeMap<String, Vec<u8>>,
    times: &BTreeMap<String, u64>,
) -> Vec<Candidate> {
    let mut candidates = dead_references(files);
    candidates.extend(stale_by_age(files, times));
    candidates.sort();
    candidates
}
