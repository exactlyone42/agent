use crate::prelude::*;

pub fn approvals_command(task_id: &OsStr) -> Result<ExitCode, Message> {
    let task_id = task_id.to_str().ok_or(Message::InvalidTaskId)?;
    let approvals = open_store(StoreAccess::Read)?.approvals(task_id)?;
    println!(
        "{}",
        serde_json::to_string(&approvals).map_err(|_| Message::OutputFailed)?
    );
    Ok(ExitCode::SUCCESS)
}
