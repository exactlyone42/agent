use crate::drift::{candidate::Candidate, content_hash::content_hash, file_group::FileGroup};
use std::collections::BTreeMap;

pub fn group_candidates(
    candidates: &[Candidate],
    files: &BTreeMap<String, Vec<u8>>,
) -> Vec<FileGroup> {
    let mut by_path: BTreeMap<&str, Vec<Candidate>> = BTreeMap::new();
    for candidate in candidates.iter().filter(|item| item.kind.needs_ai()) {
        by_path
            .entry(candidate.path.as_str())
            .or_default()
            .push(candidate.clone());
    }
    by_path
        .into_iter()
        .map(|(path, group)| FileGroup {
            path: path.to_string(),
            hash: content_hash(files.get(path).map_or(&[][..], Vec::as_slice), &group),
            candidates: group,
        })
        .collect()
}
