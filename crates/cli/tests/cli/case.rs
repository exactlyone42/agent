use crate::outcome::Outcome;
use crate::running::Running;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::TempDir;

pub struct Case {
    directory: TempDir,
    deadline: Instant,
}

impl Case {
    pub fn new() -> Self {
        Self::limited(Duration::from_secs(60))
    }

    pub fn limited(limit: Duration) -> Self {
        Self {
            directory: tempfile::Builder::new()
                .prefix("agent-cli-")
                .tempdir()
                .expect("temporary directory"),
            deadline: Instant::now() + limit,
        }
    }

    pub fn root(&self) -> &Path {
        self.directory.path()
    }

    pub fn state(&self) -> PathBuf {
        self.root().join("state")
    }

    pub fn database(&self) -> PathBuf {
        self.state().join("tasks.sqlite3")
    }

    pub fn cli(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_agent-platform"));
        command
            .args(args)
            .current_dir(self.root())
            .env("AGENT_STATE_DIR", self.state())
            .env_remove("AGENT_API_KEY");
        command
    }

    pub fn spawn(&self, command: Command) -> Running {
        Running::start(command, self.deadline)
    }

    pub fn execute(&self, command: Command) -> Outcome {
        self.spawn(command).finish()
    }

    pub fn run(&self, args: &[&str]) -> Outcome {
        self.execute(self.cli(args))
    }

    pub fn write_task(&self, request: &Value) -> PathBuf {
        let path = self.root().join("task.json");
        std::fs::write(&path, request.to_string()).expect("write task file");
        path
    }
}
