use agent_platform_xtask::{Task, exit_status, repo_root};
use std::process::ExitCode;

const USAGE_STATUS: u8 = 2;

fn main() -> ExitCode {
    let Some(task) = Task::parse(std::env::args().nth(1).as_deref()) else {
        eprintln!("usage: cargo xtask gate|all|drift|drift-scan");
        return ExitCode::from(USAGE_STATUS);
    };
    exit_status(task.run(&repo_root()))
}
