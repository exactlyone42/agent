use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn fake_root() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    for (name, body) in [
        ("Cargo.toml", "a"),
        ("Cargo.lock", "b"),
        ("rust-toolchain.toml", "c"),
        (".gitignore", ".runtime/\n"),
        ("crates/a/lib.rs", "x"),
        ("scripts/s.sh", "y"),
        (".cargo/config.toml", "z"),
    ] {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
    directory
}

pub fn write_reports(evidence: &Path, caught: u64) {
    let outcomes = evidence.join("mutation/mutants.out");
    fs::create_dir_all(&outcomes).unwrap();
    let coverage = json!({"data": [{"totals": {"lines": {"count": 2, "covered": 1}}}]});
    let mutation = json!({
        "total_mutants": caught,
        "caught": caught,
        "unviable": 0,
        "missed": 0,
        "timeout": 0,
    });
    fs::write(evidence.join("coverage-report.json"), coverage.to_string()).unwrap();
    fs::write(outcomes.join("outcomes.json"), mutation.to_string()).unwrap();
}

pub fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

pub fn pass(name: &str) -> (String, Vec<String>) {
    (
        name.to_string(),
        ["rustc", "--print", "sysroot"]
            .iter()
            .map(|part| (*part).to_string())
            .collect(),
    )
}

pub fn fail(name: &str) -> (String, Vec<String>) {
    (
        name.to_string(),
        ["rustc", "--print", "no-such-print-value"]
            .iter()
            .map(|part| (*part).to_string())
            .collect(),
    )
}
