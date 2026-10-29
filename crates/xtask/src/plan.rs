use crate::strings::strings;
use crate::text::text;
use std::path::Path;

pub type NamedCommand = (String, Vec<String>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub preflight: Vec<NamedCommand>,
    pub main: Vec<NamedCommand>,
}

impl Plan {
    fn entry(name: &str, parts: &[&str]) -> NamedCommand {
        (name.to_string(), strings(parts))
    }

    pub fn real(executable: &str, evidence: &Path, windows: bool) -> Self {
        let report = text(&evidence.join("coverage-report.json"));
        let mut mutation = strings(&[
            "cargo",
            "mutants",
            "--in-place",
            "--workspace",
            "--test-workspace",
            "true",
            "--all-features",
            "--timeout",
            "90",
            "--cargo-arg=--locked",
            "--exclude-re",
            r"crates/xtask/src/task\.rs:\d+:\d+: replace Task::(run|all) -> ",
        ]);
        if !windows {
            mutation.extend(strings(&[
                "--exclude-re",
                r"crates/agent/src/(input\.rs:(30|61|69|82)|single_link\.rs:11):",
            ]));
        }
        mutation.extend(strings(&["--output", &text(&evidence.join("mutation"))]));
        let mut coverage = strings(&[
            "cargo",
            "llvm-cov",
            "--workspace",
            "--all-features",
            "--locked",
            "--json",
            "--output-path",
        ]);
        coverage.push(report);
        Self {
            preflight: vec![
                Self::entry("gate", &[executable, "gate"]),
                Self::entry("versions-rust", &["rustc", "--version"]),
                Self::entry("versions-coverage", &["cargo", "llvm-cov", "--version"]),
                Self::entry("versions-mutation", &["cargo", "mutants", "--version"]),
                Self::entry("versions-secrets", &["gitleaks", "version"]),
                Self::entry("versions-dependencies", &["cargo", "machete", "--version"]),
            ],
            main: vec![
                ("coverage".to_string(), coverage),
                ("mutation".to_string(), mutation),
            ],
        }
    }
}
