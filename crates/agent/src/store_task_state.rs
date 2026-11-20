use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TaskState {
    Running,
    WaitingApproval,
    Succeeded,
    NeedsReview,
    Failed,
    Cancelled,
    OutcomeUnknown,
}

impl TaskState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::WaitingApproval => "waiting_approval",
            Self::Succeeded => "succeeded",
            Self::NeedsReview => "needs_review",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::OutcomeUnknown => "outcome_unknown",
        }
    }
}

impl ToSql for TaskState {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}
