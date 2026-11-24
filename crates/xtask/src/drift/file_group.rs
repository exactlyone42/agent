use crate::drift::candidate::Candidate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileGroup {
    pub path: String,
    pub candidates: Vec<Candidate>,
    pub hash: String,
}
