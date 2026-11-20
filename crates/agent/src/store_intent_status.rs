use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum IntentStatus {
    Prepared,
    Denied,
    Dispatched,
    Succeeded,
    Failed,
    Cancelled,
    OutcomeUnknown,
}

impl IntentStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Denied => "denied",
            Self::Dispatched => "dispatched",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::OutcomeUnknown => "outcome_unknown",
        }
    }
}

impl ToSql for IntentStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}
