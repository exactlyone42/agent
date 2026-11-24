use crate::drift::candidate_kind::CandidateKind;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Candidate {
    pub path: String,
    pub line: usize,
    pub kind: CandidateKind,
    pub subject: String,
}
