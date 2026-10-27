use crate::verification::{VerificationError, verify_file};
use rig_agent::{
    AgentBuilder,
    tool::{Tool, ToolContext, ToolExecutionError},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
pub use tokio_util::sync::CancellationToken;

pub const FILE_LIMIT: u64 = 1024 * 1024;
pub const TASK_TIMEOUT: Duration = Duration::from_secs(120);
pub const MANUAL_TASK_TIMEOUT: Duration = Duration::from_secs(900);

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
    #[error("工具未獲批准或批准已失效；請查詢批准紀錄")]
    ApprovalDenied,
    #[error("任務已停止；若已建立成果，請核對後再接續")]
    Cancelled { artifact_created: bool },
    #[error("任務內容不符合長度限制")]
    InvalidTask,
    #[error("來源必須是大小在限制內的一般檔案")]
    InvalidInput,
    #[error("無法開啟獲准的來源檔案")]
    Input(#[source] std::io::Error),
    #[error("模型執行失敗；可能已有部分成果，請核對後再接續")]
    Harness(#[source] Box<rig_agent::run::PromptError>),
    #[error("任務逾時；可能已有部分成果，請核對後再接續")]
    Timeout,
    #[error("工具狀態異常，不能確認成果")]
    State,
    #[error("模型已結束，但沒有可驗證的成果檔案")]
    MissingArtifact,
    #[error("成果未完整寫入或保存，請核對現有檔案")]
    IncompleteArtifact,
    #[error("成果驗證未通過")]
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

impl Tool for ReadInput {
    const NAME: &'static str = "read_input";
    type Args = ReadArgs;
    type Output = String;
    type Error = ToolExecutionError;

    fn description(&self) -> String {
        "讀取使用者指定且獲准的來源檔案".into()
    }
    fn parameters(&self) -> serde_json::Value {
        json!({"type":"object","properties":{},"additionalProperties":false})
    }
    async fn call(&self, _: &mut ToolContext, _: ReadArgs) -> Result<String, Self::Error> {
        if self.1.is_cancelled() {
            return Err(ToolExecutionError::refused("任務已要求停止，不再讀取"));
        }
        let mut guard = self
            .0
            .lock()
            .map_err(|_| ToolExecutionError::refused("來源狀態異常"))?;
        let file = guard
            .as_mut()
            .ok_or_else(|| ToolExecutionError::refused("使用者未授權讀取來源"))?;
        file.seek(SeekFrom::Start(0))
            .map_err(ToolExecutionError::from_error)?;
        let mut content = String::new();
        file.take(FILE_LIMIT + 1)
            .read_to_string(&mut content)
            .map_err(ToolExecutionError::from_error)?;
        if content.len() as u64 > FILE_LIMIT {
            return Err(ToolExecutionError::refused("來源超出大小限制"));
        }
        Ok(content)
    }
}

impl Tool for WriteOutput {
    const NAME: &'static str = "write_output";
    type Args = WriteArgs;
    type Output = String;
    type Error = ToolExecutionError;

    fn description(&self) -> String {
        "建立使用者指定且獲准的新成果檔案，不覆寫既有資料".into()
    }
    fn parameters(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"content":{"type":"string"}},"required":["content"],"additionalProperties":false})
    }
    async fn call(&self, _: &mut ToolContext, args: WriteArgs) -> Result<String, Self::Error> {
        if self.cancellation.is_cancelled() {
            return Err(ToolExecutionError::refused("任務已要求停止，不再寫入"));
        }
        if !self.allowed {
            return Err(ToolExecutionError::refused("使用者未授權寫入成果"));
        }
        if args.content.len() as u64 > FILE_LIMIT {
            return Err(ToolExecutionError::refused("成果超出大小限制"));
        }
        let mut artifact = self
            .artifact
            .lock()
            .map_err(|_| ToolExecutionError::refused("成果狀態異常"))?;
        if artifact.is_some() {
            return Err(ToolExecutionError::refused("成果已寫入，不重複執行"));
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&self.path)
            .map_err(ToolExecutionError::from_error)?;
        let artifact = artifact.insert(Artifact {
            file,
            complete: false,
        });
        artifact
            .file
            .write_all(args.content.as_bytes())
            .map_err(ToolExecutionError::from_error)?;
        artifact
            .file
            .sync_all()
            .map_err(ToolExecutionError::from_error)?;
        artifact.complete = true;
        Ok("成果已寫入，仍須獨立驗證".into())
    }
}

pub async fn run_file_task(
    model: impl Into<rig_core::DynModel<rig_core::operation::Completion>>,
    task: FileTask,
    cancellation: CancellationToken,
    hook: impl rig_agent::agent::AgentHook + 'static,
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
            crate::input::open_checked(&task.input, FILE_LIMIT).map_err(|error| match error {
                crate::input::InputError::InvalidFile => TaskError::InvalidInput,
                crate::input::InputError::Io(error) => TaskError::Input(error),
            })?;
        Some(file)
    } else {
        None
    };
    let artifact = Arc::new(Mutex::new(None));
    let agent = AgentBuilder::new(model)
        .add_hook(hook)
        .tool(ReadInput(Arc::new(Mutex::new(input)), cancellation.clone()))
        .tool(WriteOutput {
            path: task.output,
            allowed: task.allow_write,
            artifact: artifact.clone(),
            cancellation: cancellation.clone(),
        })
        .build();
    let response = tokio::select! {
        biased;
        _ = cancellation.cancelled() => {
            let artifact_created = artifact.lock().map_err(|_| TaskError::State)?.is_some();
            return Err(TaskError::Cancelled { artifact_created });
        }
        response = tokio::time::timeout(timeout, agent.prompt(task.prompt).max_turns(8).tool_concurrency(1).run()) => {
            response.map_err(|_| TaskError::Timeout)?
                .map_err(|error| TaskError::Harness(Box::new(error)))?
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
        assistant_message: response.output().to_owned(),
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
        let error = reader
            .call(&mut ToolContext::default(), ReadArgs {})
            .await
            .err();
        assert!(error.is_some_and(|error| error.to_string().contains("不再讀取")));
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
            .call(
                &mut ToolContext::default(),
                WriteArgs {
                    content: "must not write".into(),
                },
            )
            .await
            .err();
        assert!(error.is_some_and(|error| error.to_string().contains("不再寫入")));
        assert!(!path.exists());
        Ok(())
    }
}
