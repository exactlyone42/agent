use crate::prelude::*;
use std::process::ExitCode;

pub fn exit_status(result: Result<Outcome, XtaskError>) -> ExitCode {
    match result {
        Ok(Outcome::Passed) => ExitCode::SUCCESS,
        Ok(Outcome::Failed) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
