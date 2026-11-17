use rusqlite::{ToSql, types::ToSqlOutput};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum DecisionSource {
    TaskPolicy,
    LocalCli,
    Pending,
}

impl DecisionSource {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::TaskPolicy => "task_policy",
            Self::LocalCli => "local_cli",
            Self::Pending => "pending",
        }
    }
}

impl ToSql for DecisionSource {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}
