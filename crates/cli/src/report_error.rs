use crate::cli_error::CliError;
use std::{io::Write, process::ExitCode};

pub fn report_error(error: CliError) -> ExitCode {
    match writeln!(std::io::stderr(), "{}", error.line()) {
        Ok(()) => ExitCode::FAILURE,
        Err(_) => ExitCode::from(74),
    }
}
