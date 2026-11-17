use std::time::Duration;

pub const STATE_DIR_ENV: &str = "AGENT_STATE_DIR";
pub const API_KEY_ENV: &str = "AGENT_API_KEY";
pub const LANGUAGE_ENV: &str = "AGENT_LANG";
pub const LANGUAGE_ENV_PRIORITY: [&str; 4] = [LANGUAGE_ENV, "LC_ALL", "LC_MESSAGES", "LANG"];
pub const TRADITIONAL_CHINESE_TAGS: [&str; 4] = ["zh-tw", "zh-hant", "zh-hk", "zh-mo"];
pub const APP_NAME: &str = "agent-platform";
pub const DATABASE_FILE: &str = "tasks.sqlite3";
pub const COMMAND_HELP: &str = "--help";
pub const COMMAND_STATUS: &str = "status";
pub const COMMAND_APPROVALS: &str = "approvals";
pub const COMMAND_APPROVE: &str = "approve";
pub const COMMAND_DENY: &str = "deny";
pub const COMMAND_EVENTS: &str = "events";
pub const COMMAND_RUN: &str = "run";
pub const SCHEME_HTTP: &str = "http";
pub const SCHEME_HTTPS: &str = "https";
pub const BYTES_PER_KIB: u64 = 1024;
pub const TASK_FILE_LIMIT_KIB: u64 = 64;
pub const TASK_FILE_LIMIT: u64 = TASK_FILE_LIMIT_KIB * BYTES_PER_KIB;
pub const MODEL_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub const INTERRUPTED_EXIT_CODE: u8 = 130;
pub const REVIEW_NEEDED_EXIT_CODE: u8 = 2;
#[cfg(unix)]
pub const STATE_DIR_MODE: u32 = 0o700;
#[cfg(unix)]
pub const OTHER_ACCESS_MASK: u32 = 0o077;
