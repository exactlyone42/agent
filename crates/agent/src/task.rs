use crate::file_check::verification::{VerificationError, verify_file};
use crate::runtime::agent_runtime::AgentRuntime;
use crate::task_settings::TASK_MAX_TURNS;
pub use crate::task_settings::{
    FILE_LIMIT, MANUAL_TASK_TIMEOUT, READ_INPUT_DESCRIPTION, READ_INPUT_TOOL, TASK_TIMEOUT,
    WRITE_OUTPUT_DESCRIPTION, WRITE_OUTPUT_DONE, WRITE_OUTPUT_TOOL,
};
use crate::tool_refusal::ToolRefusal;
pub use agent_harness::CancellationToken;
use agent_harness::{
    AgentId, AgentIdentity, Hook, Model, MortalAgent, RunError, Tool, ToolError, ToolFuture,
    ToolSpec,
};
use agent_harness_capabilities::{Loop, ToolCalling};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileTask {
    pub prompt: String,
    pub input: PathBuf,
    pub output: PathBuf,
    pub expected: Option<String>,
    #[serde(default)]
    pub allow_read: bool,
    #[serde(default)]
    pub allow_write: bool,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactState {
    Verified,
    AwaitingReview,
}

#[derive(Debug, Serialize)]
pub struct TaskReport {
    pub artifact: ArtifactState,
    pub assistant_message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("the tool was not approved or the approval is no longer valid")]
    ApprovalDenied,
    #[error("the task was stopped")]
    Cancelled { artifact_created: bool },
    #[error("the task content is outside the length limits")]
    InvalidTask,
    #[error("the source must be a regular file within the size limit")]
    InvalidInput,
    #[error("could not open the authorized source file")]
    Input(#[source] std::io::Error),
    #[error("the model run failed")]
    Harness(#[source] RunError),
    #[error("the task timed out")]
    Timeout,
    #[error("the tool state is invalid, so the artifact cannot be confirmed")]
    State,
    #[error("the model finished but there is no artifact to verify")]
    MissingArtifact,
    #[error("the artifact was not completely written or saved")]
    IncompleteArtifact,
    #[error("the artifact failed verification")]
    Verification(#[source] VerificationError),
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadArgs {}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WriteArgs {
    pub(crate) content: String,
}

struct ReadInput(Arc<Mutex<Option<File>>>, CancellationToken);
struct Artifact {
    file: File,
    complete: bool,
}
struct WriteOutput {
    path: PathBuf,
    allowed: bool,
    artifact: Arc<Mutex<Option<Artifact>>>,
    cancellation: CancellationToken,
}

impl ReadInput {
    fn read(&self) -> Result<String, ToolError> {
        if self.1.is_cancelled() {
            return Err(ToolRefusal::ReadCancelled.into());
        }
        let mut guard = self.0.lock().map_err(|_| ToolRefusal::SourceState)?;
        let file = guard.as_mut().ok_or(ToolRefusal::ReadNotAuthorized)?;
        file.seek(SeekFrom::Start(0)).map_err(ToolError::failed)?;
        let mut content = String::new();
        file.take(FILE_LIMIT + 1)
            .read_to_string(&mut content)
            .map_err(ToolError::failed)?;
        if content.len() as u64 > FILE_LIMIT {
            return Err(ToolRefusal::SourceTooLarge.into());
        }
        Ok(content)
    }
}

impl Tool for ReadInput {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: READ_INPUT_TOOL.into(),
            description: READ_INPUT_DESCRIPTION.into(),
            parameters: json!({"type":"object","properties":{},"additionalProperties":false}),
        }
    }

    fn call(&self, args: serde_json::Value) -> ToolFuture<'_> {
        Box::pin(async move {
            serde_json::from_value::<ReadArgs>(args).map_err(|_| ToolRefusal::InvalidArguments)?;
            self.read()
        })
    }
}

impl WriteOutput {
    fn write(&self, args: WriteArgs) -> Result<String, ToolError> {
        if self.cancellation.is_cancelled() {
            return Err(ToolRefusal::WriteCancelled.into());
        }
        if !self.allowed {
            return Err(ToolRefusal::WriteNotAuthorized.into());
        }
        if args.content.len() as u64 > FILE_LIMIT {
            return Err(ToolRefusal::ArtifactTooLarge.into());
        }
        let mut artifact = self
            .artifact
            .lock()
            .map_err(|_| ToolRefusal::ArtifactState)?;
        if artifact.is_some() {
            return Err(ToolRefusal::ArtifactExists.into());
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&self.path)
            .map_err(ToolError::failed)?;
        let artifact = artifact.insert(Artifact {
            file,
            complete: false,
        });
        artifact
            .file
            .write_all(args.content.as_bytes())
            .map_err(ToolError::failed)?;
        artifact.file.sync_all().map_err(ToolError::failed)?;
        artifact.complete = true;
        Ok(WRITE_OUTPUT_DONE.into())
    }
}

impl Tool for WriteOutput {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: WRITE_OUTPUT_TOOL.into(),
            description: WRITE_OUTPUT_DESCRIPTION.into(),
            parameters: json!({"type":"object","properties":{"content":{"type":"string"}},"required":["content"],"additionalProperties":false}),
        }
    }

    fn call(&self, args: serde_json::Value) -> ToolFuture<'_> {
        Box::pin(async move {
            let args = serde_json::from_value::<WriteArgs>(args)
                .map_err(|_| ToolRefusal::InvalidArguments)?;
            self.write(args)
        })
    }
}

pub async fn run_file_task(
    model: impl Model,
    task: FileTask,
    cancellation: CancellationToken,
    hook: impl Hook,
    timeout: Duration,
) -> Result<TaskReport, TaskError> {
    if cancellation.is_cancelled() {
        return Err(TaskError::Cancelled {
            artifact_created: false,
        });
    }
    if task.prompt.trim().is_empty()
        || task.prompt.len() as u64 > FILE_LIMIT
        || task
            .expected
            .as_ref()
            .is_some_and(|s| s.len() as u64 > FILE_LIMIT)
    {
        return Err(TaskError::InvalidTask);
    }
    let input = if task.allow_read {
        let file =
            crate::file_check::input::open_checked(&task.input, FILE_LIMIT).map_err(|error| {
                match error {
                    crate::file_check::input::InputError::InvalidFile => TaskError::InvalidInput,
                    crate::file_check::input::InputError::Io(error) => TaskError::Input(error),
                }
            })?;
        Some(file)
    } else {
        None
    };
    let artifact = Arc::new(Mutex::new(None));
    let identity = AgentIdentity {
        name: "file-task".to_owned(),
    };
    let mut agent = AgentRuntime::new(
        MortalAgent::new(AgentId(uuid::Uuid::new_v4().to_string()), identity),
        model,
    )
    .hook(hook)
    .tool(ReadInput(Arc::new(Mutex::new(input)), cancellation.clone()))
    .tool(WriteOutput {
        path: task.output,
        allowed: task.allow_write,
        artifact: artifact.clone(),
        cancellation: cancellation.clone(),
    })
    .capability(Loop::new().max_turns(TASK_MAX_TURNS))
    .capability(ToolCalling::new());
    let response = tokio::select! {
        biased;
        _ = cancellation.cancelled() => {
            let artifact_created = artifact.lock().map_err(|_| TaskError::State)?.is_some();
            return Err(TaskError::Cancelled { artifact_created });
        }
        response = tokio::time::timeout(timeout, agent.run(&task.prompt, &cancellation)) => {
            response.map_err(|_| TaskError::Timeout)?
                .map_err(TaskError::Harness)?
        }
    };
    let mut guard = artifact.lock().map_err(|_| TaskError::State)?;
    if cancellation.is_cancelled() {
        return Err(TaskError::Cancelled {
            artifact_created: guard.is_some(),
        });
    }
    let artifact = guard.as_mut().ok_or(TaskError::MissingArtifact)?;
    if !artifact.complete {
        return Err(TaskError::IncompleteArtifact);
    }
    let state = if let Some(expected) = task.expected {
        verify_file(&mut artifact.file, expected.as_bytes()).map_err(TaskError::Verification)?;
        ArtifactState::Verified
    } else {
        ArtifactState::AwaitingReview
    };
    if cancellation.is_cancelled() {
        return Err(TaskError::Cancelled {
            artifact_created: true,
        });
    }
    Ok(TaskReport {
        artifact: state,
        assistant_message: response.text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelled_tools_refuse_before_touching_files() -> Result<(), Box<dyn std::error::Error>>
    {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let reader = ReadInput(Arc::new(Mutex::new(None)), cancellation.clone());
        let error = reader.call(json!({})).await.err();
        assert!(
            error.is_some_and(|error| error.to_string() == ToolRefusal::ReadCancelled.to_string())
        );
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "agent-cancelled-tool-{}-{stamp}",
            std::process::id()
        ));
        let writer = WriteOutput {
            path: path.clone(),
            allowed: true,
            artifact: Arc::new(Mutex::new(None)),
            cancellation,
        };
        let error = writer
            .call(json!({"content": "must not write"}))
            .await
            .err();
        assert!(
            error.is_some_and(|error| error.to_string() == ToolRefusal::WriteCancelled.to_string())
        );
        assert!(!path.exists());
        Ok(())
    }
}
