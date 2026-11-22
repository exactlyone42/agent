#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ruling {
    Confirmed,
    Disputed,
    NoIssue,
    Rejected,
}
