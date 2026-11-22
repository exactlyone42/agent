#[cfg(not(any(unix, windows)))]
compile_error!("agent-platform-agent supports only unix and windows");

pub mod approval;
mod file_check;
mod prelude;
mod runtime;
pub mod store;
pub mod task;
mod task_settings;
pub mod task_timeout;
pub mod tool_refusal;
mod tool_refusal_messages;

pub use file_check::{input, single_link, verification};
pub use runtime::{agent_runtime, arbitration, pending_tools};
