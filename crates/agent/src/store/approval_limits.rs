use crate::prelude::*;

pub const APPROVAL_TTL_SECONDS: i64 = 120;
pub const TOOL_CALL_LIMIT: i64 = 64;
pub const APPROVAL_LIST_LIMIT: i64 = 100;
pub(crate) const CALL_ID_LIMIT: usize = 256;
pub(crate) const MAX_ESCAPED_BYTES_PER_CHAR: usize = 6;
pub(crate) const ARGS_ENVELOPE_BYTES: usize = 1024;
pub(crate) const MAX_ARGS_BYTES: usize =
    MAX_ESCAPED_BYTES_PER_CHAR * FILE_LIMIT as usize + ARGS_ENVELOPE_BYTES;
pub(crate) const TOOL_VERSION: i64 = 1;
pub(crate) const EXPECTED_ROWS_CHANGED: usize = 1;
