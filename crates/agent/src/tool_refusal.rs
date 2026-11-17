use agent_harness::ToolError;

#[derive(Debug, thiserror::Error)]
pub enum ToolRefusal {
    #[error("the task was asked to stop, so reading is cancelled")]
    ReadCancelled,
    #[error("the source state is invalid")]
    SourceState,
    #[error("the user did not authorize reading the source")]
    ReadNotAuthorized,
    #[error("the source exceeds the size limit")]
    SourceTooLarge,
    #[error("the arguments do not match the schema")]
    InvalidArguments,
    #[error("the task was asked to stop, so writing is cancelled")]
    WriteCancelled,
    #[error("the user did not authorize writing the artifact")]
    WriteNotAuthorized,
    #[error("the artifact exceeds the size limit")]
    ArtifactTooLarge,
    #[error("the artifact state is invalid")]
    ArtifactState,
    #[error("the artifact was already written and is not written again")]
    ArtifactExists,
}

impl From<ToolRefusal> for ToolError {
    fn from(refusal: ToolRefusal) -> Self {
        Self::refused(&refusal.to_string())
    }
}
