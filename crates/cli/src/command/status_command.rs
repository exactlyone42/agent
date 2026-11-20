use crate::prelude::*;
use agent_platform_agent::store::{StoreAccess, TaskState};
use std::{ffi::OsStr, process::ExitCode};

pub fn status_command(locale: Locale, task_id: &OsStr) -> Result<ExitCode, Message> {
    let task_id = task_id.to_str().ok_or(Message::InvalidTaskId)?;
    let record = open_store(StoreAccess::Read)?.status(task_id)?;
    if record.recorded_state == TaskState::Running.as_str() {
        eprintln!("{}", Message::StatusMayBeStale.text(locale));
    }
    println!(
        "{}",
        serde_json::to_string(&record).map_err(|_| Message::OutputFailed)?
    );
    Ok(ExitCode::SUCCESS)
}
