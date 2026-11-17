mod fake_root;

use agent_platform_xtask::{Outcome, Plan, XtaskError, check_all};
use fake_root::{fail, fake_root, pass, read_json, write_reports};
use serde_json::json;
use std::fs::{self, OpenOptions};

fn summaries(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(root.join(".runtime/checks"))
        .unwrap()
        .map(|entry| entry.unwrap().path().join("summary.json"))
        .filter(|path| path.exists())
        .collect()
}

#[test]
fn passing_run_writes_summary_and_releases_the_claim() {
    let root = fake_root();
    let outcome = check_all(root.path(), |evidence| {
        write_reports(evidence, 3);
        Plan {
            preflight: vec![pass("pre")],
            main: vec![pass("coverage")],
        }
    })
    .unwrap();
    assert_eq!(outcome, Outcome::Passed);
    assert!(!root.path().join(".runtime/checks/active.json").exists());
    let found = summaries(root.path());
    assert_eq!(found.len(), 1);
    assert_eq!(read_json(&found[0])["passed"], json!(true));
}

#[test]
fn failing_run_is_reported_and_still_releases_the_claim() {
    let root = fake_root();
    let outcome = check_all(root.path(), |evidence| {
        write_reports(evidence, 3);
        Plan {
            preflight: vec![fail("pre")],
            main: Vec::new(),
        }
    })
    .unwrap();
    assert_eq!(outcome, Outcome::Failed);
    assert!(!root.path().join(".runtime/checks/active.json").exists());
}

#[test]
fn held_lock_refuses_a_second_run() {
    let root = fake_root();
    let lock = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.path().join(".git/agent-platform-check.lock"))
        .unwrap();
    lock.try_lock().unwrap();
    let result = check_all(root.path(), |_| Plan {
        preflight: Vec::new(),
        main: Vec::new(),
    });
    assert!(matches!(result, Err(XtaskError::AlreadyRunning)));
}
