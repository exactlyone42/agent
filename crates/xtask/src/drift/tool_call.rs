use std::path::Path;
use std::time::Duration;

pub struct ToolCall<'a> {
    pub program: &'a str,
    pub arguments: Vec<String>,
    pub input: &'a str,
    pub directory: &'a Path,
    pub output: &'a Path,
    pub errors: &'a Path,
    pub timeout: Duration,
}
