use crate::drift::{
    finding_schema::finding_schema, reviewers::ReviewCall, run_tool::run_tool, tool_call::ToolCall,
};
use crate::prelude::*;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::Duration;

const CLAUDE: &str = "claude";
const TOOL_TIMEOUT: Duration = Duration::from_secs(900);
const READ_ONLY_TOOLS: &str = "Read,Grep,Glob";
const BUDGET_USD: &str = "1.00";

fn arguments() -> Vec<String> {
    [
        "-p",
        "--output-format",
        "json",
        "--json-schema",
        &finding_schema().to_string(),
        "--tools",
        READ_ONLY_TOOLS,
        "--allowedTools",
        READ_ONLY_TOOLS,
        "--permission-mode",
        "dontAsk",
        "--no-session-persistence",
        "--disable-slash-commands",
        "--setting-sources",
        "",
        "--strict-mcp-config",
        "--max-budget-usd",
        BUDGET_USD,
    ]
    .iter()
    .map(|part| (*part).to_string())
    .collect()
}

fn structured(reply: &Value) -> Result<Value, XtaskError> {
    if reply.get("is_error").and_then(Value::as_bool) == Some(true) {
        return Err(XtaskError::ReviewTool("claude reported an error".into()));
    }
    reply
        .get("structured_output")
        .cloned()
        .ok_or(XtaskError::ReviewTool(
            "claude reply has no structured output".into(),
        ))
}

pub fn claude_reviewer<'a>(root: &'a Path, logs: &'a Path) -> Box<ReviewCall<'a>> {
    Box::new(move |label, prompt| {
        let output = logs.join(format!("{label}.json"));
        let errors = logs.join(format!("{label}.err"));
        run_tool(&ToolCall {
            program: CLAUDE,
            arguments: arguments(),
            input: prompt,
            directory: root,
            output: &output,
            errors: &errors,
            timeout: TOOL_TIMEOUT,
        })?;
        structured(&serde_json::from_slice(&fs::read(&output)?)?)
    })
}
