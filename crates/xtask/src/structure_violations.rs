const TOP_LEVEL: [&str; 17] = [
    ".cargo",
    ".gitignore",
    ".prettierignore",
    ".repo-policy",
    "AGENTS.md",
    "Cargo.lock",
    "Cargo.toml",
    "DEVELOPMENT.md",
    "PLAN.md",
    "README.md",
    "crates",
    "docs",
    "loops",
    "package-lock.json",
    "package.json",
    "rust-toolchain.toml",
    "scripts",
];

const CRATE_ENTRIES: [&str; 3] = ["Cargo.toml", "src", "tests"];

const DUMP_DIRECTORIES: [&str; 9] = [
    "misc", "common", "helper", "helpers", "util", "utils", "shared", "temp", "tmp",
];

pub fn structure_violations(paths: &[String]) -> Vec<String> {
    let mut violations = Vec::new();
    for path in paths {
        let parts: Vec<&str> = path.split('/').collect();
        let directories = &parts[..parts.len() - 1];
        if !TOP_LEVEL.contains(&parts[0]) {
            violations.push(format!("未登記的頂層項目：{path}"));
            continue;
        }
        if let Some(name) = directories
            .iter()
            .find(|name| DUMP_DIRECTORIES.contains(&name.to_ascii_lowercase().as_str()))
        {
            violations.push(format!("禁用的雜物目錄 {name}：{path}"));
        }
        if parts[0] != "crates" {
            continue;
        }
        if parts.len() == 2 {
            violations.push(format!("crates 底下只能放 crate 資料夾：{path}"));
        } else if !CRATE_ENTRIES.contains(&parts[2]) {
            violations.push(format!("crate 內未登記的項目：{path}"));
        }
    }
    violations
}
