use crate::prelude::*;
use crate::{en_store_text::en_store_text, en_task_text::en_task_text};
use agent_platform_agent::store::{APPROVAL_TTL_SECONDS, EVENT_PAGE_SIZE};

pub fn en_text(message: &Message) -> String {
    match message {
        Message::Help => format!(
            "Usage: {APP_NAME} {COMMAND_RUN} <task.json>\n       {APP_NAME} {COMMAND_STATUS} <task-id>\n       {APP_NAME} {COMMAND_EVENTS} <task-id> [last-seq]\n       {APP_NAME} {COMMAND_APPROVALS} <task-id>\n       {APP_NAME} {COMMAND_APPROVE} <approval-id> <task-revision>\n       {APP_NAME} {COMMAND_DENY} <approval-id> <task-revision>\n{COMMAND_EVENTS} returns at most {EVENT_PAGE_SIZE} items per call; use the seq of the last item as last-seq to read the next page.\nSet approval_mode to manual in the task JSON to confirm each tool action; each approval stays valid for {APPROVAL_TTL_SECONDS} seconds.\nThis is a development entry point that only connects to local model services; reads and writes must be explicitly authorized in the task. It is not an enterprise release."
        ),
        Message::StateDirEmpty => format!("{STATE_DIR_ENV} must not be empty"),
        Message::NoUserDataDir => {
            format!("Could not find the user data directory; set {STATE_DIR_ENV}")
        }
        Message::StateDirNotAbsolute => format!("{STATE_DIR_ENV} must be an absolute path"),
        Message::StateDirCreateFailed => "Could not create the task data directory".into(),
        Message::StateDirMissing => "Could not find the task data directory".into(),
        Message::StateDirNotPlain => {
            "The task data directory must be a regular directory, not a symbolic link".into()
        }
        #[cfg(unix)]
        Message::StateDirTooOpen => format!(
            "The task data directory must not be accessible to other users; set its permissions to {:o}",
            crate::settings::STATE_DIR_MODE
        ),
        Message::DatabaseNotPlain => {
            "The task database must be a regular file, not a symbolic link".into()
        }
        Message::DatabaseHardLink => "The task database must not be a hard link".into(),
        Message::DatabaseUnreadable => "Could not find or read the task database".into(),
        Message::InvalidTaskId => "The task ID is not valid text".into(),
        Message::InvalidApprovalId => "The approval ID is not valid text".into(),
        Message::InvalidRevision => "The task revision must be a positive integer".into(),
        Message::InvalidEventPosition => "The event position must be an integer".into(),
        Message::OutputFailed => "Could not write the result".into(),
        Message::TaskFileOpenFailed => "Could not open the task file".into(),
        Message::TaskFileReadFailed => "Could not read the task file".into(),
        Message::TaskFileTooLarge => format!("The task file is larger than {TASK_FILE_LIMIT_KIB} KiB"),
        Message::TaskFileInvalid => {
            "The task JSON is malformed or contains unknown fields".into()
        }
        Message::EndpointRejected => "Only an explicit local IP model endpoint and a non-empty model name are accepted; the URL must not carry credentials or query parameters".into(),
        Message::ApiKeyInvalid => format!("{API_KEY_ENV} is not valid text"),
        Message::ModelClientFailed => "Could not create the model client".into(),
        Message::InterruptHandlerFailed => {
            "Could not install the stop handler, so the task was not started".into()
        }
        Message::WorkingDirUnavailable => "Could not determine the working directory".into(),
        Message::TaskRecordPoisoned => "The task record state is invalid".into(),
        Message::OutcomeUnknown => {
            "The tool result is unconfirmed; check the records and the artifact before continuing".into()
        }
        Message::StatusMayBeStale => "This is the last saved state and does not prove the process is still running; check the artifact first and do not simply rerun.".into(),
        Message::TaskStarted(task_id) => format!("Task ID: {task_id}"),
        Message::ManualModeHint => format!(
            "Manual approval mode: list this task's approvals with {COMMAND_APPROVALS}, then submit your decision and the task revision with {COMMAND_APPROVE} or {COMMAND_DENY}."
        ),
        Message::TaskCancelled => "The local task was stopped; any artifact already created is kept, so check it before continuing.".into(),
        Message::Store(error) => en_store_text(error).into(),
        Message::Task(error) => en_task_text(error).into(),
        Message::FinishUnconfirmed(error) => format!(
            "{}; the task result is unconfirmed, so do not simply rerun it",
            en_store_text(error)
        ),
    }
}
