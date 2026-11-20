use crate::tool_refusal_messages::{
    ARTIFACT_EXISTS, ARTIFACT_STATE, ARTIFACT_TOO_LARGE, INVALID_ARGUMENTS, READ_CANCELLED,
    READ_NOT_AUTHORIZED, SOURCE_STATE, SOURCE_TOO_LARGE, WRITE_CANCELLED, WRITE_NOT_AUTHORIZED,
};
use agent_harness::ToolError;
use std::fmt;

#[derive(Debug)]
pub enum ToolRefusal {
    ReadCancelled,
    SourceState,
    ReadNotAuthorized,
    SourceTooLarge,
    InvalidArguments,
    WriteCancelled,
    WriteNotAuthorized,
    ArtifactTooLarge,
    ArtifactState,
    ArtifactExists,
}

impl ToolRefusal {
    fn message(&self) -> &'static str {
        match self {
            Self::ReadCancelled => READ_CANCELLED,
            Self::SourceState => SOURCE_STATE,
            Self::ReadNotAuthorized => READ_NOT_AUTHORIZED,
            Self::SourceTooLarge => SOURCE_TOO_LARGE,
            Self::InvalidArguments => INVALID_ARGUMENTS,
            Self::WriteCancelled => WRITE_CANCELLED,
            Self::WriteNotAuthorized => WRITE_NOT_AUTHORIZED,
            Self::ArtifactTooLarge => ARTIFACT_TOO_LARGE,
            Self::ArtifactState => ARTIFACT_STATE,
            Self::ArtifactExists => ARTIFACT_EXISTS,
        }
    }
}

impl fmt::Display for ToolRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for ToolRefusal {}

impl From<ToolRefusal> for ToolError {
    fn from(refusal: ToolRefusal) -> Self {
        Self::refused(&refusal.to_string())
    }
}
