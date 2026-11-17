use agent_platform_agent::task::ArtifactState;
use serde::Serialize;

#[derive(Serialize)]
pub struct Decision<'a> {
    pub approval_id: &'a str,
    pub status: &'static str,
}

#[derive(Serialize)]
pub struct Cancelled<'a> {
    pub artifact_created: bool,
    pub state: &'a str,
    pub task_id: &'a str,
}

#[derive(Serialize)]
pub struct Finished<'a> {
    pub artifact: &'a ArtifactState,
    pub assistant_message: &'a str,
    pub task_id: &'a str,
}
