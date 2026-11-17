use crate::kind_violations::kind_violations;

pub(crate) const GIT_PATH_SEPARATOR: char = '/';
pub(crate) const CARGO_DIRECTORY: &str = ".cargo";
pub(crate) const REPO_POLICY_DIRECTORY: &str = ".repo-policy";
pub(crate) const CRATES_DIRECTORY: &str = "crates";
pub(crate) const DOCS_DIRECTORY: &str = "docs";
pub(crate) const SCRIPTS_DIRECTORY: &str = "scripts";
pub(crate) const SQL_DIRECTORY: &str = "sql";
pub(crate) const CARGO_TOML: &str = "Cargo.toml";
pub(crate) const CARGO_LOCK: &str = "Cargo.lock";
pub(crate) const TOOLCHAIN_FILE: &str = "rust-toolchain.toml";

const CRATE_FOLDER_PARTS: usize = 2;
const CRATE_ENTRY_INDEX: usize = 2;

const TOP_LEVEL: [&str; 17] = [
    CARGO_DIRECTORY,
    ".gitignore",
    REPO_POLICY_DIRECTORY,
    "AGENTS.md",
    CARGO_LOCK,
    CARGO_TOML,
    "DEVELOPMENT.md",
    "PLAN.md",
    "README.md",
    CRATES_DIRECTORY,
    DOCS_DIRECTORY,
    "loops",
    "package-lock.json",
    "package.json",
    TOOLCHAIN_FILE,
    SCRIPTS_DIRECTORY,
    SQL_DIRECTORY,
];

const CRATE_ENTRIES: [&str; 3] = [CARGO_TOML, "src", "tests"];

const DUMP_DIRECTORIES: [&str; 9] = [
    "misc", "common", "helper", "helpers", "util", "utils", "shared", "temp", "tmp",
];

pub fn structure_violations(paths: &[String]) -> Vec<String> {
    let mut violations = Vec::new();
    for path in paths {
        let parts: Vec<&str> = path.split(GIT_PATH_SEPARATOR).collect();
        let directories = &parts[..parts.len() - 1];
        if !TOP_LEVEL.contains(&parts[0]) {
            violations.push(format!("unregistered top-level item: {path}"));
            continue;
        }
        if let Some(name) = directories
            .iter()
            .find(|name| DUMP_DIRECTORIES.contains(&name.to_ascii_lowercase().as_str()))
        {
            violations.push(format!("forbidden catch-all directory {name}: {path}"));
        }
        violations.extend(kind_violations(path));
        if parts[0] != CRATES_DIRECTORY {
            continue;
        }
        if parts.len() == CRATE_FOLDER_PARTS {
            violations.push(format!("crates/ only holds crate directories: {path}"));
        } else if !CRATE_ENTRIES.contains(&parts[CRATE_ENTRY_INDEX]) {
            violations.push(format!("unregistered item inside a crate: {path}"));
        }
    }
    violations
}
