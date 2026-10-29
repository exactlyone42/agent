use crate::step::Step;
use std::path::Path;

pub const LINTS: [&str; 10] = [
    "unwrap_used",
    "expect_used",
    "panic",
    "todo",
    "unimplemented",
    "unreachable",
    "disallowed_macros",
    "mod_module_files",
    "excessive_nesting",
    "redundant_else",
];

pub fn gate_steps(root: &Path) -> Vec<Step> {
    let lint_flags: Vec<String> = LINTS
        .iter()
        .flat_map(|lint| ["-D".to_string(), format!("clippy::{lint}")])
        .collect();
    let mut strict = vec![
        "clippy",
        "--workspace",
        "--lib",
        "--bins",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    strict.extend(lint_flags);
    let strict_refs: Vec<&str> = strict.iter().map(String::as_str).collect();
    vec![
        Step::new("cargo", &["fmt", "--all", "--check"]),
        Step::new("cargo", &["machete"]),
        Step::new(
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
        ),
        Step::new("cargo", &strict_refs)
            .with_environment("CLIPPY_CONF_DIR", &root.join("scripts/policy")),
        Step::new(
            "cargo",
            &["test", "--workspace", "--all-features", "--locked"],
        ),
    ]
}
