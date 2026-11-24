use agent_platform_xtask::{DRIFT_DIRECTORY, Outcome, Reviewers, XtaskError, run_drift};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

const NOW: u64 = 1_800_000_000;
const CONFIRMED_TITLE: &str = "兩個 AI 都認為需要處理";
const DISPUTED_TITLE: &str = "Claude 認為有問題、Codex 不同意";
const REJECTED_TITLE: &str = "證據對不上，已丟棄";

fn git(root: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
}

fn repository() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/a.md"),
        "first line\nsee `crates/gone/x.rs` here\n",
    )
    .unwrap();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "t@example.com"]);
    git(root, &["config", "user.name", "t"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "init"]);
    directory
}

fn update_reply(line: u64, quote: &str) -> Value {
    json!({
        "verdict": "update",
        "reason": "r",
        "evidence": [{"path": "docs/a.md", "line": line, "quote": quote}]
    })
}

type Calls = Rc<RefCell<Vec<String>>>;

fn reviewers(claude_reply: Value, codex_reply: Value, calls: &Calls) -> Reviewers<'static> {
    let claude_calls = Rc::clone(calls);
    let codex_calls = Rc::clone(calls);
    Reviewers {
        claude: Box::new(move |label, _| {
            claude_calls.borrow_mut().push(label.to_string());
            Ok(claude_reply.clone())
        }),
        codex: Box::new(move |label, _| {
            codex_calls.borrow_mut().push(label.to_string());
            Ok(codex_reply.clone())
        }),
    }
}

fn report(root: &Path) -> String {
    let directory = root.join(DRIFT_DIRECTORY);
    let mut names: Vec<_> = fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("report-"))
        .collect();
    names.sort();
    fs::read_to_string(directory.join(names.last().unwrap())).unwrap()
}

#[test]
fn proven_finding_with_codex_agreement_is_confirmed() {
    let directory = repository();
    let calls = Calls::default();
    let mut tools = reviewers(
        update_reply(2, "crates/gone/x.rs"),
        json!({"agree": true, "reason": "same"}),
        &calls,
    );
    let outcome = run_drift(directory.path(), NOW, 8, &mut tools).unwrap();
    assert_eq!(outcome, Outcome::Passed);
    assert_eq!(*calls.borrow(), ["claude-1", "codex-1"]);
    let text = report(directory.path());
    let confirmed = text.split(CONFIRMED_TITLE).nth(1).unwrap();
    assert!(
        confirmed
            .split("\n## ")
            .next()
            .unwrap()
            .contains("docs/a.md")
    );
}

#[test]
fn codex_disagreement_is_reported_separately() {
    let directory = repository();
    let calls = Calls::default();
    let mut tools = reviewers(
        update_reply(2, "crates/gone/x.rs"),
        json!({"agree": false, "reason": "no"}),
        &calls,
    );
    run_drift(directory.path(), NOW, 8, &mut tools).unwrap();
    let text = report(directory.path());
    let disputed = text.split(DISPUTED_TITLE).nth(1).unwrap();
    assert!(
        disputed
            .split("\n## ")
            .next()
            .unwrap()
            .contains("docs/a.md")
    );
    let confirmed = text.split(CONFIRMED_TITLE).nth(1).unwrap();
    assert!(
        !confirmed
            .split("\n## ")
            .next()
            .unwrap()
            .contains("docs/a.md")
    );
}

#[test]
fn unproven_finding_is_dropped_and_codex_is_never_asked() {
    let directory = repository();
    let calls = Calls::default();
    let mut tools = reviewers(
        update_reply(1, "crates/gone/x.rs"),
        json!({"agree": true, "reason": "same"}),
        &calls,
    );
    run_drift(directory.path(), NOW, 8, &mut tools).unwrap();
    assert_eq!(*calls.borrow(), ["claude-1"]);
    let text = report(directory.path());
    let rejected = text.split(REJECTED_TITLE).nth(1).unwrap();
    assert!(
        rejected
            .split("\n## ")
            .next()
            .unwrap()
            .contains("docs/a.md")
    );
}

#[test]
fn reviewed_file_is_skipped_on_the_next_run() {
    let directory = repository();
    let calls = Calls::default();
    let mut tools = reviewers(
        json!({"verdict": "ok", "reason": "fine", "evidence": []}),
        json!({"agree": true, "reason": "same"}),
        &calls,
    );
    run_drift(directory.path(), NOW, 8, &mut tools).unwrap();
    run_drift(directory.path(), NOW + 3600, 8, &mut tools).unwrap();
    assert_eq!(calls.borrow().len(), 1);
}

#[test]
fn tool_failure_fails_the_run_keeps_the_file_unmarked_and_releases_the_lock() {
    let directory = repository();
    let mut broken = Reviewers {
        claude: Box::new(|_, _| Err(XtaskError::ReviewTool("down".into()))),
        codex: Box::new(|_, _| Err(XtaskError::ReviewTool("down".into()))),
    };
    let outcome = run_drift(directory.path(), NOW, 8, &mut broken).unwrap();
    assert_eq!(outcome, Outcome::Failed);
    assert!(
        !directory
            .path()
            .join(DRIFT_DIRECTORY)
            .join("running.lock")
            .exists()
    );
    let calls = Calls::default();
    let mut working = reviewers(
        json!({"verdict": "ok", "reason": "fine", "evidence": []}),
        json!({"agree": true, "reason": "same"}),
        &calls,
    );
    let again = run_drift(directory.path(), NOW + 60, 8, &mut working).unwrap();
    assert_eq!(again, Outcome::Passed);
    assert_eq!(calls.borrow().len(), 1);
}

#[test]
fn malformed_ai_reply_is_a_failed_run_not_a_verdict() {
    let directory = repository();
    let calls = Calls::default();
    let mut tools = reviewers(json!({"verdict": "update"}), json!({}), &calls);
    let outcome = run_drift(directory.path(), NOW, 8, &mut tools).unwrap();
    assert_eq!(outcome, Outcome::Failed);
}

#[test]
fn a_running_lock_stops_a_second_run() {
    let directory = repository();
    let lock = directory.path().join(DRIFT_DIRECTORY).join("running.lock");
    fs::create_dir_all(lock.parent().unwrap()).unwrap();
    fs::write(&lock, "").unwrap();
    let calls = Calls::default();
    let mut tools = reviewers(json!({}), json!({}), &calls);
    let result = run_drift(directory.path(), NOW, 8, &mut tools);
    assert!(matches!(result, Err(XtaskError::DriftAlreadyRunning(_))));
    assert!(calls.borrow().is_empty());
    assert!(lock.exists());
}
