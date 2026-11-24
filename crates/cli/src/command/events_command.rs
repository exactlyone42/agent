use crate::prelude::*;

pub fn events_command(task_id: &OsStr, after: Option<&OsStr>) -> Result<ExitCode, Message> {
    let task_id = task_id.to_str().ok_or(Message::InvalidTaskId)?;
    let after_seq = match after {
        Some(value) => value
            .to_str()
            .and_then(|value| value.parse::<i64>().ok())
            .ok_or(Message::InvalidEventPosition)?,
        None => 0,
    };
    for event in open_store(StoreAccess::Read)?.events(task_id, after_seq)? {
        println!(
            "{}",
            serde_json::to_string(&event).map_err(|_| Message::OutputFailed)?
        );
    }
    Ok(ExitCode::SUCCESS)
}
