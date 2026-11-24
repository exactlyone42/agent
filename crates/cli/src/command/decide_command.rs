use crate::output::Decision;
use crate::prelude::*;
use agent_platform_agent::store::ApprovalStatus;

pub fn decide_command(
    approve: bool,
    approval_id: &OsStr,
    revision: &OsStr,
) -> Result<ExitCode, Message> {
    let approval_id = approval_id.to_str().ok_or(Message::InvalidApprovalId)?;
    let revision = revision
        .to_str()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .ok_or(Message::InvalidRevision)?;
    open_store(StoreAccess::Write)?.decide(approval_id, revision, approve)?;
    let status = if approve {
        ApprovalStatus::Approved
    } else {
        ApprovalStatus::Denied
    };
    println!(
        "{}",
        serde_json::to_string(&Decision {
            approval_id,
            status: status.as_str(),
        })
        .map_err(|_| Message::OutputFailed)?
    );
    Ok(ExitCode::SUCCESS)
}
