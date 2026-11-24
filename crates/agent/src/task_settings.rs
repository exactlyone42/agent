use crate::prelude::*;

pub const FILE_LIMIT: u64 = 1024 * 1024;
pub const TASK_TIMEOUT: Duration = Duration::from_secs(120);
pub const MANUAL_TASK_TIMEOUT: Duration = Duration::from_secs(900);
pub const READ_INPUT_TOOL: &str = "read_input";
pub const WRITE_OUTPUT_TOOL: &str = "write_output";
pub const READ_INPUT_DESCRIPTION: &str = "Read the source file that the user named and authorized";
pub const WRITE_OUTPUT_DESCRIPTION: &str = "Create the new artifact file that the user named and authorized; never overwrite existing data";
pub const WRITE_OUTPUT_DONE: &str =
    "the artifact was written and still needs independent verification";
pub(crate) const TASK_MAX_TURNS: usize = 8;
