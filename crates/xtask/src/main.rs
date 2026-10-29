use agent_platform_xtask::{Task, exit_status, repo_root};
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(task) = Task::parse(std::env::args().nth(1).as_deref()) else {
        eprintln!("用法：cargo xtask gate|all");
        return ExitCode::from(2);
    };
    ExitCode::from(exit_status(task.run(&repo_root())))
}
