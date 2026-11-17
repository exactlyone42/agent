#[derive(Debug, thiserror::Error)]
pub enum XtaskError {
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the secret scan refused a link that leaves the project: {0}")]
    OutsideLink(String),
    #[error("the secret scan found no files, so it cannot be judged as passed")]
    EmptySnapshot,
    #[error("could not list the files tracked by Git, so the structure cannot be judged as passed")]
    TrackedFilesUnavailable,
    #[error(
        "the full check is already running; continue the running process instead of starting another"
    )]
    AlreadyRunning,
    #[error("a report field is missing or has the wrong type: {0}")]
    Report(&'static str),
}
