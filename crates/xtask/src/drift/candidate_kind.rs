#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CandidateKind {
    DeadReference,
    DocBehindCode,
    DiagramBehindCode,
}

impl CandidateKind {
    pub fn describe(self) -> &'static str {
        match self {
            Self::DeadReference => "the file mentions a path that is not tracked in the repository",
            Self::DocBehindCode => "the file was last changed long before code it refers to",
            Self::DiagramBehindCode => "the diagram was last changed long before the code",
        }
    }

    pub fn needs_ai(self) -> bool {
        self != Self::DiagramBehindCode
    }
}
