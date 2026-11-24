use crate::prelude::*;

pub(crate) fn tool_failure(program: &str, detail: impl std::fmt::Display) -> XtaskError {
    XtaskError::ReviewTool(format!("{program}: {detail}"))
}
