use agent_platform_agent::{store::StoreError, task::TaskError};
use serde_json::json;

pub const ERROR_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub struct CliError {
    code: &'static str,
    message: String,
}

impl CliError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn noting(mut self, note: &str) -> Self {
        self.message.push_str(note);
        self
    }

    pub fn line(&self) -> String {
        json!({
            "schema_version": ERROR_SCHEMA_VERSION,
            "error_code": self.code,
            "message": self.message,
        })
        .to_string()
    }
}

impl From<StoreError> for CliError {
    fn from(error: StoreError) -> Self {
        let code = match error {
            StoreError::Database(_) => "store_database",
            StoreError::Incompatible => "store_incompatible",
            StoreError::NotFound => "task_not_found",
            StoreError::Conflict => "task_conflict",
            StoreError::Unsettled => "resume_unsettled",
            StoreError::Session(_) => "session_unavailable",
            StoreError::InvalidCursor => "invalid_cursor",
            StoreError::InvalidSpec => "invalid_task_spec",
            StoreError::InvalidToolCall => "invalid_tool_call",
            StoreError::InvalidInstruction => "invalid_instruction",
            StoreError::InstructionBacklog => "instruction_backlog",
            StoreError::Process(_) => "cancel_unconfirmed",
        };
        Self::new(code, error.to_string())
    }
}

impl From<TaskError> for CliError {
    fn from(error: TaskError) -> Self {
        let code = match error {
            TaskError::ApprovalDenied => "approval_denied",
            TaskError::Cancelled { .. } => "cancelled",
            TaskError::InvalidTask => "invalid_task",
            TaskError::InvalidInput => "invalid_input",
            TaskError::Input(_) => "input_unreadable",
            TaskError::Harness(_) => "model_run_failed",
            TaskError::Timeout => "task_timeout",
            TaskError::State => "tool_state_error",
            TaskError::MissingArtifact => "artifact_missing",
            TaskError::IncompleteArtifact => "artifact_incomplete",
            TaskError::Verification(_) => "verification_failed",
        };
        Self::new(code, error.to_string())
    }
}
