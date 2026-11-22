use crate::drift::{
    candidate::Candidate, candidate_kind::CandidateKind, newest_under::newest_under,
    referenced_paths::referenced_paths,
};
use std::collections::BTreeMap;

pub const DOC_BEHIND_SECONDS: u64 = 30 * 86_400;
const CODE_ROOTS: [&str; 4] = ["crates/", "sql/", "scripts/", ".cargo/"];
const SECONDS_PER_DAY: u64 = 86_400;

pub fn doc_behind(
    path: &str,
    text: &str,
    files: &BTreeMap<String, Vec<u8>>,
    times: &BTreeMap<String, u64>,
) -> Option<Candidate> {
    let doc_time = *times.get(path)?;
    let (line, newest, newest_time) = referenced_paths(path, text)
        .into_iter()
        .filter(|(_, target)| CODE_ROOTS.iter().any(|root| target.starts_with(root)))
        .filter_map(|(line, target)| {
            newest_under(files, times, &target).map(|(name, seconds)| (line, name, seconds))
        })
        .max_by_key(|(_, _, seconds)| *seconds)?;
    let gap = newest_time.checked_sub(doc_time)?;
    (gap > DOC_BEHIND_SECONDS).then(|| Candidate {
        path: path.to_string(),
        line,
        kind: CandidateKind::DocBehindCode,
        subject: format!(
            "{newest} changed {} days after this file",
            gap / SECONDS_PER_DAY
        ),
    })
}
