use crate::ModelError;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("the run was asked to stop")]
    Cancelled,
    #[error("the turn limit was exceeded")]
    TurnLimit,
    #[error("the model failed: {0}")]
    Model(#[from] ModelError),
    #[error("the run was stopped by a hook: {0}")]
    Stopped(String),
    #[error("no capability proposed a next step and there is no answer")]
    Stalled,
}
