use crate::{cli_error::CliError, open_store};
use agent_platform_agent::store::StoreAccess;
use std::process::ExitCode;

pub fn instruct_task(
    task_id: &str,
    client_message_id: &str,
    content: &str,
) -> Result<ExitCode, CliError> {
    let receipt =
        open_store(StoreAccess::Write)?.receive_instruction(task_id, client_message_id, content)?;
    println!(
        "{}",
        serde_json::json!({"schema_version":1,"task_id":task_id,"client_message_id":client_message_id,"seq":receipt.seq,"status":"received","duplicate":receipt.duplicate,"applied":false})
    );
    Ok(ExitCode::SUCCESS)
}
