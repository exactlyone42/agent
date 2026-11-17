mod approvals_command;
mod decide_command;
mod en_store_text;
mod en_task_text;
mod en_text;
mod events_command;
mod help_command;
mod locale;
mod message;
mod open_store;
mod output;
mod prelude;
mod request;
mod run_command;
mod settings;
mod status_command;
mod zh_tw_store_text;
mod zh_tw_task_text;
mod zh_tw_text;

use approvals_command::approvals_command;
use decide_command::decide_command;
use events_command::events_command;
use help_command::help_command;
use locale::Locale;
use message::Message;
use run_command::run_command;
use settings::{
    COMMAND_APPROVALS, COMMAND_APPROVE, COMMAND_DENY, COMMAND_EVENTS, COMMAND_HELP, COMMAND_RUN,
    COMMAND_STATUS,
};
use status_command::status_command;
use std::{path::Path, process::ExitCode};

async fn run(locale: Locale) -> Result<ExitCode, Message> {
    let mut arguments = std::env::args_os();
    arguments.next();
    let args: Vec<_> = arguments.collect();
    let Some((command, rest)) = args.split_first() else {
        return Err(Message::Help);
    };
    match (command.to_str(), rest) {
        (Some(COMMAND_HELP), []) => help_command(locale),
        (Some(COMMAND_STATUS), [task_id]) => status_command(locale, task_id),
        (Some(COMMAND_APPROVALS), [task_id]) => approvals_command(task_id),
        (Some(COMMAND_APPROVE), [approval_id, revision]) => {
            decide_command(true, approval_id, revision)
        }
        (Some(COMMAND_DENY), [approval_id, revision]) => {
            decide_command(false, approval_id, revision)
        }
        (Some(COMMAND_EVENTS), [task_id]) => events_command(task_id, None),
        (Some(COMMAND_EVENTS), [task_id, after]) => events_command(task_id, Some(after)),
        (Some(COMMAND_RUN), [path]) => run_command(locale, Path::new(path)).await,
        _ => Err(Message::Help),
    }
}

pub async fn execute() -> ExitCode {
    let locale = Locale::from_env();
    match run(locale).await {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{}", message.text(locale));
            ExitCode::FAILURE
        }
    }
}
