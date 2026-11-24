use crate::prelude::*;

pub fn help_command(locale: Locale) -> Result<ExitCode, Message> {
    println!("{}", Message::Help.text(locale));
    Ok(ExitCode::SUCCESS)
}
