use crate::drift::{
    opinion_schema::opinion_schema, reviewers::ReviewCall, run_tool::run_tool, tool_call::ToolCall,
};
use crate::prelude::*;
use std::fs;
use std::path::Path;
use std::time::Duration;

const CODEX: &str = "codex";
const TOOL_TIMEOUT: Duration = Duration::from_secs(900);

fn arguments(schema: &Path, last: &Path, root: &Path) -> Vec<String> {
    [
        "exec",
        "--sandbox",
        "read-only",
        "--skip-git-repo-check",
        "--ephemeral",
        "--output-schema",
        &text(schema),
        "-o",
        &text(last),
        "-C",
        &text(root),
        "-",
    ]
    .iter()
    .map(|part| (*part).to_string())
    .collect()
}

pub fn codex_reviewer<'a>(root: &'a Path, logs: &'a Path) -> Box<ReviewCall<'a>> {
    Box::new(move |label, prompt| {
        let schema = logs.join(format!("{label}.schema.json"));
        let last = logs.join(format!("{label}.json"));
        let output = logs.join(format!("{label}.out"));
        let errors = logs.join(format!("{label}.err"));
        fs::write(&schema, opinion_schema().to_string())?;
        run_tool(&ToolCall {
            program: CODEX,
            arguments: arguments(&schema, &last, root),
            input: prompt,
            directory: root,
            output: &output,
            errors: &errors,
            timeout: TOOL_TIMEOUT,
        })?;
        Ok(serde_json::from_slice(&fs::read(&last)?)?)
    })
}
