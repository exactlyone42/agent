use rusqlite::{ToSql, types::ToSqlOutput};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum EventKind {
    Started,
    ApprovalRequested,
    ApprovalGranted,
    ApprovalDenied,
    ApprovalExpired,
    ApprovalRevoked,
    ToolDispatched,
    ToolSucceeded,
    ToolFailed,
    ToolRejected,
    ToolOutcomeUnknown,
}

impl EventKind {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::ApprovalRequested => "approval_requested",
            Self::ApprovalGranted => "approval_granted",
            Self::ApprovalDenied => "approval_denied",
            Self::ApprovalExpired => "approval_expired",
            Self::ApprovalRevoked => "approval_revoked",
            Self::ToolDispatched => "tool_dispatched",
            Self::ToolSucceeded => "tool_succeeded",
            Self::ToolFailed => "tool_failed",
            Self::ToolRejected => "tool_rejected",
            Self::ToolOutcomeUnknown => "tool_outcome_unknown",
        }
    }
}

impl ToSql for EventKind {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}
