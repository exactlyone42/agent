use crate::drift::{
    candidate::Candidate, candidate_kind::CandidateKind, newest_under::newest_under,
};
use std::collections::BTreeMap;

pub const DIAGRAM_BEHIND_SECONDS: u64 = 14 * 86_400;
pub const CURRENT_DIAGRAMS: [&str; 2] = [
    "docs/diagrams/current-class-diagram.pdf",
    "docs/diagrams/mortal-agent-architecture.pdf",
];
const SOURCE_ROOT: &str = "crates";
const SECONDS_PER_DAY: u64 = 86_400;

pub fn diagram_behind(
    diagram: &str,
    files: &BTreeMap<String, Vec<u8>>,
    times: &BTreeMap<String, u64>,
) -> Option<Candidate> {
    let diagram_time = *times.get(diagram)?;
    let (newest, newest_time) = newest_under(files, times, SOURCE_ROOT)?;
    let gap = newest_time.checked_sub(diagram_time)?;
    (gap > DIAGRAM_BEHIND_SECONDS).then(|| Candidate {
        path: diagram.to_string(),
        line: 0,
        kind: CandidateKind::DiagramBehindCode,
        subject: format!(
            "{newest} changed {} days after this diagram",
            gap / SECONDS_PER_DAY
        ),
    })
}
