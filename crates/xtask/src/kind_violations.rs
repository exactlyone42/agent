use crate::prelude::*;
use crate::structure_violations::{DOCS_DIRECTORY, REPO_POLICY_DIRECTORY, SQL_DIRECTORY};
use std::path::Path;

const RUNTIME_SUFFIXES: [&str; 6] = ["log", "pid", "tmp", "out", "err", "lock"];
const SCRIPT_SUFFIXES: [&str; 4] = ["py", "sh", "ps1", "bat"];
const SCRIPT_ROOTS: [&str; 2] = [SCRIPTS_DIRECTORY, REPO_POLICY_DIRECTORY];
const DOC_SUFFIXES: [&str; 2] = ["md", "pdf"];

pub fn kind_violations(path: &str) -> Vec<String> {
    let mut violations = Vec::new();
    let top = path.split(GIT_PATH_SEPARATOR).next().unwrap_or(path);
    let suffix = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if RUNTIME_SUFFIXES.contains(&suffix) && path != CARGO_LOCK {
        violations.push(format!("run artifacts must not be tracked by Git: {path}"));
    }
    if SCRIPT_SUFFIXES.contains(&suffix) && !SCRIPT_ROOTS.contains(&top) {
        violations.push(format!(
            "scripts are only allowed in registered script roots: {path}"
        ));
    }
    if suffix == "sql" && top != SQL_DIRECTORY {
        violations.push(format!("sql files are only allowed in sql/: {path}"));
    }
    if top == DOCS_DIRECTORY && !DOC_SUFFIXES.contains(&suffix) {
        violations.push(format!("docs/ only holds .md and .pdf files: {path}"));
    }
    violations
}
