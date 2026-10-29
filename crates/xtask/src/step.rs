use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub program: String,
    pub arguments: Vec<String>,
    pub environment: Vec<(String, String)>,
}

impl Step {
    pub fn new(program: &str, arguments: &[&str]) -> Self {
        Self {
            program: program.to_string(),
            arguments: arguments.iter().map(|part| (*part).to_string()).collect(),
            environment: Vec::new(),
        }
    }

    pub fn with_environment(mut self, name: &str, value: &Path) -> Self {
        self.environment
            .push((name.to_string(), value.to_string_lossy().into_owned()));
        self
    }

    pub fn command(&self, root: &Path) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args(&self.arguments)
            .env_remove("CARGO_PKG_NAME")
            .envs(self.environment.iter().cloned())
            .current_dir(root);
        command
    }
}
