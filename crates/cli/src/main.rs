mod execute;
mod run;
use run::run;
mod announce_task;
mod cancel_task;
mod cli_error;
mod event_stream;
mod instruct_task;
mod interactive;
#[cfg(target_os = "linux")]
mod linux_cancellation;
mod report_error;
mod task_cancellation;
mod watch_control;

use agent_harness_openai::OpenAi;
use agent_platform_agent::task::{
    ArtifactState, CancellationToken, FileTask, TaskError, run_file_task_with_session,
};
use agent_platform_agent::{
    approval::task_gate,
    single_link::single_link,
    store::{ApprovalMode, StoreAccess, TaskSpec, TaskStore},
    task_timeout::task_timeout,
};
use cli_error::CliError;
use serde::Deserialize;
use std::{
    io::Read,
    path::PathBuf,
    process::ExitCode,
    sync::{Arc, Mutex},
};

fn version() -> String {
    serde_json::json!({
        "schema_version": 1,
        "program_version": env!("CARGO_PKG_VERSION"),
        "error_schema_version": cli_error::ERROR_SCHEMA_VERSION,
        "event_schema_version": 4,
        "database_schema_version": agent_platform_agent::store::SCHEMA_VERSION,
        "capabilities": ["list", "result", "run", "resume", "cancel", "instruct", "status", "events", "approvals", "approve", "deny"]
    })
    .to_string()
}

fn fail(code: &'static str, message: &str) -> CliError {
    CliError::new(code, message)
}

fn open_store(access: StoreAccess) -> Result<TaskStore, CliError> {
    let create = access == StoreAccess::Create;
    let directory = match std::env::var_os("AGENT_STATE_DIR") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        Some(_) => return Err(fail("invalid_state_dir", "AGENT_STATE_DIR 不可為空")),
        None => directories::ProjectDirs::from("", "", "agent-platform")
            .map(|dirs| dirs.data_local_dir().to_path_buf())
            .ok_or_else(|| {
                fail(
                    "state_dir_unavailable",
                    "找不到使用者資料目錄，請設定 AGENT_STATE_DIR",
                )
            })?,
    };
    if !directory.is_absolute() {
        return Err(fail("invalid_state_dir", "AGENT_STATE_DIR 必須是絕對路徑"));
    }
    if create {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&directory)
            .map_err(|_| fail("state_dir_unavailable", "無法建立任務資料目錄"))?;
    }
    let metadata = std::fs::symlink_metadata(&directory)
        .map_err(|_| fail("state_dir_unavailable", "找不到任務資料目錄"))?;
    if !metadata.is_dir() {
        return Err(fail(
            "invalid_state_dir",
            "任務資料目錄必須是一般目錄，不可使用符號連結",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(fail(
                "invalid_state_dir",
                "任務資料目錄不可讓其他使用者存取，請設定權限為 700",
            ));
        }
    }
    let path = directory.join("tasks.sqlite3");
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(fail(
                    "invalid_database_file",
                    "任務資料庫必須是一般檔案，不可使用符號連結",
                ));
            }
            if !single_link(&metadata) {
                return Err(fail("invalid_database_file", "任務資料庫不可使用硬連結"));
            }
        }
        Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(fail("database_unavailable", "找不到或無法讀取任務資料庫")),
    }
    match access {
        StoreAccess::Create => TaskStore::open(&path),
        StoreAccess::Read => TaskStore::read(&path),
        StoreAccess::Write => TaskStore::write(&path),
    }
    .map_err(CliError::from)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    endpoint: url::Url,
    model: String,
    task: FileTask,
    #[serde(default)]
    approval_mode: ApprovalMode,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => code,
        Err(error) => report_error::report_error(error),
    }
}
