use crate::drift::{
    candidate::Candidate, candidate_kind::CandidateKind, path_exists::path_exists,
    referenced_paths::referenced_paths,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn missing_in_doc(
    path: &str,
    bytes: &[u8],
    files: &BTreeMap<String, Vec<u8>>,
) -> Vec<Candidate> {
    let text = String::from_utf8_lossy(bytes);
    let mut seen = BTreeSet::new();
    referenced_paths(path, &text)
        .into_iter()
        .filter(|(_, target)| !path_exists(files, target))
        .filter(|(_, target)| seen.insert(target.clone()))
        .map(|(line, target)| Candidate {
            path: path.to_string(),
            line,
            kind: CandidateKind::DeadReference,
            subject: target,
        })
        .collect()
}
