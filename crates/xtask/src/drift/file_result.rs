use crate::drift::{finding::Finding, ruling::Ruling, second_opinion::SecondOpinion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileResult {
    pub path: String,
    pub ruling: Ruling,
    pub finding: Finding,
    pub opinion: Option<SecondOpinion>,
}
