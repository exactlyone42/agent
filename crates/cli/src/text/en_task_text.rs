use crate::prelude::*;

pub fn en_task_text(error: &TaskError) -> &'static str {
    match error {
        TaskError::ApprovalDenied => {
            "The tool was not approved or the approval is no longer valid; check the approval records"
        }
        TaskError::Cancelled { .. } => {
            "The task was stopped; if an artifact was created, check it before continuing"
        }
        TaskError::InvalidTask => "The task content is outside the length limits",
        TaskError::InvalidInput => "The source must be a regular file within the size limit",
        TaskError::Input(_) => "Could not open the authorized source file",
        TaskError::Harness(_) => {
            "The model run failed; there may be a partial artifact, so check it before continuing"
        }
        TaskError::Timeout => {
            "The task timed out; there may be a partial artifact, so check it before continuing"
        }
        TaskError::State => "The tool state is invalid, so the artifact cannot be confirmed",
        TaskError::MissingArtifact => {
            "The model finished, but there is no artifact that can be verified"
        }
        TaskError::IncompleteArtifact => {
            "The artifact was not completely written or saved; check the existing file"
        }
        TaskError::Verification(_) => "The artifact failed verification",
    }
}
