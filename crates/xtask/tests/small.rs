use agent_platform_xtask::{
    Step, Task, XtaskError, all_passed, exit_status, now_seconds, run_steps, source_hashes,
};
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

#[test]
fn clock_is_after_2023() {
    assert!(now_seconds() > 1.7e9);
}

#[test]
fn all_passed_needs_every_code_zero() {
    let mut results: BTreeMap<&str, i32> = BTreeMap::new();
    assert!(all_passed(&results));
    results.insert("a", 0);
    assert!(all_passed(&results));
    results.insert("b", 3);
    assert!(!all_passed(&results));
}

#[test]
fn exit_status_maps_results() {
    assert_eq!(exit_status(Ok(0)), 0);
    assert_eq!(exit_status(Ok(5)), 1);
    assert_eq!(exit_status(Err(XtaskError::EmptySnapshot)), 1);
}

#[test]
fn task_names_are_parsed() {
    assert_eq!(Task::parse(Some("gate")), Some(Task::Gate));
    assert_eq!(Task::parse(Some("all")), Some(Task::All));
    assert_eq!(Task::parse(Some("other")), None);
    assert_eq!(Task::parse(None), None);
}

#[test]
fn binary_rejects_missing_and_unknown_task() {
    let program = env!("CARGO_BIN_EXE_xtask");
    assert_eq!(Command::new(program).status().unwrap().code(), Some(2));
    let unknown = Command::new(program).arg("other").status().unwrap();
    assert_eq!(unknown.code(), Some(2));
}

#[test]
fn run_steps_stops_at_first_failure() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let good = Step::new("rustc", &["--print", "sysroot"]);
    let bad = Step::new("rustc", &["--print", "no-such-print-value"]);
    let mark = Step::new("git", &["init", "-q", "marker"]);
    let expected = Command::new("rustc")
        .args(["--print", "no-such-print-value"])
        .output()
        .unwrap()
        .status
        .code();
    let code = run_steps(root, &[good.clone(), bad, mark.clone()]).unwrap();
    assert_eq!(Some(code), expected);
    assert!(!root.join("marker").exists());
    assert_eq!(run_steps(root, &[good, mark]).unwrap(), 0);
    assert!(root.join("marker").exists());
}

#[test]
fn source_hashes_cover_nested_files_with_known_digests() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"] {
        fs::write(root.join(name), "hello").unwrap();
    }
    fs::create_dir_all(root.join("crates/a/b")).unwrap();
    fs::write(root.join("crates/a/b/deep.rs"), "hello").unwrap();
    let hashes = source_hashes(root).unwrap();
    let digest = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
    assert_eq!(hashes.len(), 4);
    assert!(hashes.values().all(|value| value == digest));
    assert!(hashes.contains_key("crates/a/b/deep.rs"));
    assert!(hashes.contains_key("Cargo.lock"));
}

#[test]
fn source_hashes_reject_unreadable_directory() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "scripts"] {
        fs::write(root.join(name), "hello").unwrap();
    }
    assert!(source_hashes(root).is_err());
}
