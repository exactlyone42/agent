use agent_platform_xtask::{
    Candidate, CandidateKind, FileGroup, Finding, ReviewState, RunLock, Verdict, XtaskError,
    group_candidates, select_files,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const DAY: u64 = 86_400;

fn files() -> BTreeMap<String, Vec<u8>> {
    let mut map = BTreeMap::new();
    map.insert(
        "docs/a.md".to_string(),
        b"first line\nsee crates/gone/x.rs here\n".to_vec(),
    );
    map
}

fn finding(verdict: &str, line: u64, quote: &str) -> Finding {
    let value: Value = json!({
        "verdict": verdict,
        "reason": "r",
        "evidence": [{"path": "docs/a.md", "line": line, "quote": quote}]
    });
    Finding::parse(&value).unwrap()
}

#[test]
fn evidence_that_matches_the_real_line_holds() {
    assert!(finding("update", 2, "crates/gone/x.rs").holds(&files()));
}

#[test]
fn evidence_with_wrong_quote_line_or_path_is_rejected() {
    assert!(!finding("update", 2, "crates/other/y.rs").holds(&files()));
    assert!(!finding("update", 1, "crates/gone/x.rs").holds(&files()));
    assert!(!finding("update", 9, "crates/gone/x.rs").holds(&files()));
    assert!(!finding("update", 0, "crates/gone/x.rs").holds(&files()));
    assert!(!finding("delete", 2, "ab").holds(&files()));
    let value = json!({
        "verdict": "delete",
        "reason": "r",
        "evidence": [{"path": "docs/none.md", "line": 1, "quote": "first line"}]
    });
    assert!(!Finding::parse(&value).unwrap().holds(&files()));
}

#[test]
fn update_without_evidence_is_rejected_but_ok_needs_none() {
    let none = json!({"verdict": "update", "reason": "r", "evidence": []});
    assert!(!Finding::parse(&none).unwrap().holds(&files()));
    let fine = json!({"verdict": "ok", "reason": "r", "evidence": []});
    assert!(Finding::parse(&fine).unwrap().holds(&files()));
}

#[test]
fn one_bad_proof_rejects_the_whole_finding() {
    let value = json!({
        "verdict": "update",
        "reason": "r",
        "evidence": [
            {"path": "docs/a.md", "line": 2, "quote": "crates/gone/x.rs"},
            {"path": "docs/a.md", "line": 1, "quote": "not on this line"}
        ]
    });
    assert!(!Finding::parse(&value).unwrap().holds(&files()));
}

#[test]
fn malformed_replies_do_not_parse() {
    assert!(Finding::parse(&json!({"verdict": "maybe", "reason": "r", "evidence": []})).is_none());
    assert!(Finding::parse(&json!({"verdict": "ok", "reason": "r"})).is_none());
    assert!(Verdict::parse("delete").is_some());
    assert!(Verdict::parse("remove").is_none());
}

fn candidate(path: &str, kind: CandidateKind) -> Candidate {
    Candidate {
        path: path.to_string(),
        line: 1,
        kind,
        subject: "s".to_string(),
    }
}

#[test]
fn diagram_candidates_never_reach_the_ai() {
    let all = vec![
        candidate("docs/a.md", CandidateKind::DeadReference),
        candidate("docs/diagrams/x.pdf", CandidateKind::DiagramBehindCode),
    ];
    let groups = group_candidates(&all, &files());
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].path, "docs/a.md");
}

fn group(path: &str, hash: &str) -> FileGroup {
    FileGroup {
        path: path.to_string(),
        candidates: Vec::new(),
        hash: hash.to_string(),
    }
}

#[test]
fn recently_reviewed_unchanged_file_is_skipped() {
    let mut state = ReviewState::default();
    state.mark("a.md", "h1", 1_000);
    let picked = select_files(vec![group("a.md", "h1")], &state, 1_000 + 10 * DAY, 8);
    assert!(picked.chosen.is_empty());
    assert_eq!(picked.fresh, 1);
}

#[test]
fn changed_content_or_old_review_is_checked_again() {
    let mut state = ReviewState::default();
    state.mark("a.md", "h1", 1_000);
    let changed = select_files(vec![group("a.md", "h2")], &state, 1_000 + DAY, 8);
    assert_eq!(changed.chosen.len(), 1);
    let old = select_files(vec![group("a.md", "h1")], &state, 1_000 + 29 * DAY, 8);
    assert_eq!(old.chosen.len(), 1);
}

#[test]
fn never_reviewed_files_go_first_and_the_limit_defers_the_rest() {
    let mut state = ReviewState::default();
    state.mark("old.md", "h", 1_000);
    let groups = vec![group("old.md", "x"), group("b.md", "h"), group("a.md", "h")];
    let picked = select_files(groups, &state, 1_000 + 40 * DAY, 2);
    let names: Vec<&str> = picked.chosen.iter().map(|g| g.path.as_str()).collect();
    assert_eq!(names, ["a.md", "b.md"]);
    assert_eq!(picked.deferred, 1);
}

#[test]
fn state_survives_a_save_and_load_and_a_broken_file_is_an_error() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("state.json");
    assert_eq!(ReviewState::load(&path).unwrap(), ReviewState::default());
    let mut state = ReviewState::default();
    state.mark("a.md", "h", 5);
    state.save(&path).unwrap();
    assert_eq!(ReviewState::load(&path).unwrap(), state);
    std::fs::write(&path, "[1, 2]").unwrap();
    assert!(ReviewState::load(&path).is_err());
}

#[test]
fn second_lock_is_refused_until_the_first_is_dropped() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("a").join("running.lock");
    let first = RunLock::acquire(&marker).unwrap();
    assert!(matches!(
        RunLock::acquire(&marker),
        Err(XtaskError::DriftAlreadyRunning(_))
    ));
    drop(first);
    assert!(!marker.exists());
    assert!(RunLock::acquire(&marker).is_ok());
}
